//! [`Replay`]: plays a recorded event list into a [`Session`] on a clock, for demos, tests and
//! offline previews. Pure: you pass the time in, so it works with any clock (egui's included).

use crate::event::AgentEvent;
use crate::session::Session;

/// Plays recorded events back at a natural pace: deltas quickly, structural events with a short
/// pause. While the session has an approval waiting, playback holds, so the user (or your code)
/// decides before the run continues.
///
/// ```
/// use glowkit_stream::{replay::Replay, AgentEvent, Session};
/// let mut r = Replay::new(vec![AgentEvent::text("t", "a"), AgentEvent::text("t", "b")]);
/// let mut s = Session::new();
/// r.advance(0.0, &mut s);
/// r.advance(10.0, &mut s);
/// assert!(r.is_done());
/// assert_eq!(s.messages[0].text(), "ab");
/// ```
#[derive(Clone, Debug)]
pub struct Replay {
    events: Vec<AgentEvent>,
    cursor: usize,
    next_at: Option<f64>,
    /// Playback speed multiplier (1.0 = natural).
    pub speed: f64,
}

impl Replay {
    /// Replay `events`.
    pub fn new(events: Vec<AgentEvent>) -> Self {
        Self { events, cursor: 0, next_at: None, speed: 1.0 }
    }

    /// All events were applied.
    pub fn is_done(&self) -> bool {
        self.cursor >= self.events.len()
    }

    /// Playback is holding for an approval.
    pub fn is_waiting(&self, session: &Session) -> bool {
        !self.is_done() && session.pending_approvals().next().is_some()
    }

    /// Fraction of events applied.
    pub fn progress(&self) -> f32 {
        if self.events.is_empty() {
            1.0
        } else {
            self.cursor as f32 / self.events.len() as f32
        }
    }

    /// The events not yet applied.
    pub fn remaining(&self) -> &[AgentEvent] {
        &self.events[self.cursor.min(self.events.len())..]
    }

    /// Drop the rest of the recording.
    pub fn stop(&mut self) {
        self.cursor = self.events.len();
    }

    /// Apply every event due at time `now` (seconds). Returns how many were applied.
    pub fn advance(&mut self, now: f64, session: &mut Session) -> usize {
        let mut n = 0;
        let mut at = *self.next_at.get_or_insert(now);
        while !self.is_done() && now >= at {
            if session.pending_approvals().next().is_some() {
                // hold; resume with a fresh pause once answered
                self.next_at = Some(now + 0.5 / self.speed.max(0.01));
                return n;
            }
            let e = self.events[self.cursor].clone();
            self.cursor += 1;
            at += delay(&e) / self.speed.max(0.01);
            session.apply(e);
            n += 1;
        }
        // never let a long stall make playback burst
        self.next_at = Some(at.max(now - 0.05));
        n
    }
}

/// Seconds to wait after `e`.
fn delay(e: &AgentEvent) -> f64 {
    match e {
        AgentEvent::TextDelta { .. } => 0.045,
        AgentEvent::ReasoningDelta { .. } => 0.04,
        AgentEvent::ToolArgsDelta { .. } => 0.06,
        AgentEvent::Usage(_) | AgentEvent::Custom { .. } | AgentEvent::TextStart { .. } => 0.0,
        AgentEvent::TextEnd { .. } | AgentEvent::ReasoningEnd { .. } => 0.3,
        AgentEvent::ToolInputReady { .. } | AgentEvent::ToolRunning { .. } => 0.6,
        AgentEvent::ToolResult { .. } => 0.5,
        AgentEvent::RunFinish { .. } => 0.8,
        _ => 0.35,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_for_approvals() {
        let mut r = Replay::new(vec![
            AgentEvent::ApprovalRequest {
                id: "a".into(),
                tool_call_id: None,
                title: None,
                description: None,
                options: vec![],
            },
            AgentEvent::text("t", "after"),
        ]);
        let mut s = Session::new();
        r.advance(0.0, &mut s);
        assert!(r.is_waiting(&s));
        r.advance(100.0, &mut s);
        assert_eq!(s.messages[0].text(), "");
        s.apply(AgentEvent::decision("a", true));
        r.advance(100.0, &mut s);
        r.advance(101.0, &mut s);
        assert!(r.is_done());
        assert_eq!(s.messages[0].text(), "after");
    }

    #[test]
    fn paces_events() {
        let mut r = Replay::new((0..10).map(|_| AgentEvent::text("t", "x")).collect());
        let mut s = Session::new();
        assert_eq!(r.advance(0.0, &mut s), 1);
        assert_eq!(r.advance(0.1, &mut s), 2);
        assert!(r.progress() > 0.2 && r.progress() < 0.4);
    }
}

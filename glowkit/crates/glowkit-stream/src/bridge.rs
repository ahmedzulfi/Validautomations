//! The thread-safe bridge between a background agent runtime and the UI thread.

use crate::event::AgentEvent;
use crate::session::Session;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;

type Notify = Arc<dyn Fn() + Send + Sync>;

/// The UI went away (the [`EventReceiver`] was dropped).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disconnected;

impl std::fmt::Display for Disconnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the UI side of the event bridge was dropped")
    }
}

impl std::error::Error for Disconnected {}

/// Create a connected sender / receiver pair. Built on `std::sync::mpsc`: no async runtime is
/// assumed, so it works from tokio, async-std, smol or a plain thread.
///
/// ```
/// use glowkit_stream::{bridge, AgentEvent, Session};
/// let (tx, mut rx) = bridge::channel();
/// std::thread::spawn(move || {
///     tx.send(AgentEvent::text("t", "hello from a worker")).ok();
/// })
/// .join()
/// .unwrap();
/// let mut session = Session::new();
/// rx.drain_into(&mut session);
/// assert_eq!(session.messages[0].text(), "hello from a worker");
/// ```
pub fn channel() -> (EventSender, EventReceiver) {
    let (tx, rx) = mpsc::channel();
    (EventSender { tx, notify: None }, EventReceiver { rx, closed: false })
}

/// The producer half: clone it into every task that emits events.
#[derive(Clone)]
pub struct EventSender {
    tx: Sender<AgentEvent>,
    notify: Option<Notify>,
}

impl std::fmt::Debug for EventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSender").field("notify", &self.notify.is_some()).finish()
    }
}

impl EventSender {
    /// Call `notify` after every send (wake your UI loop).
    pub fn with_notify(mut self, notify: impl Fn() + Send + Sync + 'static) -> Self {
        self.notify = Some(Arc::new(notify));
        self
    }

    /// Ask egui to repaint after every send, so the UI updates without polling.
    #[cfg(feature = "egui")]
    pub fn with_repaint(self, ctx: &egui::Context) -> Self {
        let ctx = ctx.clone();
        self.with_notify(move || ctx.request_repaint())
    }

    /// Send one event.
    pub fn send(&self, event: AgentEvent) -> Result<(), Disconnected> {
        self.tx.send(event).map_err(|_| Disconnected)?;
        if let Some(n) = &self.notify {
            n();
        }
        Ok(())
    }

    /// Send several events, notifying once.
    pub fn send_all(&self, events: impl IntoIterator<Item = AgentEvent>) -> Result<(), Disconnected> {
        let mut any = false;
        for e in events {
            self.tx.send(e).map_err(|_| Disconnected)?;
            any = true;
        }
        if any {
            if let Some(n) = &self.notify {
                n();
            }
        }
        Ok(())
    }
}

/// The UI half: drain it into a [`Session`] once per frame.
#[derive(Debug)]
pub struct EventReceiver {
    rx: Receiver<AgentEvent>,
    closed: bool,
}

impl EventReceiver {
    /// Apply every queued event to `session`; returns how many were applied. Never blocks.
    pub fn drain_into(&mut self, session: &mut Session) -> usize {
        let mut n = 0;
        for e in self.drain() {
            session.apply(e);
            n += 1;
        }
        n
    }

    /// Take every queued event without applying them.
    pub fn drain(&mut self) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(e) => out.push(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.closed = true;
                    break;
                }
            }
        }
        out
    }

    /// Whether every sender was dropped and everything was drained (the stream is over).
    /// Updated by [`Self::drain`] / [`Self::drain_into`].
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn notify_runs_per_send_and_drain_applies() {
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let (tx, mut rx) = channel();
        let tx = tx.with_notify(move || {
            h.fetch_add(1, Ordering::SeqCst);
        });
        let workers: Vec<_> = (0..4)
            .map(|i| {
                let tx = tx.clone();
                std::thread::spawn(move || tx.send(AgentEvent::text(format!("t{i}"), "x")).unwrap())
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        tx.send_all([AgentEvent::text("t0", "y"), AgentEvent::text("t0", "z")]).unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 5);
        let mut s = Session::new();
        assert_eq!(rx.drain_into(&mut s), 6);
        assert_eq!(s.messages[0].parts.len(), 4);
        assert!(!rx.is_closed());
        drop(tx);
        assert_eq!(rx.drain_into(&mut s), 0);
        assert!(rx.is_closed());
    }

    #[test]
    fn send_after_ui_dropped_errors() {
        let (tx, rx) = channel();
        drop(rx);
        assert_eq!(tx.send(AgentEvent::error("x")), Err(Disconnected));
    }
}

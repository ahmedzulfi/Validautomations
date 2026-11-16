use crate::replay::Replay;
use crate::{fixtures, AgentEvent, AgentThread, RunStatus, Session, StopReason};
use egui::{vec2, Align2, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Proto {
    AgUi,
    Vercel,
    Acp,
}

impl Proto {
    const ALL: [Proto; 3] = [Proto::AgUi, Proto::Vercel, Proto::Acp];

    fn label(self) -> &'static str {
        match self {
            Proto::AgUi => "AG-UI",
            Proto::Vercel => "Vercel AI SDK",
            Proto::Acp => "ACP-style",
        }
    }

    fn events(self) -> Vec<AgentEvent> {
        match self {
            Proto::AgUi => fixtures::ag_ui_events(),
            Proto::Vercel => fixtures::vercel_events(),
            Proto::Acp => fixtures::acp_events(),
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Proto::Acp => fixtures::CODING_PROMPT,
            _ => fixtures::TRAVEL_PROMPT,
        }
    }
}

#[derive(Clone)]
struct Live {
    proto: Proto,
    session: Session,
    replay: Replay,
    waiting_since: Option<f64>,
    done_at: Option<f64>,
}

impl Live {
    fn new(proto: Proto) -> Self {
        let mut session = Session::new();
        session.push_user(proto.prompt());
        let mut replay = Replay::new(proto.events());
        replay.speed = 1.6;
        Self { proto, session, replay, waiting_since: None, done_at: None }
    }
}

/// Seconds an unanswered approval waits before the demo approves it by itself.
const AUTO_APPROVE: f64 = 6.0;

fn live_agent_stream(ui: &mut Ui) {
    let t = Theme::of(ui);
    let now = ui.input(|i| i.time);
    let id = ui.scope_id().with("live-agent-stream");
    let mut st: Live = glowkit_core::state(ui, id, || Live::new(Proto::AgUi));

    // advance the recording through the real reducer
    st.replay.advance(now, &mut st.session);
    let waiting = st.replay.is_waiting(&st.session);
    match (waiting, st.waiting_since) {
        (true, None) => st.waiting_since = Some(now),
        (true, Some(since)) if now - since > AUTO_APPROVE => {
            let ids: Vec<String> = st.session.pending_approvals().map(|a| a.id.clone()).collect();
            for a in ids {
                st.session.apply(AgentEvent::decision(a, true));
            }
            st.waiting_since = None;
        }
        (false, _) => st.waiting_since = None,
        _ => {}
    }
    if st.replay.is_done() && st.done_at.is_none() {
        st.done_at = Some(now);
    }
    if st.done_at.is_some_and(|d| now - d > 6.0) {
        st = Live::new(st.proto);
    }

    ui.vertical_centered(|ui| {
        ui.set_max_width(760.0_f32.min(ui.available_width()));
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            // protocol switcher + status
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for p in Proto::ALL {
                    if pill(ui, p.label(), p == st.proto).clicked() && p != st.proto {
                        st = Live::new(p);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill(ui, "↻ Replay", false).clicked() {
                        st = Live::new(st.proto);
                    }
                    let status = match (&st.session.status, st.waiting_since) {
                        (_, Some(since)) => {
                            format!("waiting for approval · auto in {:.0}s", (AUTO_APPROVE - (now - since)).max(0.0))
                        }
                        (RunStatus::Running, _) => format!("streaming · {:.0}%", st.replay.progress() * 100.0),
                        (RunStatus::Failed(_), _) => "run failed".to_owned(),
                        (RunStatus::Finished(StopReason::EndTurn), _) => "finished".to_owned(),
                        (RunStatus::Finished(s), _) => s.label().to_lowercase(),
                        (RunStatus::Idle, _) => "idle".to_owned(),
                    };
                    ui.label(egui::RichText::new(status).size(11.5).color(t.muted_foreground));
                });
            });
            ui.add_space(10.0);
            let out = AgentThread::new(&mut st.session).height(420.0).id_salt(st.proto.label()).show(ui);
            if out.decisions.iter().any(|d| !d.approved) {
                // the recording only has the approved branch: end the run politely
                st.replay.stop();
                st.session.extend([
                    AgentEvent::RunStart { run_id: None },
                    AgentEvent::text("denied", "Okay, I won't do that. Tell me if you'd like another option."),
                    AgentEvent::TextEnd { id: "denied".into() },
                    AgentEvent::RunFinish { stop: StopReason::EndTurn },
                ]);
            }
        });
    });

    glowkit_core::animate(ui);
    glowkit_core::set_state(ui, id, st);
}

fn pill(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let t = Theme::of(ui);
    let font = FontId::proportional(12.5);
    let g = ui.painter().layout_no_wrap(label.to_owned(), font, t.foreground);
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 22.0, 28.0), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    let fill = if active {
        with_alpha(t.primary, 0.16)
    } else if hovered {
        with_alpha(t.foreground, 0.06)
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke = if active { with_alpha(t.primary, 0.6) } else { t.border };
    p.rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let color = if active { t.foreground } else { t.muted_foreground };
    p.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.5), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn all() -> Vec<Demo> {
    vec![Demo {
        id: "live-agent-stream",
        name: "Live agent stream",
        category: Category::Ai,
        description: "A recorded agent run replayed through the real stream reducer: reasoning, streamed Markdown, tool calls with live arguments, an approval, a plan and an error. Switch between AG-UI, Vercel AI SDK and ACP-style sources.",
        covers: &[
            "Vercel AI Elements: Conversation",
            "prompt-kit: Response Stream",
            "assistant-ui: Thread",
            "CopilotKit: CopilotChat",
        ],
        code: r##"// background task: any adapter -> bridge
let (tx, mut rx) = glowkit_stream::bridge::channel();
let tx = tx.with_repaint(ctx);
let mut decoder = SseDecoder::new(SseProtocol::AgUi);
for chunk in body { tx.send_all(decoder.feed(&chunk)).ok(); }

// UI, every frame
rx.drain_into(&mut session);
let out = AgentThread::new(&mut session).show(ui);
for d in out.decisions {
    // send the approval back to the agent
}"##,
        show: live_agent_stream,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_protocol_fixture_replays_to_completion() {
        for p in Proto::ALL {
            let mut live = Live::new(p);
            let mut now = 0.0;
            while !live.replay.is_done() && now < 600.0 {
                live.replay.advance(now, &mut live.session);
                let ids: Vec<String> = live.session.pending_approvals().map(|a| a.id.clone()).collect();
                for a in ids {
                    live.session.apply(AgentEvent::decision(a, true));
                }
                now += 0.1;
            }
            assert!(live.replay.is_done(), "{} replay finished", p.label());
            assert!(live.session.messages.len() >= 2);
        }
    }
}

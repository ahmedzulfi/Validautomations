//! Replays the bundled recordings through the adapters and the reducer.
#![cfg(all(feature = "ag-ui", feature = "vercel", feature = "acp-wire"))]

use glowkit_stream::adapters::{SseDecoder, SseProtocol};
use glowkit_stream::{fixtures, AgentEvent, Part, PlanStatus, Role, RunStatus, Session, StopReason, ToolState};

fn session(events: Vec<AgentEvent>) -> Session {
    let mut s = Session::new();
    s.extend(events);
    s
}

fn reasoning_count(s: &Session) -> usize {
    s.messages.iter().flat_map(|m| &m.parts).filter(|p| matches!(p, Part::Reasoning(_))).count()
}

fn all_text(s: &Session) -> String {
    s.messages.iter().map(|m| m.text()).collect::<Vec<_>>().join("\n")
}

/// Decoding in odd-sized chunks gives exactly the same events as decoding at once.
fn assert_chunking_is_invisible(protocol: SseProtocol, body: &str) {
    let whole = SseDecoder::decode_all(protocol, body.as_bytes());
    for size in [1, 7, 64, 333] {
        let mut d = SseDecoder::new(protocol);
        let mut split = Vec::new();
        for c in body.as_bytes().chunks(size) {
            split.extend(d.feed(c));
        }
        split.extend(d.finish());
        assert_eq!(split, whole, "chunk size {size}");
        assert_eq!(d.skipped, 0);
    }
}

#[test]
fn ag_ui_recording() {
    assert_chunking_is_invisible(SseProtocol::AgUi, fixtures::AG_UI_RUN);
    let events = fixtures::ag_ui_events();

    // stop where the first run pauses for approval
    let pause = events
        .iter()
        .position(|e| matches!(e, AgentEvent::RunFinish { stop: StopReason::Interrupted }))
        .expect("first run ends with an interrupt");
    let mut s = session(events[..=pause].to_vec());
    let approval = s.pending_approvals().next().expect("approval requested").clone();
    assert_eq!(approval.title, "Book SkyFjord SF 512 for €89?");
    assert_eq!(approval.tool_call_id.as_deref(), Some("call-book-1"));
    let book = s.tool_call("call-book-1").unwrap();
    assert_eq!(book.state, ToolState::Pending);
    assert_eq!(book.input.as_ref().unwrap()["flight"], "SF 512");
    assert_eq!(s.status, RunStatus::Finished(StopReason::Interrupted));
    assert_eq!(s.usage.input_tokens, Some(1840));

    // the user approves, the second run continues
    s.apply(AgentEvent::decision(&approval.id, true));
    assert_eq!(s.tool_call("call-book-1").unwrap().state, ToolState::Running);
    s.extend(events[pause + 1..].to_vec());

    let search = s.tool_call("call-search-1").unwrap();
    assert_eq!((search.name.as_str(), search.state), ("search_flights", ToolState::Done));
    assert_eq!(search.input.as_ref().unwrap()["to"], "LIS");
    assert_eq!(search.output.as_ref().unwrap()["flights"][0]["fare_eur"], 89);
    let book = s.tool_call("call-book-1").unwrap();
    assert_eq!(book.state, ToolState::Done);
    assert_eq!(book.output.as_ref().unwrap()["confirmation"], "LX7Q2P");
    assert_eq!(s.approval(&approval.id).unwrap().decision, Some(true));

    assert_eq!(reasoning_count(&s), 2, "REASONING_* and the deprecated THINKING_* both render");
    let text = all_text(&s);
    assert!(text.contains("**SkyFjord SF 512**"), "TEXT_MESSAGE_CHUNK text arrives");
    assert!(text.contains("LX7Q2P"));
    assert_eq!(s.messages.iter().filter(|m| m.role == Role::Assistant).count(), 2);

    assert!(matches!(&s.status, RunStatus::Failed(m) if m.starts_with("Mail service unavailable")));
    let last = s.messages.last().unwrap().parts.last().unwrap();
    assert!(matches!(last, Part::Error(e) if e.code.as_deref() == Some("UPSTREAM_UNAVAILABLE")));
    assert_eq!(s.usage.input_tokens, Some(2310));
    let customs: Vec<&str> = s.custom.iter().map(|(n, _)| n.as_str()).collect();
    for name in ["ag-ui/state_snapshot", "ag-ui/state_delta", "ag-ui/activity_snapshot", "email_status"] {
        assert!(customs.contains(&name), "{name} kept");
    }
    assert!(s.messages.iter().flat_map(|m| &m.parts).all(|p| match p {
        Part::Text(t) => !t.streaming,
        Part::Reasoning(r) => !r.streaming,
        _ => true,
    }));
}

#[test]
fn vercel_recording() {
    assert_chunking_is_invisible(SseProtocol::Vercel, fixtures::VERCEL_RUN);
    let events = fixtures::vercel_events();
    let pause = events
        .iter()
        .position(|e| matches!(e, AgentEvent::RunFinish { stop: StopReason::ToolCalls }))
        .expect("first response ends with tool-calls");
    let mut s = session(events[..=pause].to_vec());
    let approval = s.pending_approvals().next().expect("approval requested").clone();
    assert_eq!(approval.id, "appr-1");
    assert_eq!(approval.title, "Allow book_flight?");
    assert_eq!(s.messages[0].sources().count(), 1);
    assert!(s.custom.iter().any(|(n, v)| n == "vercel/data-fare-hold" && v["flight"] == "SF 512"));

    s.apply(AgentEvent::decision("appr-1", true));
    s.extend(events[pause + 1..].to_vec());
    let search = s.tool_call("call-search-1").unwrap();
    assert!(search.input_final);
    assert_eq!(search.state, ToolState::Done);
    let book = s.tool_call("call-book-1").unwrap();
    assert_eq!(book.state, ToolState::Done);
    assert_eq!(book.output.as_ref().unwrap()["charged_eur"], 89);
    assert_eq!(reasoning_count(&s), 2);
    assert!(all_text(&s).contains("| SF 512 | 07:40 | 11:55 | €89 |"));
    assert!(matches!(&s.status, RunStatus::Failed(m) if m.contains("(503)")));
    assert!(matches!(s.messages.last().unwrap().parts.last(), Some(Part::Error(_))));
}

#[test]
fn vercel_denied_branch() {
    // what the UI does when the user denies instead
    let events = fixtures::vercel_events();
    let pause = events.iter().position(|e| matches!(e, AgentEvent::RunFinish { .. })).unwrap();
    let mut s = session(events[..=pause].to_vec());
    s.apply(AgentEvent::decision("appr-1", false));
    assert_eq!(s.tool_call("call-book-1").unwrap().state, ToolState::Denied);
    assert_eq!(s.pending_approvals().count(), 0);
}

#[test]
fn acp_recording() {
    let s = session(fixtures::acp_events());
    let parts = &s.messages[0].parts;
    assert!(matches!(&parts[0], Part::Plan(_)), "the plan is shown first and updated in place");
    assert_eq!(parts.iter().filter(|p| matches!(p, Part::Plan(_))).count(), 1);
    if let Part::Plan(entries) = &parts[0] {
        let st: Vec<PlanStatus> = entries.iter().map(|e| e.status).collect();
        assert_eq!(st, [PlanStatus::Completed, PlanStatus::Completed, PlanStatus::Failed]);
    }
    assert_eq!(reasoning_count(&s), 1);

    let read = s.tool_call("call-read-1").unwrap();
    assert_eq!((read.name.as_str(), read.state), ("Read src/tokenizer.rs", ToolState::Done));
    assert!(read.output_pretty().unwrap().contains("self.src[self.pos]"));
    let edit = s.tool_call("call-edit-1").unwrap();
    assert_eq!(edit.state, ToolState::Done);
    let approval = s.approval("11").unwrap();
    assert_eq!(approval.options.len(), 3);
    assert_eq!(approval.options.iter().filter(|o| o.approves).count(), 2);
    assert_eq!(approval.decision, Some(true));
    let test = s.tool_call("call-test-1").unwrap();
    assert_eq!(test.state, ToolState::Error);
    assert!(test.error.as_deref().unwrap().contains("1 failed"));

    // text segments stay in the order the agent worked
    let order: Vec<&str> = parts
        .iter()
        .filter_map(|p| match p {
            Part::Text(_) => Some("text"),
            Part::ToolCall(_) => Some("tool"),
            _ => None,
        })
        .collect();
    assert_eq!(order, ["text", "tool", "text", "tool", "tool", "text"]);
    assert!(s.messages[0].text().contains("unicode_idents"));
    assert_eq!((s.usage.context_used, s.usage.context_size), (Some(48210), Some(200000)));
    assert_eq!(s.status, RunStatus::Finished(StopReason::EndTurn));
}

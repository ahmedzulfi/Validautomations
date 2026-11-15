use super::common::*;
use super::*;
use egui::{pos2, vec2, Pos2, Rect, Sense, Ui, Vec2};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;

const C: Category = Category::Ai;

pub(super) fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, egui::Id::unique(("asst_demo", key)), init)
}

pub(super) fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, egui::Id::unique(("asst_demo", key)), v);
}

/// A centered column of fixed width.
pub(super) fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.vertical_centered(|ui| {
        ui.set_max_width(width.min(ui.available_width()));
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), add);
    });
}

/// A scripted autoplay director: a step counter with the time the step started, paused while the
/// user interacts with the preview.
#[derive(Clone, Copy)]
struct Script {
    step: usize,
    at: f64,
}

pub(super) struct Director {
    key: &'static str,
    pub(super) step: usize,
    pub(super) elapsed: f32,
    pub(super) playing: bool,
    now: f64,
}

impl Director {
    /// `area` is the preview rect from the previous frame (user activity there pauses autoplay).
    pub(super) fn new(ui: &Ui, key: &'static str, resume_after: f32) -> Self {
        let now = ui.input(|i| i.time);
        let area: Rect = mem(ui, &format!("{key}_area"), || Rect::NOTHING);
        let active = user_active(ui, area);
        let idle = idle_for(ui, ui.scope_id().with(key), active);
        let s: Script = mem(ui, key, || Script { step: 0, at: now });
        let playing = idle > resume_after;
        if !playing {
            // keep the step clock frozen while paused
            set_mem(ui, key, Script { step: s.step, at: now - 0.0 });
        }
        glowkit_core::animate(ui);
        Self { key, step: s.step, elapsed: (now - s.at) as f32, playing, now }
    }
    pub(super) fn go(&mut self, ui: &Ui, step: usize) {
        self.step = step;
        self.elapsed = 0.0;
        set_mem(ui, self.key, Script { step, at: self.now });
    }
    pub(super) fn area(&self, ui: &Ui, r: Rect) {
        set_mem(ui, &format!("{}_area", self.key), r);
    }
}

/// Spring the ghost cursor toward `target` and paint it.
pub(super) fn ghost(ui: &Ui, key: &str, target: Pos2, visible: bool, pressed: bool) -> Pos2 {
    let t = Theme::of(ui);
    let id = ui.scope_id().with((key, "ghost"));
    let p = glowkit_core::anim::spring2(ui, id, target.to_vec2(), 90.0, 16.0).to_pos2();
    let a = toggle(ui, id.with("a"), visible);
    ghost_cursor(&ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, id)), &t, p, a, pressed);
    p
}

// ---------------------------------------------------------------------------------------------

const R_SUMMARY: &str = "Here's a tighter version:\n\n**Q3 in one line:** revenue grew **18%** on the back of self-serve, while churn fell to **2.1%**.\n\n- Expansion revenue doubled in EMEA\n- Support tickets dropped 30% after the onboarding revamp\n- Next: price the team plan before Q4";
const R_SUMMARY_2: &str = "Shorter still:\n\n> Self-serve drove **+18%** revenue; churn is down to **2.1%**. EMEA expansion doubled and onboarding cut tickets by a third.\n\nWant it as a slide title instead?";
const R_EDIT: &str = "Sure, here it is for the board deck:\n\n1. **Growth** — revenue up 18% quarter over quarter\n2. **Retention** — churn at an all-time low of 2.1%\n3. **Efficiency** — 30% fewer support tickets\n\nEach point fits on one slide with a single chart.";

fn thread_seed() -> ThreadModel {
    ThreadModel::new().with_exchange(
        "Summarize the Q3 report for me",
        "The Q3 report covers revenue, retention and support. Revenue grew **18%** quarter over quarter, mostly from self-serve upgrades. Churn fell to **2.1%**, and support volume dropped after the new onboarding flow shipped.",
    )
}

fn demo_thread(ui: &mut Ui) {
    let t = Theme::of(ui);
    let mut model: ThreadModel = mem(ui, "model", thread_seed);
    let mut d = Director::new(ui, "dir", 5.0);
    let mut speed: f32 = mem(ui, "speed", || 90.0);
    model.chars_per_sec = speed;
    let rects: (Option<Rect>, Option<Rect>, Option<Rect>) = mem(ui, "rects", || (None, None, None));
    let mut cursor = None::<(Pos2, bool)>;
    if d.playing {
        let e = d.elapsed;
        match d.step {
            0 => {
                if e > 1.0 {
                    model = thread_seed();
                    model.chars_per_sec = speed;
                    model.ask("Can you make that shorter?");
                    model.stream_reply(R_SUMMARY);
                    d.go(ui, 1);
                }
            }
            1 => {
                if !model.is_streaming() && e > 2.0 {
                    d.go(ui, 2);
                }
            }
            2 => {
                if let Some(r) = rects.0 {
                    cursor = Some((r.center(), e > 1.1 && e < 1.3));
                }
                if e > 1.3 {
                    let last = model.turns.len() - 1;
                    model.regenerate_with(last, R_SUMMARY_2);
                    d.go(ui, 3);
                }
            }
            3 => {
                if let Some(r) = rects.0 {
                    cursor = Some((r.center() + vec2(30.0, 40.0), false));
                }
                if !model.is_streaming() && e > 2.5 {
                    d.go(ui, 4);
                }
            }
            4 => {
                if let Some(r) = rects.1 {
                    cursor = Some((r.center(), e > 1.1 && e < 1.3));
                }
                if e > 1.3 {
                    let last = model.turns.len() - 1;
                    model.begin_edit(last);
                    if let Some(b) = model.edit_buffer() {
                        b.clear();
                    }
                    d.go(ui, 5);
                }
            }
            5 => {
                let target = "Make it three bullets for the board deck";
                let n = ((e - 0.3) * 24.0).max(0.0) as usize;
                if let Some(b) = model.edit_buffer() {
                    *b = take_chars(target, n).to_owned();
                }
                if let Some(r) = rects.2 {
                    cursor = Some((r.center(), e > 2.4 && e < 2.6));
                }
                if e > 2.6 {
                    model.submit_edit();
                    model.stream_reply(R_EDIT);
                    d.go(ui, 6);
                }
            }
            _ => {
                if !model.is_streaming() && e > 4.0 {
                    d.go(ui, 0);
                }
            }
        }
    }
    column(ui, 720.0, |ui| {
        let top = ui.cursor().min;
        let p = ui.painter();
        let _ = p;
        let out = AssistantThread::new(&mut model).height(300.0).show(ui);
        for e in out.events {
            match e {
                ThreadEvent::Regenerate(turn) => model.regenerate_with(turn, R_SUMMARY_2),
                ThreadEvent::Edited(_) => model.stream_reply(R_EDIT),
                _ => {}
            }
        }
        set_mem(ui, "rects", (out.regenerate_rect, out.edit_rect, out.send_edit_rect));
        ui.add_space(6.0);
        let mut text: String = mem(ui, "draft", String::new);
        let po = glowkit_ai::PromptInput::new(&mut text)
            .placeholder("Message the assistant…")
            .generating(model.is_streaming())
            .min_rows(1)
            .id_salt("thread_prompt")
            .show(ui, |_| {});
        if let Some(s) = po.submitted {
            model.ask(&s);
            model.stream_reply("Got it. In a real app this is where your model's reply streams in, token by token, with the same caret and soft fade you see here.");
        }
        if po.stop {
            model.finish_stream();
        }
        set_mem(ui, "draft", text);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                glowkit_effects::ControlSlider::new("Stream speed", &mut speed, 20.0..=240.0)
                    .suffix(" ch/s")
                    .width(260.0),
            );
        });
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(720.0, 0.0)));
        let _ = t;
    });
    if let Some((p, pressed)) = cursor {
        ghost(ui, "thread", p, true, pressed);
    } else {
        ghost(ui, "thread", pos2(0.0, 0.0), false, false);
    }
    set_mem(ui, "speed", speed);
    set_mem(ui, "model", model);
    let _ = (Sense::hover(), Vec2::ZERO);
}

pub(super) fn all() -> Vec<Demo> {
    let mut v = thread_demos();
    v.extend(super::demos_chrome::all());
    v.extend(super::demos_tools::all());
    v
}

fn thread_demos() -> Vec<Demo> {
    vec![Demo {
        id: "asst-thread",
        name: "Assistant Thread",
        category: C,
        description: "A branching chat thread: streaming Markdown replies, regenerate with a reply picker, and inline edit-and-resend that forks the conversation.",
        covers: &["assistant-ui: Thread", "assistant-ui: Branch Picker", "assistant-ui: Edit Composer", "nextui (agent): Conversation"],
        code: r##"let mut model = ThreadModel::new().with_exchange("Summarize Q3", "Revenue grew 18%...");
let out = AssistantThread::new(&mut model).height(320.0).show(ui);
for e in out.events {
    match e {
        ThreadEvent::Regenerate(turn) => model.regenerate_with(turn, &next_reply()),
        ThreadEvent::Edited(_) => model.stream_reply(&next_reply()),
        _ => {}
    }
}"##,
        show: demo_thread,
    }]
}

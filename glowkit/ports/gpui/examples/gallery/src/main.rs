//! glowkit for GPUI: component gallery.
//!
//! ```text
//! cargo run -p glowkit-gpui-gallery                       # overview page, Midnight theme
//! cargo run -p glowkit-gpui-gallery -- --page chat --theme daylight
//! ```

use glowkit_gpui::color::{mix, palettes, with_alpha};
use glowkit_gpui::prelude::*;
use gpui::prelude::*;
use gpui::{
    div, px, size, AnyElement, App, Application, Bounds, ClickEvent, Context, Entity, FontWeight, SharedString,
    Task, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Overview,
    ShinyText,
    GradientText,
    CountUp,
    StreamingText,
    ShimmerButton,
    MagneticButton,
    BorderBeam,
    SpotlightCard,
    TiltCard,
    Marquee,
    AnimatedTabs,
    Toasts,
    Chat,
    PromptInput,
    Approval,
    Sessions,
    StatTile,
}

impl Page {
    const ALL: [Page; 18] = [
        Page::Overview,
        Page::ShinyText,
        Page::GradientText,
        Page::CountUp,
        Page::StreamingText,
        Page::ShimmerButton,
        Page::MagneticButton,
        Page::BorderBeam,
        Page::SpotlightCard,
        Page::TiltCard,
        Page::Marquee,
        Page::AnimatedTabs,
        Page::Toasts,
        Page::Chat,
        Page::PromptInput,
        Page::Approval,
        Page::Sessions,
        Page::StatTile,
    ];

    /// (slug, title, category, description)
    fn info(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Page::Overview => ("overview", "Overview", "Start", "A tour of glowkit's flagship components, rebuilt on GPUI."),
            Page::ShinyText => ("shiny-text", "Shiny Text", "Text", "A band of light sweeps across the glyphs, like light catching metal."),
            Page::GradientText => ("gradient-text", "Gradient Text", "Text", "Text filled with the theme's brand gradient, flowing slowly."),
            Page::CountUp => ("count-up", "Count Up", "Text", "Numbers that count up with an ease-out curve when they appear or change."),
            Page::StreamingText => ("streaming-text", "Streaming Text", "Text", "Typewriter reveal with a caret, or a live caret for text still arriving from a model."),
            Page::ShimmerButton => ("shimmer-button", "Shimmer Button", "Buttons", "A spark of light races around the border; an inner glow rises on hover."),
            Page::MagneticButton => ("magnetic-button", "Magnetic Button", "Buttons", "Leans toward the pointer and springs back with a bouncy spring."),
            Page::BorderBeam => ("border-beam", "Border Beam", "Cards", "A beam of light with a soft head and long tail travels around the card."),
            Page::SpotlightCard => ("spotlight-card", "Spotlight Card", "Cards", "A soft spotlight follows the pointer and catches the nearby border."),
            Page::TiltCard => ("tilt-card", "Tilt Card", "Cards", "2.5D tilt: parallax, a glare from the pointer side and a counter-shifted shadow."),
            Page::Marquee => ("marquee", "Marquee", "Motion", "An endless strip that eases to a stop on hover, with faded edges."),
            Page::AnimatedTabs => ("animated-tabs", "Animated Tabs", "Motion", "The highlight springs between tabs and stretches in the direction of travel."),
            Page::Toasts => ("toasts", "Toast Stack", "Motion", "Sonner-style stack: toasts spring in, tuck into a deck and fan out on hover."),
            Page::Chat => ("chat", "Chat + Streaming", "AI", "Chat turns with a tiny Markdown subset and a live streaming caret. Type below and press Enter."),
            Page::PromptInput => ("prompt-input", "Prompt Input", "AI", "A focusable composer with an animated focus ring and a send / stop button."),
            Page::Approval => ("approval", "Approval Card", "AI", "Human-in-the-loop tool approval: command, reason, risk and wait time."),
            Page::Sessions => ("sessions", "Agent Sessions", "AI", "Sessions across devices, grouped by urgency, with live status pulses."),
            Page::StatTile => ("stat-tile", "Stat Tile", "Data", "KPI tiles with count-up values, delta badges and draw-in sparklines."),
        }
    }

    fn from_slug(s: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|p| p.info().0 == s)
    }
}

struct Msg {
    role: Role,
    text: String,
    streaming: bool,
}

struct Approval {
    agent: &'static str,
    tool: &'static str,
    command: &'static str,
    reason: &'static str,
    risk: Risk,
    decision: Option<Decision>,
    since: Instant,
}

struct Gallery {
    page: Page,
    toasts: Entity<ToastStack>,
    prompt: Entity<PromptInput>,
    chat: Vec<Msg>,
    stream: Option<Task<()>>,
    tabs: [usize; 3],
    approvals: Vec<Approval>,
    session: Option<u64>,
    replay: u64,
    toast_n: usize,
}

const REPLY: &str = "Here's the short version:\n\n- GPUI renders on the **GPU** every frame, so effects like `ShinyText` are just per-glyph colors.\n- Springs live in `use_keyed_state` entities and request frames until they settle.\n\n```\nTheme::dark().install(cx);\n```\nAnything else you want to animate?";

impl Gallery {
    fn new(page: Page, cx: &mut Context<Self>) -> Self {
        let toasts = cx.new(|_| ToastStack::new());
        let prompt = cx.new(PromptInput::new);
        cx.subscribe(&prompt, |this, _, ev: &PromptSubmit, cx| {
            this.chat.push(Msg { role: Role::User, text: ev.0.clone(), streaming: false });
            this.start_reply(cx);
        })
        .detach();
        cx.subscribe(&prompt, |this, _, _: &PromptStop, cx| this.stop_reply(cx)).detach();
        let now = Instant::now();
        let mut g = Self {
            page,
            toasts,
            prompt,
            chat: vec![
                Msg { role: Role::System, text: "Session started \u{00B7} glow-1".into(), streaming: false },
                Msg { role: Role::User, text: "How do animations work in **glowkit for GPUI**?".into(), streaming: false },
                Msg {
                    role: Role::Assistant,
                    text: "Two ways. Looping effects use GPUI's declarative `with_animation`; interactive ones run glowkit's **spring** physics and request animation frames until they settle.".into(),
                    streaming: false,
                },
                Msg { role: Role::User, text: "Show me the short version?".into(), streaming: false },
            ],
            stream: None,
            tabs: [0, 1, 0],
            approvals: vec![
                Approval {
                    agent: "deploy-bot",
                    tool: "shell",
                    command: "kubectl rollout restart deploy/api -n prod",
                    reason: "The new config map only takes effect after a restart. 3 replicas, rolling.",
                    risk: Risk::High,
                    decision: None,
                    since: now - Duration::from_secs(42),
                },
                Approval {
                    agent: "refactor-agent",
                    tool: "fs.write",
                    command: "write crates/core/src/theme.rs (+48 -12)",
                    reason: "Adds a `surface()` token and migrates 9 call sites.",
                    risk: Risk::Medium,
                    decision: None,
                    since: now - Duration::from_secs(8),
                },
                Approval {
                    agent: "docs-agent",
                    tool: "git",
                    command: "git commit -m \"docs: gallery screenshots\"",
                    reason: "Docs only.",
                    risk: Risk::Low,
                    decision: Some(Decision::Approved),
                    since: now,
                },
            ],
            session: Some(2),
            replay: 0,
            toast_n: 0,
        };
        if page == Page::Chat {
            g.start_reply(cx);
        }
        if page == Page::Toasts {
            for _ in 0..3 {
                g.push_toast(cx);
            }
        }
        g
    }

    fn start_reply(&mut self, cx: &mut Context<Self>) {
        self.chat.push(Msg { role: Role::Assistant, text: String::new(), streaming: true });
        self.prompt.update(cx, |p, cx| p.set_busy(true, cx));
        let chars: Vec<char> = REPLY.chars().collect();
        self.stream = Some(cx.spawn(async move |this, cx| {
            let mut i = 0;
            while i < chars.len() {
                cx.background_executor().timer(Duration::from_millis(28)).await;
                let n = 2 + (i * 7) % 3; // a little jitter, like tokens
                let chunk: String = chars[i..(i + n).min(chars.len())].iter().collect();
                i += n;
                let done = i >= chars.len();
                let ok = this.update(cx, |g, cx| {
                    if let Some(m) = g.chat.last_mut() {
                        m.text.push_str(&chunk);
                        if done {
                            m.streaming = false;
                        }
                    }
                    if done {
                        g.prompt.update(cx, |p, cx| p.set_busy(false, cx));
                    }
                    cx.notify();
                });
                if ok.is_err() {
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn stop_reply(&mut self, cx: &mut Context<Self>) {
        self.stream = None;
        if let Some(m) = self.chat.last_mut() {
            m.streaming = false;
        }
        self.prompt.update(cx, |p, cx| p.set_busy(false, cx));
        cx.notify();
    }

    fn push_toast(&mut self, cx: &mut Context<Self>) {
        let samples = [
            Toast::new("Deployment finished").description("api-prod is live in 3 regions").kind(ToastKind::Success),
            Toast::new("Agent needs approval").description("deploy-bot wants to run kubectl").kind(ToastKind::Warning),
            Toast::new("New model available").description("glow-2 is 30% faster on long contexts").kind(ToastKind::Info),
            Toast::new("Build failed").description("3 tests failed in glowkit-charts").kind(ToastKind::Error),
            Toast::new("Saved").description("Theme preset \u{201C}Ember\u{201D} applied").kind(ToastKind::Default),
        ];
        let toast = samples[self.toast_n % samples.len()].clone().duration(6.0);
        self.toast_n += 1;
        self.toasts.update(cx, |s, cx| {
            s.push(toast, cx);
        });
    }

    // -- chrome ---------------------------------------------------------------------------------

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme().clone();
        let mut items: Vec<AnyElement> = Vec::new();
        let mut last_cat = "";
        for p in Page::ALL {
            let (_, title, cat, _) = p.info();
            if cat != last_cat {
                items.push(
                    div()
                        .px(px(12.))
                        .pt(px(14.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(mix(t.muted_foreground, t.background, 0.25))
                        .child(cat.to_uppercase())
                        .into_any_element(),
                );
                last_cat = cat;
            }
            let on = p == self.page;
            items.push(
                div()
                    .id(SharedString::from(format!("nav-{}", p.info().0)))
                    .mx(px(6.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .text_color(if on { t.foreground } else { mix(t.muted_foreground, t.foreground, 0.2) })
                    .when(on, |el| el.bg(with_alpha(t.foreground, if t.dark { 0.08 } else { 0.06 })).font_weight(FontWeight::MEDIUM))
                    .when(!on, |el| el.hover(|s| s.bg(with_alpha(t.foreground, 0.04)).text_color(t.foreground)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.page = p;
                        this.replay += 1;
                        cx.notify();
                    }))
                    .child(title)
                    .into_any_element(),
            );
        }
        let themes = Theme::presets();
        div()
            .flex_none()
            .w(px(232.))
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(t.border)
            .bg(if t.dark { mix(t.background, t.card, 0.5) } else { mix(t.background, t.muted, 0.5) })
            .child(
                div()
                    .px(px(18.))
                    .pt(px(18.))
                    .pb(px(6.))
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .child(GradientText::new("brand", "\u{2726} glowkit").size(20.0).spread(0.8))
                    .child(div().text_size(px(12.)).text_color(t.muted_foreground).child("for GPUI")),
            )
            .child(div().id("nav").flex_1().overflow_y_scroll().pb(px(12.)).children(items))
            .child(
                div()
                    .p(px(12.))
                    .border_t_1()
                    .border_color(t.border)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(div().text_size(px(11.)).text_color(t.muted_foreground).child("THEME"))
                    .child(div().flex().flex_wrap().gap(px(6.)).children(themes.into_iter().map(|th| {
                        let on = th.name == t.name;
                        let (name, sw_a, sw_b, bg) = (th.name, th.gradient[0], th.gradient[1], th.background);
                        div()
                            .id(SharedString::from(format!("theme-{name}")))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .px(px(8.))
                            .py(px(4.))
                            .rounded_full()
                            .border_1()
                            .border_color(if on { t.ring } else { t.border })
                            .text_size(px(11.5))
                            .text_color(if on { t.foreground } else { t.muted_foreground })
                            .cursor_pointer()
                            .hover(|s| s.text_color(t.foreground))
                            .on_click(move |_, _, cx| Theme::by_name(name).unwrap_or_default().install(cx))
                            .child(
                                div().size(px(10.)).rounded_full().border_1().border_color(bg).bg(gpui::linear_gradient(
                                    135.,
                                    gpui::linear_color_stop(sw_a, 0.0),
                                    gpui::linear_color_stop(sw_b, 1.0),
                                )),
                            )
                            .child(name)
                    }))),
            )
    }

    fn stage(t: &Theme, content: impl IntoElement) -> impl IntoElement {
        div()
            .w_full()
            .min_h(px(260.))
            .p(px(40.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(20.))
            .rounded(px(t.radius * 1.6))
            .border_1()
            .border_color(t.border)
            .bg(if t.dark { mix(t.background, t.card, 0.35) } else { mix(t.background, t.muted, 0.35) })
            .child(content)
    }

    fn caption(t: &Theme, text: &'static str) -> impl IntoElement {
        div().text_size(px(12.)).text_color(t.muted_foreground).child(text)
    }

    // -- pages ----------------------------------------------------------------------------------

    fn page_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.theme().clone();
        let r = self.replay;
        match self.page {
            Page::Overview => self.overview(&t, cx).into_any_element(),
            Page::ShinyText => Self::stage(
                &t,
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(18.))
                    .child(ShinyText::new("shiny-hero", "Ship beautiful native apps").size(44.0))
                    .child(ShinyText::new("shiny-sub", "Pure Rust \u{00B7} GPU accelerated \u{00B7} 120 fps").size(18.0).speed(2.2))
                    .child(
                        ShinyText::new("shiny-accent", "Now on GPUI")
                            .size(26.0)
                            .color(with_alpha(t.accent, 0.55))
                            .shine_color(gpui::white())
                            .band(0.3),
                    ),
            )
            .into_any_element(),
            Page::GradientText => Self::stage(
                &t,
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(14.))
                    .child(GradientText::new("grad-1", "Make it glow.").size(64.0))
                    .child(GradientText::new("grad-2", "Aurora palette, slower and wider").size(28.0).colors(palettes::aurora()).speed(10.0).spread(0.9))
                    .child(GradientText::new("grad-3", "rainbow / neon / magic").size(22.0).colors(palettes::rainbow()).speed(4.0)),
            )
            .into_any_element(),
            Page::CountUp => Self::stage(
                &t,
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(18.))
                    .child(
                        div()
                            .flex()
                            .gap(px(48.))
                            .child(stat_count(&t, &format!("cu-users-{r}"), 12_840.0, "", "", 0, "Active users"))
                            .child(stat_count(&t, &format!("cu-rev-{r}"), 48_213.57, "$", "", 2, "Revenue"))
                            .child(stat_count(&t, &format!("cu-up-{r}"), 99.98, "", "%", 2, "Uptime")),
                    )
                    .child(ShimmerButton::new("cu-replay", "Replay").size(13.0).on_click(cx.listener(|this, _, _, cx| {
                        this.replay += 1;
                        cx.notify();
                    }))),
            )
            .into_any_element(),
            Page::StreamingText => Self::stage(
                &t,
                div()
                    .w(px(560.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .child(Self::caption(&t, "Typewriter (45 chars/s)"))
                    .child(StreamingText::new(("typewriter", r as usize), "glowkit gives Rust apps the polish web apps get from React component libraries: springs, glows, beams and streaming text, all native.").text_size(px(16.)))
                    .child(Self::caption(&t, "Live (text arriving from a model)"))
                    .child(StreamingText::new("live", "Thinking about the best way to lay this out").live(true).text_size(px(16.)).color(t.muted_foreground))
                    .child(div().child(ShimmerButton::new("st-replay", "Replay").size(13.0).on_click(cx.listener(|this, _, _, cx| {
                        this.replay += 1;
                        cx.notify();
                    })))),
            )
            .into_any_element(),
            Page::ShimmerButton => Self::stage(
                &t,
                div()
                    .flex()
                    .items_center()
                    .gap(px(20.))
                    .child(ShimmerButton::new("sb-1", "Shimmer Button").on_click(cx.listener(|this, _, _, cx| this.push_toast(cx))))
                    .child(ShimmerButton::new("sb-2", "Get started  \u{2192}").size(17.0).duration(2.2))
                    .child(ShimmerButton::new("sb-3", "Accent").background(t.accent).shimmer_color(gpui::white()).size(14.0)),
            )
            .into_any_element(),
            Page::MagneticButton => Self::stage(
                &t,
                div()
                    .flex()
                    .items_center()
                    .gap(px(24.))
                    .child(MagneticButton::new("mb-1", "Hover me"))
                    .child(MagneticButton::new("mb-2", "Strong pull").strength(0.6).reach(44.0).size(17.0))
                    .child(Self::caption(&t, "Move the pointer near the buttons.")),
            )
            .into_any_element(),
            Page::BorderBeam => Self::stage(
                &t,
                div()
                    .flex()
                    .gap(px(24.))
                    .child(beam_card(&t, "beam-1", "Border Beam", "6 s per lap, theme colors").w(px(300.)))
                    .child(
                        BorderBeam::new("beam-2")
                            .duration(4.0)
                            .length(0.35)
                            .thickness(2.0)
                            .from(t.chart[1])
                            .to(t.chart[2])
                            .reverse(true)
                            .w(px(300.))
                            .p(px(22.))
                            .child(card_body(&t, "Faster, reversed", "Custom colors, longer beam, 2 px")),
                    ),
            )
            .into_any_element(),
            Page::SpotlightCard => Self::stage(
                &t,
                div()
                    .flex()
                    .gap(px(24.))
                    .child(SpotlightCard::new("spot-1").w(px(320.)).p(px(24.)).child(card_body(&t, "Spotlight Card", "Move the pointer over the card: the light follows it and catches the border.")))
                    .child(SpotlightCard::new("spot-2").color(t.chart[1]).spot_size(130.0).w(px(320.)).p(px(24.)).child(card_body(&t, "Cyan, wider", "Any color and radius."))),
            )
            .into_any_element(),
            Page::TiltCard => Self::stage(
                &t,
                div()
                    .flex()
                    .gap(px(32.))
                    .child(
                        TiltCard::new("tilt-1").w(px(300.)).child(
                            div()
                                .p(px(24.))
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .child(GradientText::new("tilt-title", "Pro plan").size(28.0))
                                .child(div().text_size(px(13.)).text_color(t.muted_foreground).child("Parallax, glare and a lifting shadow. GPUI has no 3D transforms, so the tilt is emulated in 2.5D."))
                                .child(div().pt(px(6.)).child(ShimmerButton::new("tilt-cta", "Upgrade").size(13.0))),
                        ),
                    )
                    .child(TiltCard::new("tilt-2").max_shift(12.0).w(px(220.)).h(px(220.)).child(
                        div().size_full().flex().items_center().justify_center().child(ShinyText::new("tilt-shiny", "12 px").size(36.0)),
                    )),
            )
            .into_any_element(),
            Page::Marquee => Self::stage(
                &t,
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(Marquee::new("mq-1", ["Rust", "GPUI", "egui", "WebAssembly", "Bevy", "wgpu", "Tokio", "Zed", "Tauri", "Iced"]))
                    .child(Marquee::new("mq-2", ["Springs", "Easings", "Beams", "Glows", "Shimmer", "Spotlight", "Tilt", "Toasts"]).reverse(true).speed(36.0))
                    .child(Self::caption(&t, "Hover a strip: it eases to a stop.")),
            )
            .into_any_element(),
            Page::AnimatedTabs => {
                let tabs = self.tabs;
                Self::stage(
                    &t,
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(26.))
                        .child(AnimatedTabs::new("tabs-pill", ["Overview", "Analytics", "Reports", "Settings"]).selected(tabs[0]).on_change(self.tab_setter(0, cx)))
                        .child(AnimatedTabs::new("tabs-solid", ["Day", "Week", "Month", "Year"]).style(TabHighlight::Solid).selected(tabs[1]).on_change(self.tab_setter(1, cx)))
                        .child(AnimatedTabs::new("tabs-line", ["Code", "Issues", "Pull requests", "Actions"]).style(TabHighlight::Underline).selected(tabs[2]).on_change(self.tab_setter(2, cx))),
                )
                .into_any_element()
            }
            Page::Toasts => Self::stage(
                &t,
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(14.))
                    .child(ShimmerButton::new("toast-btn", "Show a toast").on_click(cx.listener(|this, _, _, cx| this.push_toast(cx))))
                    .child(Self::caption(&t, "Toasts stack in the bottom-right corner. Hover the stack to fan it out.")),
            )
            .into_any_element(),
            Page::Chat => self.chat_page(&t).into_any_element(),
            Page::PromptInput => Self::stage(
                &t,
                div()
                    .w(px(620.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(self.prompt.clone())
                    .child(Self::caption(&t, "Click to focus, type, Enter to send (the reply streams on the Chat page).")),
            )
            .into_any_element(),
            Page::Approval => {
                let cards: Vec<AnyElement> = self
                    .approvals
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        ApprovalCard::new(("approval", i), a.agent, a.tool, a.command)
                            .reason(a.reason)
                            .risk(a.risk)
                            .waiting(a.since.elapsed().as_secs_f32())
                            .decision(a.decision)
                            .on_approve(cx.listener(move |this, _, _, cx| this.decide(i, Decision::Approved, cx)))
                            .on_deny(cx.listener(move |this, _, _, cx| this.decide(i, Decision::Denied, cx)))
                            .into_any_element()
                    })
                    .collect();
                // keep the "waiting" timers ticking
                let pending = self.approvals.iter().any(|a| a.decision.is_none());
                if pending {
                    cx.spawn(async |this, cx| {
                        cx.background_executor().timer(Duration::from_millis(500)).await;
                        this.update(cx, |_, cx| cx.notify()).ok();
                    })
                    .detach();
                }
                Self::stage(&t, div().w(px(560.)).flex().flex_col().gap(px(14.)).children(cards)).into_any_element()
            }
            Page::Sessions => {
                let list = AgentSessionList::new("sessions", demo_sessions())
                    .selected(self.session)
                    .on_select({
                        let this = cx.entity().downgrade();
                        move |id, _, cx| {
                            this.update(cx, |g, cx| {
                                g.session = Some(id);
                                cx.notify();
                            })
                            .ok();
                        }
                    });
                Self::stage(&t, div().w(px(440.)).child(list)).into_any_element()
            }
            Page::StatTile => Self::stage(&t, stat_row(&t, r)).into_any_element(),
        }
    }

    fn tab_setter(&self, k: usize, cx: &mut Context<Self>) -> impl Fn(usize, &mut Window, &mut App) + 'static {
        let this = cx.entity().downgrade();
        move |i, _, cx| {
            this.update(cx, |g, cx| {
                g.tabs[k] = i;
                cx.notify();
            })
            .ok();
        }
    }

    fn decide(&mut self, i: usize, d: Decision, cx: &mut Context<Self>) {
        if let Some(a) = self.approvals.get_mut(i) {
            a.decision = Some(d);
            let toast = match d {
                Decision::Approved => Toast::new(format!("Approved {}", a.tool)).description(a.agent).kind(ToastKind::Success),
                Decision::Denied => Toast::new(format!("Denied {}", a.tool)).description(a.agent).kind(ToastKind::Error),
            };
            self.toasts.update(cx, |s, cx| {
                s.push(toast, cx);
            });
        }
        cx.notify();
    }

    fn chat_page(&self, t: &Theme) -> impl IntoElement {
        div()
            .w_full()
            .max_w(px(760.))
            .flex()
            .flex_col()
            .gap(px(18.))
            .p(px(24.))
            .rounded(px(t.radius * 1.6))
            .border_1()
            .border_color(t.border)
            .bg(if t.dark { mix(t.background, t.card, 0.35) } else { t.background })
            .children(self.chat.iter().enumerate().map(|(i, m)| {
                ChatMessage::new(("chat", i), m.role, m.text.clone())
                    .name("glow-1")
                    .timestamp(if m.streaming { "now" } else { "2:14 PM" })
                    .streaming(m.streaming)
            }))
            .child(div().pt(px(6.)).child(self.prompt.clone()))
    }

    fn overview(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(
                Self::stage(
                    t,
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(14.))
                        .child(ShinyText::new("ov-shiny", "Premium, animated UI for Rust").size(40.0))
                        .child(GradientText::new("ov-grad", "now on GPUI").size(30.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .pt(px(8.))
                                .child(ShimmerButton::new("ov-shimmer", "Get started  \u{2192}").on_click(cx.listener(|this, _, _, cx| this.push_toast(cx))))
                                .child(MagneticButton::new("ov-magnet", "Magnetic").reach(16.0)),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(18.))
                    .child(beam_card(t, "ov-beam", "Border Beam", "Light travels the border.").flex_1())
                    .child(SpotlightCard::new("ov-spot").flex_1().p(px(22.)).child(card_body(t, "Spotlight", "Follows your pointer.")))
                    .child(TiltCard::new("ov-tilt").flex_1().child(div().p(px(22.)).child(card_body(t, "Tilt", "2.5D parallax + glare.")))),
            )
            .child(stat_row(t, self.replay))
            .child(
                div()
                    .flex()
                    .gap(px(18.))
                    .items_start()
                    .child(
                        div().flex_1().flex().flex_col().gap(px(14.)).child(
                            AnimatedTabs::new("ov-tabs", ["Overview", "Analytics", "Reports"]).selected(self.tabs[0]).on_change(self.tab_setter(0, cx)),
                        )
                        .child(
                            ApprovalCard::new("ov-approval", "deploy-bot", "shell", "kubectl rollout restart deploy/api")
                                .risk(Risk::High)
                                .waiting(42.0)
                                .reason("Config map change needs a restart."),
                        ),
                    )
                    .child(div().w(px(380.)).child(AgentSessionList::new("ov-sessions", demo_sessions().into_iter().take(4).collect()).selected(self.session))),
            )
            .child(Marquee::new("ov-mq", ["ShinyText", "GradientText", "CountUp", "ShimmerButton", "MagneticButton", "BorderBeam", "SpotlightCard", "TiltCard", "Marquee", "AnimatedTabs", "Toasts", "ChatMessage", "PromptInput", "ApprovalCard", "AgentSessionList", "StatTile"]))
    }
}

fn card_body(t: &Theme, title: &'static str, body: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).text_color(t.foreground).child(title))
        .child(div().text_size(px(13.)).line_height(px(20.)).text_color(t.muted_foreground).child(body))
}

fn beam_card(t: &Theme, id: &'static str, title: &'static str, body: &'static str) -> BorderBeam {
    BorderBeam::new(id).p(px(22.)).child(card_body(t, title, body))
}

fn stat_count(t: &Theme, id: &str, v: f64, pre: &'static str, suf: &'static str, dec: usize, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .child(CountUp::new(id.to_string(), v).prefix(pre).suffix(suf).decimals(dec).size(44.0))
        .child(div().text_size(px(12.5)).text_color(t.muted_foreground).child(label))
}

fn stat_row(t: &Theme, r: u64) -> impl IntoElement {
    div()
        .flex()
        .gap(px(16.))
        .child(StatTile::new(format!("st-rev-{r}"), "Revenue", 45_231.89).prefix("$").decimals(2).delta(20.1).caption("vs last month").spark(vec![12.0, 18.0, 15.0, 22.0, 28.0, 26.0, 34.0, 31.0, 40.0]))
        .child(StatTile::new(format!("st-users-{r}"), "Active agents", 2_350.0).delta(12.4).caption("this week").spark(vec![8.0, 9.0, 12.0, 11.0, 15.0, 19.0, 18.0, 24.0]).color(t.chart[1]))
        .child(StatTile::new(format!("st-lat-{r}"), "p95 latency", 182.0).suffix(" ms").delta(-8.2).invert_delta(true).caption("lower is better").spark(vec![260.0, 240.0, 250.0, 220.0, 210.0, 205.0, 190.0, 182.0]))
}

fn demo_sessions() -> Vec<AgentSession> {
    vec![
        AgentSession::new(1, "Migrate auth to passkeys", RunState::NeedsInput)
            .repo("web-app")
            .device(Device::Laptop, "MacBook")
            .preview("Wants to run: pnpm db:migrate --env prod")
            .activity(2, 40.0),
        AgentSession::new(2, "Fix flaky checkout test", RunState::Running)
            .repo("shop")
            .device(Device::Cloud, "cloud-7")
            .preview("Re-running e2e/checkout.spec.ts (attempt 3/5)")
            .activity(0, 12.0),
        AgentSession::new(3, "Port glowkit to GPUI", RunState::Running)
            .repo("glowkit")
            .device(Device::Terminal, "devbox")
            .preview("Rendering gallery screenshots under Xvfb")
            .activity(1, 75.0),
        AgentSession::new(4, "Weekly dependency bump", RunState::Done)
            .repo("infra")
            .device(Device::Cloud, "cloud-2")
            .preview("Opened PR #482 \u{00B7} 14 crates updated")
            .activity(0, 3_600.0 * 3.0),
        AgentSession::new(5, "Summarize support inbox", RunState::Failed)
            .repo("ops")
            .device(Device::Phone, "Pixel")
            .preview("Rate limited by the mail API")
            .activity(0, 5_400.0),
        AgentSession::new(6, "Draft release notes", RunState::Paused)
            .repo("glowkit")
            .device(Device::Laptop, "MacBook")
            .preview("Paused by you")
            .activity(0, 86_400.0 * 2.0),
    ]
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme().clone();
        let (_, title, cat, desc) = self.page.info();
        let content = self.page_content(cx);
        div()
            .relative()
            .size_full()
            .flex()
            .bg(t.background)
            .text_color(t.foreground)
            .font_family(t.font_family)
            .child(self.sidebar(cx))
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .max_w(px(1040.))
                            .px(px(40.))
                            .py(px(32.))
                            .flex()
                            .flex_col()
                            .gap(px(22.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(div().text_size(px(12.)).text_color(t.accent).font_weight(FontWeight::MEDIUM).child(cat))
                                    .child(div().text_size(px(28.)).font_weight(FontWeight::BOLD).child(title))
                                    .child(div().text_size(px(14.)).text_color(t.muted_foreground).child(desc)),
                            )
                            .child(content),
                    ),
            )
            .child(self.toasts.clone())
    }
}

fn main() {
    env_logger::init();
    let mut page = Page::Overview;
    let mut theme = Theme::dark();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--page" if i + 1 < args.len() => {
                page = Page::from_slug(&args[i + 1]).unwrap_or_else(|| {
                    let all: Vec<_> = Page::ALL.iter().map(|p| p.info().0).collect();
                    panic!("unknown page {:?}; one of {}", args[i + 1], all.join(", "))
                });
                i += 1;
            }
            "--theme" if i + 1 < args.len() => {
                theme = Theme::by_name(&args[i + 1]).expect("unknown theme (midnight, daylight, ocean, ember, matrix)");
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    Application::new().run(move |cx: &mut App| {
        theme.install(cx);
        let bounds = Bounds::centered(None, size(px(1280.), px(840.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("glowkit for GPUI".into()), ..Default::default() }),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| Gallery::new(page, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}

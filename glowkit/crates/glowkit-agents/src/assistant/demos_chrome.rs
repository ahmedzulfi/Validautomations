use super::common::*;
use super::demos::{column, ghost, mem, set_mem, Director};
use super::*;
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Ui};
use glowkit_ai::{Icon, VoiceState};
use glowkit_charts::{AreaChart, Series};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;
use glowkit_effects::ControlSlider;

const C: Category = Category::Ai;

fn people() -> Vec<MentionItem> {
    vec![
        MentionItem::new("Mara", "teammate", hex("#EC4899")),
        MentionItem::new("Max", "agent", hex("#8B5CF6")),
        MentionItem::new("Aiden", "agent", hex("#0EA5E9")),
        MentionItem::new("Ana", "teammate", hex("#F59E0B")),
        MentionItem::new("pricing.md", "file", hex("#10B981")),
    ]
}

fn commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand::new("review", "Review the current diff", Icon::Eye),
        SlashCommand::new("explain", "Explain the selection", Icon::Lightbulb),
        SlashCommand::new("branch", "Start a new branch", Icon::Code),
        SlashCommand::new("improve", "Suggest improvements", Icon::Sparkle),
        SlashCommand::new("summarize", "Summarize the thread", Icon::File),
    ]
}

/// Scripted typing helper: the text at `e` seconds for a list of (start time, full text) frames.
fn typed(frames: &[(f32, &str, f32)], e: f32) -> String {
    // (start, text, chars per second; 0 = instant)
    let mut cur = String::new();
    for (start, s, cps) in frames {
        if e >= *start {
            if *cps <= 0.0 {
                cur = s.to_string();
            } else {
                // type the suffix beyond the common prefix with the previous frame
                let base_len = cur.chars().count().min(s.chars().count());
                let common = cur.chars().zip(s.chars()).take(base_len).take_while(|(a, b)| a == b).count();
                let n = common + ((e - start) * cps) as usize;
                cur = take_chars(s, n).to_string();
            }
        }
    }
    cur
}

fn demo_composer(ui: &mut Ui) {
    let d = Director::new(ui, "cmp_dir", 5.0);
    let mut st: ComposerState = mem(ui, "cmp_state", ComposerState::default);
    let ppl = people();
    let cmds = commands();
    if d.playing {
        let e = d.elapsed % 13.0;
        let frames = [
            (0.4, "/", 0.0),
            (1.2, "/rev", 6.0),
            (2.6, "/review ", 0.0),
            (3.2, "/review the auth changes with @", 14.0),
            (5.6, "/review the auth changes with @ma", 6.0),
            (7.0, "/review the auth changes with @Mara ", 0.0),
            (7.6, "/review the auth changes with @Mara before we merge", 14.0),
            (12.2, "", 0.0),
        ];
        st.text = typed(&frames, e);
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(230.0);
        let out = SmartComposer::new(&mut st)
            .mentions(&ppl)
            .commands(&cmds)
            .model_label("Fable 5")
            .focus_ring(d.playing)
            .id_salt("cmp")
            .show(ui);
        if out.submitted.is_some() {
            st.text.clear();
        }
        ui.add_space(10.0);
        let t = Theme::of(ui);
        ui.label(
            egui::RichText::new("Type / for commands or @ to mention, arrows + Enter to pick")
                .size(11.5)
                .color(t.muted_foreground),
        );
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    set_mem(ui, "cmp_state", st);
}

fn demo_attachments(ui: &mut Ui) {
    let d = Director::new(ui, "att_dir", 5.0);
    let mut st: ComposerState = mem(ui, "att_state", ComposerState::default);
    let mut drag = false;
    let mut cursor: Option<(Pos2, bool)> = None;
    let crect: Rect = mem(ui, "att_rect", || Rect::NOTHING);
    if d.playing {
        let e = d.elapsed % 12.0;
        let reset_at = 0.2;
        if e < reset_at + 0.05 && !st.files.is_empty() {
            st = ComposerState::default();
        }
        if e > 0.8 && e < 2.6 {
            drag = true;
            let k = ((e - 0.8) / 1.2).clamp(0.0, 1.0);
            cursor = Some((crect.center().lerp(crect.center() - vec2(160.0, 140.0), 1.0 - k), false));
        }
        if e > 2.6 && st.files.is_empty() {
            st.files.push(StagedFile::simulated("screenshot.png", "128 KB"));
            st.files.push(StagedFile::simulated("trace.log", "38 KB"));
        }
        if e > 4.0 && st.chips.is_empty() {
            st.chips.push(ContextChip::new("Selection · auth.rs L12–40", Icon::Code));
            st.chips.push(ContextChip::new("Tab · Pricing page", Icon::Globe));
        }
        if e > 5.0 {
            st.text = typed(&[(5.0, "Why does login fail after the token refresh?", 18.0)], e);
        }
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(110.0);
        let out = SmartComposer::new(&mut st)
            .model_label("Fable 5")
            .drag_preview(drag)
            .placeholder("Ask anything, or drop files here")
            .id_salt("att")
            .show(ui);
        set_mem(ui, "att_rect", out.rect);
        if out.attach_clicked {
            st.files.push(StagedFile::simulated(format!("notes-{}.md", st.files.len() + 1), "4 KB"));
        }
        ui.add_space(10.0);
        let t = Theme::of(ui);
        ui.label(
            egui::RichText::new("Drop real files onto the window, or press + to stage one")
                .size(11.5)
                .color(t.muted_foreground),
        );
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    match cursor {
        Some((p, pr)) => {
            ghost(ui, "att", p, true, pr);
            // a dragged file card follows the cursor
            let t = Theme::of(ui);
            let painter =
                ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("asst_att_drag")));
            let r = Rect::from_min_size(p + vec2(14.0, 14.0), vec2(120.0, 34.0));
            painter.rect_filled(r, cr(9.0), surface(&t));
            painter.rect_stroke(r, cr(9.0), hairline(&t), egui::StrokeKind::Inside);
            icon(&painter, Icon::Image, pos2(r.min.x + 16.0, r.center().y), 13.0, t.muted_foreground);
            text(&painter, pos2(r.min.x + 30.0, r.center().y), Align2::LEFT_CENTER, "2 files", 12.0, t.foreground);
        }
        None => {
            ghost(ui, "att", pos2(0.0, 0.0), false, false);
        }
    }
    set_mem(ui, "att_state", st);
}

fn thread_entries() -> Vec<ThreadEntry> {
    vec![
        ThreadEntry::new("Q3 board deck outline", "Today").pinned(),
        ThreadEntry::new("Fix flaky checkout test", "Today"),
        ThreadEntry::new("Deploy checklist for v2.4", "Today"),
        ThreadEntry::new("Pricing page copy review", "Yesterday"),
        ThreadEntry::new("Onboarding email sequence", "Yesterday"),
        ThreadEntry::new("Postgres 17 upgrade plan", "Previous 7 days"),
        ThreadEntry::new("Rate limit headers", "Previous 7 days"),
        ThreadEntry::new("Hiring loop rubric", "Older").archived(),
        ThreadEntry::new("Old deploy notes", "Older").archived(),
    ]
}

fn demo_thread_list(ui: &mut Ui) {
    let d = Director::new(ui, "tl_dir", 5.0);
    let mut st: ThreadListState =
        mem(ui, "tl_state", || ThreadListState { entries: thread_entries(), selected: Some(1), ..Default::default() });
    let rows: Vec<(usize, Rect)> = mem(ui, "tl_rows", Vec::new);
    let mut cursor = None;
    if d.playing {
        let e = d.elapsed % 16.0;
        if e < 0.1 {
            st = ThreadListState { entries: thread_entries(), selected: Some(1), ..Default::default() };
        }
        // 1) pin "Deploy checklist", 2) rename it, 3) search, 4) open archived
        let find = |i: usize| rows.iter().find(|r| r.0 == i).map(|r| r.1);
        if (1.0..2.6).contains(&e) {
            if let Some(r) = find(2) {
                cursor = Some((pos2(r.max.x - 62.0, r.center().y), e > 2.3));
            }
            if e > 2.45 && !st.entries[2].pinned {
                st.entries[2].pinned = true;
            }
        }
        if (3.2..6.4).contains(&e) {
            if let Some(r) = find(2) {
                cursor = Some((pos2(r.max.x - 38.0, r.center().y), e > 3.9 && e < 4.1));
            }
            if e > 4.0 && e < 6.0 {
                let s = typed(&[(4.1, "Deploy checklist", 0.0), (4.4, "Deploy checklist · v2.4.1", 12.0)], e);
                st.renaming = Some((2, s));
            }
            if e >= 6.0 {
                if let Some((i, s)) = st.renaming.take() {
                    st.entries[i].title = s;
                }
            }
        }
        if (7.0..11.0).contains(&e) {
            st.search = typed(&[(7.2, "deploy", 8.0)], e);
        } else if e >= 11.0 {
            st.search.clear();
        }
        if e > 11.8 && e < 15.5 {
            st.show_archived = true;
        } else if e >= 15.5 {
            st.show_archived = false;
        }
    }
    column(ui, 700.0, |ui| {
        let top = ui.cursor().min;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            let out = ThreadListSidebar::new(&mut st).height(420.0).width(270.0).id_salt("tl").show(ui);
            set_mem(ui, "tl_rows", out.rows.clone());
            if out.new_thread {
                st.entries.insert(0, ThreadEntry::new("New chat", "Today"));
                st.selected = Some(0);
            }
            // conversation pane
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 420.0), Sense::hover());
            let t = Theme::of(ui);
            let p = ui.painter();
            p.rect_filled(r, cr(16.0), surface(&t));
            p.rect_stroke(r, cr(16.0), hairline(&t), egui::StrokeKind::Inside);
            let title = st.selected.and_then(|i| st.entries.get(i)).map(|e| e.title.clone()).unwrap_or_default();
            text_bold(p, pos2(r.min.x + 20.0, r.min.y + 26.0), Align2::LEFT_CENTER, &title, 14.0, t.foreground);
            p.line_segment([pos2(r.min.x, r.min.y + 50.0), pos2(r.max.x, r.min.y + 50.0)], hairline(&t));
            for (i, wv) in [0.7f32, 0.9, 0.55, 0.8].iter().enumerate() {
                let y = r.min.y + 80.0 + i as f32 * 26.0;
                p.rect_filled(
                    Rect::from_min_size(pos2(r.min.x + 20.0, y), vec2((r.width() - 40.0) * wv, 10.0)),
                    cr(5.0),
                    well(&t),
                );
            }
            let b = Rect::from_min_size(pos2(r.max.x - 200.0, r.min.y + 200.0), vec2(180.0, 36.0));
            p.rect_filled(b, cr(16.0), well(&t));
            text(p, pos2(r.min.x + 20.0, r.max.y - 30.0), Align2::LEFT_CENTER, "Message…", 13.0, t.muted_foreground);
        });
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(700.0, 0.0)));
    });
    if let Some((p, pr)) = cursor {
        ghost(ui, "tl", p, true, pr);
    } else {
        ghost(ui, "tl", pos2(0.0, 0.0), false, false);
    }
    set_mem(ui, "tl_state", st);
}

fn demo_welcome(ui: &mut Ui) {
    let d = Director::new(ui, "wel_dir", 5.0);
    let mut st: ComposerState = mem(ui, "wel_state", ComposerState::default);
    let cycle: u64 = mem(ui, "wel_cycle", || 0);
    let sugg = [
        WelcomeSuggestion::new("Summarize a document", "Drop a PDF and get the gist", Icon::File),
        WelcomeSuggestion::new("Write a SQL query", "Top customers by revenue", Icon::Code),
        WelcomeSuggestion::new("Plan a trip", "Three days in Lisbon in May", Icon::Globe),
        WelcomeSuggestion::new("Brainstorm names", "For a calm productivity app", Icon::Lightbulb),
    ];
    let mut hl = None;
    if d.playing {
        let e = d.elapsed % 11.0;
        if e > 2.5 && e < 9.0 {
            hl = Some(((e - 2.5) / 1.4) as usize % 4);
        }
        if (9.0..9.1).contains(&e) && st.text.is_empty() {
            st.text = "Plan a trip: three days in Lisbon in May".into();
        }
        if e > 10.8 {
            st.text.clear();
            set_mem(ui, "wel_cycle", cycle + 1);
        }
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(6.0);
        if let Some(i) = WelcomeScreen::new("How can I help you today?")
            .subtitle("Ask anything, or start from one of these")
            .suggestions(&sugg)
            .highlight(hl)
            .id_salt(("wel", cycle))
            .show(ui, |ui| {
                SmartComposer::new(&mut st).model_label("Fable 5").id_salt("wel_c").show(ui);
            })
        {
            st.text = format!("{}: {}", sugg[i].title, sugg[i].detail);
        }
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    set_mem(ui, "wel_state", st);
}

/// A painted analytics dashboard used as the host app behind copilot panels.
fn app_mock(ui: &mut Ui) {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let p = ui.painter();
    text_bold(p, pos2(r.min.x, r.min.y + 12.0), Align2::LEFT_CENTER, "Acme Analytics", 16.0, t.foreground);
    text(
        p,
        pos2(r.min.x, r.min.y + 32.0),
        Align2::LEFT_CENTER,
        "Revenue overview · last 8 weeks",
        12.0,
        t.muted_foreground,
    );
    let (r, _) = ui.allocate_exact_size(vec2(w, 74.0), Sense::hover());
    let n = if w > 420.0 { 3 } else { 2 };
    let gap = 10.0;
    let cw = (w - gap * (n - 1) as f32) / n as f32;
    let stats = [("Revenue", "$48.2k", "+12%"), ("Active users", "3,912", "+4%"), ("Churn", "2.1%", "-0.3%")];
    for (i, (a, b, c)) in stats.iter().enumerate().take(n) {
        let cr_ = Rect::from_min_size(pos2(r.min.x + i as f32 * (cw + gap), r.min.y), vec2(cw, 70.0));
        let p = ui.painter();
        p.rect_filled(cr_, cr(12.0), surface(&t));
        p.rect_stroke(cr_, cr(12.0), hairline(&t), egui::StrokeKind::Inside);
        text(p, pos2(cr_.min.x + 14.0, cr_.min.y + 18.0), Align2::LEFT_CENTER, a, 11.5, t.muted_foreground);
        text_bold(p, pos2(cr_.min.x + 14.0, cr_.min.y + 44.0), Align2::LEFT_CENTER, b, 19.0, t.foreground);
        text(p, pos2(cr_.max.x - 12.0, cr_.min.y + 18.0), Align2::RIGHT_CENTER, c, 11.0, t.success);
    }
    ui.add_space(6.0);
    let s = [Series::new("Revenue", vec![31.0, 34.0, 33.0, 38.0, 41.0, 40.0, 45.0, 48.0])];
    let (cr2, _) = ui.allocate_exact_size(vec2(w, 200.0), Sense::hover());
    ui.painter().rect_filled(cr2, cr(12.0), surface(&t));
    ui.painter().rect_stroke(cr2, cr(12.0), hairline(&t), egui::StrokeKind::Inside);
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(cr2.shrink(12.0)));
    c.add(
        AreaChart::new(&s)
            .labels(&["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"])
            .height(176.0)
            .width(cr2.width() - 24.0)
            .legend(false)
            .id_salt("app_mock_chart"),
    );
}

const COPILOT_REPLY: &str = "Revenue is up **12%** over 8 weeks. The jump in **W4** lines up with the annual-plan discount; W6 dipped slightly as the promo ended.";

fn mini_chat(ui: &mut Ui, key: &'static str, model: &mut ThreadModel, st: &mut ComposerState) {
    let h = ui.available_height() - 104.0;
    let out = AssistantThread::new(model).height(h.max(120.0)).text_size(13.0).id_salt((key, "thread")).show(ui);
    for e in out.events {
        if let ThreadEvent::Regenerate(turn) = e {
            model.regenerate_with(turn, COPILOT_REPLY);
        }
    }
    let o = SmartComposer::new(st).placeholder("Ask about this page…").id_salt((key, "composer")).show(ui);
    if let Some(s) = o.submitted {
        model.ask(&s);
        model.stream_reply(COPILOT_REPLY);
    }
}

fn demo_copilot_sidebar(ui: &mut Ui) {
    let d = Director::new(ui, "cps_dir", 6.0);
    let mut open: bool = mem(ui, "cps_open", || false);
    let mut model: ThreadModel = mem(ui, "cps_model", ThreadModel::new);
    let mut st: ComposerState = mem(ui, "cps_st", ComposerState::default);
    let mut push: f32 = mem(ui, "cps_push", || 1.0);
    if d.playing {
        let e = d.elapsed % 14.0;
        let want = (1.2..12.5).contains(&e);
        if want != open {
            open = want;
            if open {
                model = ThreadModel::new();
            }
        }
        if e > 2.4 && model.turns.is_empty() && open {
            model.ask("Why did revenue jump in W4?");
            model.stream_reply(COPILOT_REPLY);
        }
    }
    column(ui, 860.0, |ui| {
        let top = ui.cursor().min;
        let _ = CopilotSidebar::new(&mut open)
            .height(400.0)
            .panel_width(330.0)
            .push_content(push > 0.5)
            .id_salt("cps")
            .show(ui, app_mock, |ui| mini_chat(ui, "cps", &mut model, &mut st));
        ui.add_space(10.0);
        ui.add(ControlSlider::new("Push content (0 = overlay)", &mut push, 0.0..=1.0).width(280.0));
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(860.0, 0.0)));
    });
    set_mem(ui, "cps_open", open);
    set_mem(ui, "cps_model", model);
    set_mem(ui, "cps_st", st);
    set_mem(ui, "cps_push", push);
}

fn demo_copilot_popup(ui: &mut Ui) {
    let d = Director::new(ui, "cpp_dir", 6.0);
    let mut open: bool = mem(ui, "cpp_open", || false);
    let mut model: ThreadModel = mem(ui, "cpp_model", ThreadModel::new);
    let mut st: ComposerState = mem(ui, "cpp_st", ComposerState::default);
    let launcher: Rect = mem(ui, "cpp_launch", || Rect::NOTHING);
    let mut cursor = None;
    if d.playing {
        let e = d.elapsed % 13.0;
        if launcher.is_positive() && ((0.3..1.6).contains(&e) || (11.0..12.3).contains(&e)) {
            cursor = Some((launcher.center(), (1.3..1.6).contains(&e) || (12.0..12.3).contains(&e)));
        }
        let want = (1.5..12.2).contains(&e);
        if want != open {
            open = want;
            if open {
                model =
                    ThreadModel::new().with_exchange("Hi!", "Hi there! I can answer questions about this dashboard.");
            }
        }
        if e > 3.0 && model.turns.len() == 1 && open {
            model.ask("Summarize this week in one line");
            model.stream_reply("Revenue hit **$48.2k** (+12%) with churn steady at **2.1%**: a strong, quiet week.");
        }
    }
    column(ui, 860.0, |ui| {
        let top = ui.cursor().min;
        let r = CopilotPopup::new(&mut open)
            .height(440.0)
            .id_salt("cpp")
            .show(ui, app_mock, |ui| mini_chat(ui, "cpp", &mut model, &mut st));
        set_mem(ui, "cpp_launch", r.rect);
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(860.0, 0.0)));
    });
    match cursor {
        Some((p, pr)) => ghost(ui, "cpp", p + vec2(4.0, 4.0), true, pr),
        None => ghost(ui, "cpp", pos2(0.0, 0.0), false, false),
    };
    set_mem(ui, "cpp_open", open);
    set_mem(ui, "cpp_model", model);
    set_mem(ui, "cpp_st", st);
}

fn completion(text: &str) -> Option<&'static str> {
    let table: [(&str, &str); 4] = [
        (
            "Hi Jonas,\n\nThanks for the quick turnaround on ",
            "the pricing page. The new layout reads much better, and the FAQ answers exactly what sales keeps hearing.",
        ),
        (
            "the FAQ answers exactly what sales keeps hearing.\n\nCould you ",
            "also add the annual discount to the comparison table before Friday's launch?",
        ),
        ("Friday's launch?\n\n", "Best,\nMara"),
        ("Hi Jonas,\n\n", "Thanks for the quick turnaround on the pricing page."),
    ];
    table.iter().find(|(k, _)| text.ends_with(k)).map(|(_, v)| *v)
}

fn demo_inline(ui: &mut Ui) {
    let d = Director::new(ui, "inl_dir", 5.0);
    let mut text_s: String = mem(ui, "inl_text", String::new);
    let mut delay: f32 = mem(ui, "inl_delay", || 0.5);
    let mut accept = false;
    if d.playing {
        let e = d.elapsed % 16.0;
        let frames = [(0.2, "", 0.0), (0.4, "Hi Jonas,\n\nThanks for the quick turnaround on ", 22.0)];
        if e < 3.0 {
            text_s = typed(&frames, e);
        }
        if (4.2..4.3).contains(&e) || (8.4..8.5).contains(&e) || (11.4..11.5).contains(&e) {
            accept = true;
        }
        if (5.4..7.4).contains(&e) && text_s.ends_with("hearing.") {
            // the user types on, then pauses
        }
        if (5.4..7.6).contains(&e) {
            let base = "Hi Jonas,\n\nThanks for the quick turnaround on the pricing page. The new layout reads much better, and the FAQ answers exactly what sales keeps hearing.";
            if text_s.starts_with(base) || text_s == base {
                text_s = typed(&[(5.4, base, 0.0), (5.5, &format!("{base}\n\nCould you "), 10.0)], e);
            }
        }
        if (9.4..10.6).contains(&e) && text_s.ends_with("launch?") {
            text_s.push_str("\n\n");
        }
        if e > 15.6 {
            text_s.clear();
        }
    }
    let sugg = completion(&text_s);
    column(ui, 600.0, |ui| {
        let top = ui.cursor().min;
        let t = Theme::of(ui);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(600.0, 26.0), Sense::hover());
            let p = ui.painter();
            text(
                p,
                pos2(r.min.x + 2.0, r.center().y),
                Align2::LEFT_CENTER,
                "To:  jonas@acme.studio",
                12.5,
                t.muted_foreground,
            );
            chip(p, &t, ui, pos2(r.max.x - 120.0, r.min.y + 2.0), "Autocomplete on", 11.0, Some(t.accent));
        });
        InlineSuggestion::new(&mut text_s, sugg).delay(delay).rows(7).accept_now(accept).id_salt("inl").show(ui);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(300.0, 20.0), Sense::hover());
            let p = ui.painter();
            let k = kbd(p, &t, ui, pos2(r.min.x + 30.0, r.center().y), "Tab");
            text(p, pos2(k.max.x + 6.0, r.center().y), Align2::LEFT_CENTER, "accept", 11.5, t.muted_foreground);
            let k2 = kbd(p, &t, ui, pos2(k.max.x + 80.0, r.center().y), "Esc");
            text(p, pos2(k2.max.x + 6.0, r.center().y), Align2::LEFT_CENTER, "dismiss", 11.5, t.muted_foreground);
        });
        ui.add(ControlSlider::new("Suggestion delay", &mut delay, 0.0..=2.0).decimals(1).suffix("s").width(260.0));
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(600.0, 0.0)));
    });
    set_mem(ui, "inl_text", text_s);
    set_mem(ui, "inl_delay", delay);
}

const PARA: &str = "Our onboarding funnel loses most people at the workspace step: 38% of new signups never invite a teammate, and those accounts churn three times faster. The fix we keep coming back to is letting people start solo and nudging invites only after their first finished project.";

fn demo_selection(ui: &mut Ui) {
    let d = Director::new(ui, "sel_dir", 5.0);
    let actions = [
        SelectionAction::new("Explain", Icon::Lightbulb, "Accounts that never add a second person have no **social anchor**: nobody else depends on the workspace, so it is easy to abandon. Three times the churn means this one step drives most early losses."),
        SelectionAction::new("Improve", Icon::Pencil, "**38% of new signups never invite a teammate, and those accounts churn 3× faster.**"),
        SelectionAction::new("Translate", Icon::Globe, "38 % des nouveaux inscrits n'invitent jamais de coéquipier, et ces comptes résilient trois fois plus vite."),
    ];
    let cpos: Vec<Pos2> = mem(ui, "sel_cpos", Vec::new);
    let btns: Vec<Rect> = mem(ui, "sel_btns", Vec::new);
    let (s0, s1) = (PARA.find("38%").unwrap_or(0), PARA.find(" faster").map(|i| i + 7).unwrap_or(10));
    let (c0, c1) = (PARA[..s0].chars().count(), PARA[..s1].chars().count());
    let mut select = None;
    let mut trigger = None;
    let mut cursor = None;
    if d.playing {
        let e = d.elapsed % 13.0;
        if e < 0.2 {
            select = Some(None);
        }
        if (0.8..2.6).contains(&e) && cpos.len() > c1 {
            let k = ((e - 1.2) / 1.2).clamp(0.0, 1.0);
            let n = c0 + ((c1 - c0) as f32 * k) as usize;
            cursor = Some((cpos[n], e > 1.2));
            if k > 0.02 {
                select = Some(Some((c0, n.max(c0 + 1))));
            }
        }
        if (2.6..4.4).contains(&e) && btns.len() > 1 {
            cursor = Some((btns[1].center(), e > 4.0));
        }
        if e > 4.2 && e < 12.6 {
            trigger = Some(0);
        }
        if e >= 12.6 {
            select = Some(None);
        }
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        let t = Theme::of(ui);
        ui.label(egui::RichText::new("Q3 onboarding review").size(18.0).color(t.foreground));
        let mut b = SelectionAskBar::new(PARA, &actions).text_size(15.5).id_salt("sel");
        if let Some(s) = select {
            b = b.select(s);
        }
        let out = b.trigger(trigger).show(ui);
        set_mem(ui, "sel_cpos", out.char_pos.clone());
        set_mem(ui, "sel_btns", out.buttons.clone());
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    match cursor {
        Some((p, pr)) => ghost(ui, "sel", p, true, pr),
        None => ghost(ui, "sel", pos2(0.0, 0.0), false, false),
    };
}

fn demo_modes(ui: &mut Ui) {
    let d = Director::new(ui, "mode_dir", 5.0);
    let mut sel: usize = mem(ui, "mode_sel", || 1);
    let mut st: ComposerState = mem(ui, "mode_st", ComposerState::default);
    let t = Theme::of(ui);
    let modes = [
        AgentMode::new("Ask", Icon::Search, "Answers questions, never touches your files", t.info),
        AgentMode::new("Agent", Icon::Bolt, "Edits files and runs commands, asks before risky steps", t.accent),
        AgentMode::new("Plan", Icon::Lightbulb, "Researches and writes a plan for you to approve", t.success),
    ];
    if d.playing {
        sel = [1usize, 2, 0, 1][((d.elapsed / 2.6) as usize) % 4];
    }
    column(ui, 600.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.add(AgentModeSwitcher::new(&modes, &mut sel).id_salt("modes"));
        });
        ui.add_space(18.0);
        let ph =
            ["Ask a question about the codebase…", "Tell the agent what to build…", "Describe what you want planned…"]
                [sel];
        SmartComposer::new(&mut st).placeholder(ph).model_label(modes[sel].name.as_str()).id_salt("mode_c").show(ui);
        ui.add_space(18.0);
        ui.vertical_centered(|ui| {
            ui.add(AgentModeSwitcher::new(&modes, &mut sel).describe(false).segment_width(76.0).id_salt("modes_small"));
        });
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(600.0, 0.0)));
    });
    set_mem(ui, "mode_sel", sel);
    set_mem(ui, "mode_st", st);
}

fn models() -> Vec<ModelSpec> {
    vec![
        ModelSpec::new("Fable 5", "Fable", hex("#D97757"))
            .description("Most capable, for deep work and agents")
            .speed_cost(2, 3)
            .context("1M")
            .tags(&["Reasoning", "Vision"]),
        ModelSpec::new("Quill 4.5", "Quill", hex("#6366F1"))
            .description("Balanced speed and intelligence")
            .speed_cost(2, 2)
            .context("400K")
            .tags(&["Vision"]),
        ModelSpec::new("Swift 3 mini", "Swift", hex("#0EA5E9"))
            .description("Fast and cheap for everyday chat")
            .speed_cost(3, 1)
            .context("200K"),
        ModelSpec::new("Atlas Coder", "Atlas", hex("#10B981"))
            .description("Tuned for code edits and refactors")
            .speed_cost(2, 2)
            .context("256K")
            .tags(&["Reasoning"]),
    ]
}

fn demo_model_picker(ui: &mut Ui) {
    let d = Director::new(ui, "mp_dir", 5.0);
    let mut sel: usize = mem(ui, "mp_sel", || 0);
    let ms = models();
    let mut open = None;
    let mut hover = None;
    if d.playing {
        let e = d.elapsed % 10.0;
        open = Some((1.0..7.6).contains(&e));
        if (1.6..7.6).contains(&e) {
            hover = Some(((e - 1.6) / 1.2) as usize % 4);
        }
        if (7.5..7.6).contains(&e) {
            sel = (sel + 1) % 4;
        }
    }
    column(ui, 600.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let mut p = ModelPickerRich::new(&ms, &mut sel).id_salt("mp");
            if let Some(o) = open {
                p = p.open(o);
            }
            if hover.is_some() {
                p = p.hover_row(hover);
            }
            p.show(ui);
        });
        ui.add_space(320.0);
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(600.0, 0.0)));
    });
    set_mem(ui, "mp_sel", sel);
}

fn demo_usage(ui: &mut Ui) {
    let d = Director::new(ui, "use_dir", 5.0);
    let mut used: f32 = mem(ui, "use_used", || 30.0);
    if d.playing {
        let e = d.elapsed % 12.0;
        used = if e < 2.5 {
            30.0
        } else if e < 6.0 {
            43.0
        } else {
            50.0
        };
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        ui.add_space(20.0);
        let out = UsageLimitBanner::new(used as u32, 50).resets_in("3h 12m").plan("Pro").id_salt("use").show(ui);
        let _ = out;
        ui.add_space(12.0);
        let mut st = ComposerState::default();
        let at_limit = used >= 50.0;
        ui.scope(|ui| {
            if at_limit {
                ui.multiply_opacity(0.55);
            }
            SmartComposer::new(&mut st)
                .placeholder(if at_limit {
                    "You can send messages again in 3h 12m"
                } else {
                    "Message the assistant…"
                })
                .id_salt("use_c")
                .show(ui);
        });
        ui.add_space(14.0);
        ui.add(ControlSlider::new("Messages used", &mut used, 0.0..=50.0).width(280.0));
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    set_mem(ui, "use_used", used);
}

const CITED: &str = "Rust prevents data races at compile time through its ownership and borrowing rules [1]. The `Send` and `Sync` marker traits tell the compiler which types can cross thread boundaries [2], and most concurrency bugs become type errors instead of runtime crashes [3]. Async runtimes build on the same guarantees [4].";

fn demo_citations(ui: &mut Ui) {
    let d = Director::new(ui, "cit_dir", 5.0);
    let mut open: bool = mem(ui, "cit_open", || false);
    let hl_prev: Option<usize> = mem(ui, "cit_hl", || None);
    let src = [
        Citation::new("Fearless Concurrency", "doc.rust-lang.org", hex("#CE422B")),
        Citation::new("Send and Sync, explained", "nomicon.dev", hex("#6366F1")),
        Citation::new("Concurrency bugs in practice", "research.acme.org", hex("#0EA5E9")),
        Citation::new("Async Rust: the guarantees", "tokio.rs", hex("#10B981")),
    ];
    let mut reveal = None;
    let mut hl_auto = None;
    if d.playing {
        let e = d.elapsed % 14.0;
        reveal = Some(((e - 0.3).max(0.0) * 90.0) as usize);
        open = (5.0..13.5).contains(&e);
        if e > 6.5 && e < 13.0 {
            hl_auto = Some(((e - 6.5) / 1.4) as usize % 4);
        }
    }
    column(ui, 620.0, |ui| {
        let top = ui.cursor().min;
        let t = Theme::of(ui);
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
            bot_avatar(ui.painter(), &t, r.center(), 13.0);
            ui.vertical(|ui| {
                let mut ct = CitedText::new(CITED);
                if let Some(n) = reveal {
                    ct = ct.reveal(n);
                }
                let h1 = ct.show(ui, hl_auto.or(hl_prev));
                ui.add_space(8.0);
                let h2 = CitationsFooter::new(&src, &mut open).id_salt("cit").show(ui, hl_auto.or(h1));
                set_mem(ui, "cit_hl", h2);
            });
        });
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(620.0, 0.0)));
    });
    set_mem(ui, "cit_open", open);
}

const VOICE_USER: &str = "What's on my calendar this afternoon?";
const VOICE_BOT: &str =
    "You have a design review at two, then a quiet block until four thirty when Mara joins for the launch sync.";

fn demo_voice(ui: &mut Ui) {
    let d = Director::new(ui, "vm_dir", 5.0);
    let mut open: bool = mem(ui, "vm_open", || false);
    let mut muted: bool = mem(ui, "vm_muted", || false);
    let time = glowkit_core::time(ui);
    let e = if d.playing { d.elapsed % 16.0 } else { 6.0 };
    if d.playing {
        open = (1.0..15.0).contains(&e);
    }
    let (state, caption): (VoiceState, String) = if e < 2.2 {
        (VoiceState::Idle, String::new())
    } else if e < 5.4 {
        let words: Vec<&str> = VOICE_USER.split(' ').collect();
        let n = (((e - 2.4) * 3.0).max(0.0) as usize).min(words.len());
        (VoiceState::Listening, words[..n].join(" "))
    } else if e < 6.6 {
        (VoiceState::Thinking, VOICE_USER.to_owned())
    } else {
        let words: Vec<&str> = VOICE_BOT.split(' ').collect();
        let n = (((e - 6.6) * 3.2) as usize).min(words.len());
        (VoiceState::Speaking, words[..n].join(" "))
    };
    let level = match state {
        VoiceState::Listening | VoiceState::Speaking => {
            0.35 + 0.3 * (time * 7.0).sin().abs() * (time * 2.3).sin().abs()
        }
        _ => 0.05,
    };
    column(ui, 760.0, |ui| {
        let top = ui.cursor().min;
        let out = VoiceModeOverlay::new(&mut open)
            .state(state)
            .level(level)
            .caption(&caption)
            .muted(muted)
            .height(430.0)
            .id_salt("vm")
            .show(ui, |ui| {
                let t = Theme::of(ui);
                ui.label(egui::RichText::new("Assistant").size(15.0).color(t.foreground));
                ui.add_space(8.0);
                ui.add(glowkit_ai::ChatMessage::new(
                    glowkit_ai::Role::Assistant,
                    "Hi! Tap the mic to talk instead of typing.",
                ));
            });
        if out.mute_clicked {
            muted = !muted;
        }
        // mic button in the host composer (where the overlay grows from)
        let t = Theme::of(ui);
        let c = out.origin;
        let r = Rect::from_center_size(c, egui::Vec2::splat(40.0));
        let b = ui.interact(r, egui::Id::unique("asst_vm_mic"), Sense::click());
        if !open {
            let p = ui.painter();
            let bar = Rect::from_min_max(pos2(r.min.x - 520.0, r.min.y - 6.0), pos2(r.max.x + 30.0, r.max.y + 6.0));
            p.rect_filled(bar, cr(26.0), surface(&t));
            p.rect_stroke(bar, cr(26.0), hairline(&t), egui::StrokeKind::Inside);
            text(p, pos2(bar.min.x + 20.0, bar.center().y), Align2::LEFT_CENTER, "Message…", 13.5, t.muted_foreground);
            p.circle_filled(c, 18.0, t.primary);
            icon(p, Icon::Mic, c, 17.0, t.primary_foreground);
        }
        if b.clicked() {
            open = true;
        }
        d.area(ui, Rect::from_min_max(top, ui.cursor().min + vec2(760.0, 0.0)));
        let _ = with_alpha(Color32::WHITE, 1.0);
    });
    set_mem(ui, "vm_open", open);
    set_mem(ui, "vm_muted", muted);
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "asst-composer",
            name: "Smart Composer",
            category: C,
            description: "Agent composer with @mentions and /slash-command menus that filter as you type, highlighted inline tokens and a waking send button.",
            covers: &["assistant-ui: Slash commands", "assistant-ui: Mentions", "assistant-ui: Composer trigger popover", "nextui (agent): Prompt Composer"],
            code: r##"let people = [MentionItem::new("Mara", "teammate", hex("#EC4899"))];
let cmds = [SlashCommand::new("review", "Review the current diff", Icon::Eye)];
let out = SmartComposer::new(&mut state).mentions(&people).commands(&cmds).model_label("Fable 5").show(ui);
if let Some(msg) = out.submitted { send(msg); }"##,
            show: demo_composer,
        },
        Demo {
            id: "asst-composer-attachments",
            name: "Attachments & Context",
            category: C,
            description: "Drag files onto the composer (a marching dashed drop zone), staged files with upload rings, and removable context chips.",
            covers: &["assistant-ui: Attachments", "assistant-ui: Context", "CopilotKit: Copilot Textarea (context)", "nextui (agent): File Upload"],
            code: r##"state.files.push(StagedFile::new("screenshot.png", "128 KB"));
state.chips.push(ContextChip::new("Selection · auth.rs L12–40", Icon::Code));
SmartComposer::new(&mut state).placeholder("Ask anything, or drop files here").show(ui); // dropped files are staged automatically"##,
            show: demo_attachments,
        },
        Demo {
            id: "asst-thread-list",
            name: "Thread List Sidebar",
            category: C,
            description: "Conversation sidebar with search highlighting, pinned and date groups, inline rename and a collapsible archive.",
            covers: &["assistant-ui: Thread List", "assistant-ui: Thread search", "nextui (agent): Chat History"],
            code: r##"let mut st = ThreadListState { entries: vec![ThreadEntry::new("Q3 deck", "Today").pinned()], ..Default::default() };
let out = ThreadListSidebar::new(&mut st).height(420.0).show(ui);
if let Some(i) = out.opened { open_thread(i); }"##,
            show: demo_thread_list,
        },
        Demo {
            id: "asst-welcome",
            name: "Welcome Suggestions",
            category: C,
            description: "The empty state: a glowing mark, greeting, staggered starter cards that lift on hover, and the composer.",
            covers: &["assistant-ui: Thread Welcome", "assistant-ui: Empty state", "CopilotKit: Suggestions", "nextui (agent): Welcome Screen"],
            code: r##"let s = [WelcomeSuggestion::new("Plan a trip", "Three days in Lisbon", Icon::Globe)];
if let Some(i) = WelcomeScreen::new("How can I help you today?").suggestions(&s).show(ui, |ui| {
    SmartComposer::new(&mut composer).show(ui);
}) { composer.text = s[i].title.clone(); }"##,
            show: demo_welcome,
        },
        Demo {
            id: "asst-copilot-sidebar",
            name: "Copilot Sidebar",
            category: C,
            description: "A docked assistant panel that springs in from the right edge, pushing (or overlaying) the app, with a live chat inside.",
            covers: &["CopilotKit: CopilotSidebar", "assistant-ui: Assistant sidebar"],
            code: r##"CopilotSidebar::new(&mut open).panel_width(330.0).show(ui,
    |ui| my_app(ui),
    |ui| { AssistantThread::new(&mut model).height(260.0).show(ui); },
);"##,
            show: demo_copilot_sidebar,
        },
        Demo {
            id: "asst-copilot-popup",
            name: "Copilot Popup",
            category: C,
            description: "A floating launcher whose sparkle morphs into a cross as a chat window springs out of the corner.",
            covers: &["CopilotKit: CopilotPopup", "assistant-ui: Assistant modal", "assistant-ui: Launcher"],
            code: r##"CopilotPopup::new(&mut open).window_size(egui::vec2(330.0, 380.0)).show(ui,
    |ui| my_app(ui),
    |ui| { AssistantThread::new(&mut model).show(ui); },
);"##,
            show: demo_copilot_popup,
        },
        Demo {
            id: "asst-inline-suggestion",
            name: "Inline Suggestion",
            category: C,
            description: "Ghost-text autocomplete: a muted continuation fades in after a pause with a Tab hint; Tab accepts with a soft flash.",
            covers: &["CopilotKit: CopilotTextarea", "nextui (agent): Autocomplete"],
            code: r##"let ghost = my_model_completion(&text); // Option<&str>
let out = InlineSuggestion::new(&mut text, ghost).delay(0.5).show(ui);
if out.accepted { /* log acceptance */ }"##,
            show: demo_inline,
        },
        Demo {
            id: "asst-selection-toolbar",
            name: "Selection Ask AI",
            category: C,
            description: "Select text to pop a floating Ask AI toolbar; quick actions stream an answer card under the selection.",
            covers: &["assistant-ui: Selection Toolbar", "assistant-ui: Quote", "CopilotKit: Copilot Textarea (ask on selection)"],
            code: r##"let actions = [SelectionAction::new("Explain", Icon::Lightbulb, answer)];
let out = SelectionAskBar::new(paragraph, &actions).show(ui);"##,
            show: demo_selection,
        },
        Demo {
            id: "asst-mode-switcher",
            name: "Agent Mode Switcher",
            category: C,
            description: "Ask / Agent / Plan segmented switch: a tinted pill glides between modes and the description slides along.",
            covers: &["nextui (agent): Mode Selector", "assistant-ui: Composer mode"],
            code: r##"let modes = [AgentMode::new("Ask", Icon::Search, "Answers only", t.info), AgentMode::new("Agent", Icon::Bolt, "Edits files", t.accent)];
ui.add(AgentModeSwitcher::new(&modes, &mut mode));"##,
            show: demo_modes,
        },
        Demo {
            id: "asst-model-picker",
            name: "Model Picker (rich)",
            category: C,
            description: "A model popover with search, provider marks, speed bolts, cost, context-window chips and capability tags.",
            covers: &["assistant-ui: Model selector", "nextui (agent): Model Selector"],
            code: r##"let models = [ModelSpec::new("Swift 3 mini", "Swift", hex("#0EA5E9")).speed_cost(3, 1).context("200K")];
ModelPickerRich::new(&models, &mut selected).show(ui);"##,
            show: demo_model_picker,
        },
        Demo {
            id: "asst-usage-banner",
            name: "Usage Limit Banner",
            category: C,
            description: "A quota banner whose tone morphs from calm to amber to red, with a segmented meter, reset timer and upgrade shine.",
            covers: &["assistant-ui: Quota banner", "nextui (agent): Usage Limit"],
            code: r##"let out = UsageLimitBanner::new(42, 50).resets_in("3h 12m").plan("Pro").show(ui);
if out.upgrade { open_billing(); }"##,
            show: demo_usage,
        },
        Demo {
            id: "asst-citations",
            name: "Citations Footer",
            category: C,
            description: "Answer text with numbered citation pills and a Sources footer that expands into cards; hovering links both.",
            covers: &["assistant-ui: Sources", "assistant-ui: Inline citation", "nextui (agent): Citations"],
            code: r##"let hovered = CitedText::new("Rust prevents data races [1].").show(ui, footer_hover);
let footer_hover = CitationsFooter::new(&sources, &mut open).show(ui, hovered);"##,
            show: demo_citations,
        },
        Demo {
            id: "asst-voice-mode",
            name: "Voice Mode Overlay",
            category: C,
            description: "A voice conversation overlay that grows out of the mic button: living orb, shimmering status, live captions and call controls.",
            covers: &["assistant-ui: Voice conversation", "CopilotKit: Voice (push to talk)", "nextui (agent): Voice Mode"],
            code: r##"VoiceModeOverlay::new(&mut open).state(VoiceState::Speaking).level(level).caption(&caption)
    .show(ui, |ui| my_chat(ui));"##,
            show: demo_voice,
        },
    ]
}

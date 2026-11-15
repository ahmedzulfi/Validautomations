use super::common::*;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// What happened in a [`UsageLimitBanner`] this frame.
pub struct UsageBannerOutput {
    /// Banner response.
    pub response: Response,
    /// The upgrade button was clicked.
    pub upgrade: bool,
    /// The dismiss button was clicked.
    pub dismissed: bool,
}

/// A usage / quota banner for assistants with message limits. It reads calm and neutral while
/// there is room, warms to amber past a threshold and turns red at the limit, with the tone
/// morphing smoothly. A segmented meter fills block by block on a spring, a reset countdown sits
/// under the title, and at the limit the Upgrade button gets a slow shine.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let out = glowkit_agents::UsageLimitBanner::new(42, 50).resets_in("3h 12m").show(ui);
/// if out.upgrade { /* open billing */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct UsageLimitBanner<'a> {
    used: u32,
    limit: u32,
    resets: Option<&'a str>,
    warn_at: f32,
    plan: &'a str,
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> UsageLimitBanner<'a> {
    /// `used` of `limit` messages.
    pub fn new(used: u32, limit: u32) -> Self {
        Self { used, limit: limit.max(1), resets: None, warn_at: 0.8, plan: "Pro", width: None, salt: None }
    }
    /// Time until the quota resets (e.g. "3h 12m").
    pub fn resets_in(mut self, s: &'a str) -> Self {
        self.resets = Some(s);
        self
    }
    /// Fraction where the banner turns amber (default 0.8).
    pub fn warn_at(mut self, f: f32) -> Self {
        self.warn_at = f.clamp(0.1, 1.0);
        self
    }
    /// Plan name offered by the upgrade button.
    pub fn plan(mut self, p: &'a str) -> Self {
        self.plan = p;
        self
    }
    /// Width (default: available, at most 620).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the banner.
    pub fn show(self, ui: &mut Ui) -> UsageBannerOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_usage");
        let w = self.width.unwrap_or_else(|| ui.available_width().min(620.0));
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 74.0), Sense::hover());
        let frac = self.used as f32 / self.limit as f32;
        let f = glowkit_core::anim::spring(ui, id.with("f"), frac.min(1.0), 120.0, 20.0);
        let level = if frac >= 1.0 { 2.0 } else if frac >= self.warn_at { 1.0 } else { 0.0 };
        let lv = glowkit_core::anim::spring(ui, id.with("lv"), level, 160.0, 24.0);
        let tone = if lv <= 1.0 { mix(t.info, t.warning, lv.clamp(0.0, 1.0)) } else { mix(t.warning, t.destructive, (lv - 1.0).clamp(0.0, 1.0)) };
        let time = glowkit_core::time(ui);
        if level >= 2.0 {
            glowkit_core::animate(ui);
        }
        let p = ui.painter();
        let bg = mix(surface(&t), tone, if t.dark { 0.1 } else { 0.06 } * lv.clamp(0.0, 2.0) / 2.0 + 0.02);
        paint::soft_shadow(p, rect, 14.0, 12.0, vec2(0.0, 3.0), shadow(&t, 0.6));
        p.rect_filled(rect, cr(14.0), bg);
        p.rect_stroke(rect, cr(14.0), Stroke::new(1.0, mix(hairline(&t).color, tone, 0.35 * (lv / 2.0 + 0.2))), StrokeKind::Inside);
        let ic = pos2(rect.min.x + 26.0, rect.min.y + 26.0);
        p.circle_filled(ic, 13.0, with_alpha(tone, 0.14));
        icon(p, if level >= 2.0 { Icon::Lock } else if level >= 1.0 { Icon::AlertCircle } else { Icon::Bolt }, ic, 14.0, tone);
        let left = self.limit.saturating_sub(self.used);
        let title = if level >= 2.0 {
            "You've reached your message limit".to_owned()
        } else if level >= 1.0 {
            format!("{left} messages left")
        } else {
            format!("{} of {} messages used", self.used, self.limit)
        };
        text_bold(p, pos2(rect.min.x + 48.0, rect.min.y + 20.0), Align2::LEFT_CENTER, &title, 13.5, t.foreground);
        if let Some(r) = self.resets {
            text(p, pos2(rect.min.x + 48.0, rect.min.y + 38.0), Align2::LEFT_CENTER, &format!("Resets in {r}"), 11.5, t.muted_foreground);
        }
        // segmented meter
        let segs = self.limit.min(25) as usize;
        let meter = Rect::from_min_max(pos2(rect.min.x + 48.0, rect.max.y - 18.0), pos2(rect.max.x - 150.0, rect.max.y - 12.0));
        let gap = 3.0;
        let sw = (meter.width() - gap * (segs - 1) as f32) / segs as f32;
        for s in 0..segs {
            let r = Rect::from_min_size(pos2(meter.min.x + s as f32 * (sw + gap), meter.min.y), vec2(sw, meter.height()));
            p.rect_filled(r, cr(2.0), well(&t));
            let fill = (f * segs as f32 - s as f32).clamp(0.0, 1.0);
            if fill > 0.0 {
                p.rect_filled(Rect::from_min_size(r.min, vec2(sw * fill, r.height())), cr(2.0), tone);
            }
        }
        // buttons
        let label = format!("Upgrade to {}", self.plan);
        let bw = button_w(ui, &label, 32.0, false);
        let br = Rect::from_min_size(pos2(rect.max.x - 44.0 - bw, rect.center().y - 16.0), vec2(bw, 32.0));
        let up = button(ui, id.with("up"), br, &label, Btn::Primary, None).clicked();
        if level >= 2.0 {
            let ph = (time * 0.5).fract() * 1.8 - 0.4;
            let sx = br.min.x + br.width() * ph;
            let sh = Rect::from_min_max(pos2(sx - 16.0, br.min.y), pos2(sx + 16.0, br.max.y)).intersect(br);
            if sh.width() > 1.0 {
                paint::gradient_rect(ui.painter(), sh, 0.0, 0.0, &[with_alpha(Color32::WHITE, 0.0), with_alpha(Color32::WHITE, if t.dark { 0.35 } else { 0.25 }), with_alpha(Color32::WHITE, 0.0)]);
            }
        }
        let xr = Rect::from_center_size(pos2(rect.max.x - 22.0, rect.center().y), Vec2::splat(26.0));
        let dismissed = icon_button(ui, id.with("x"), xr, Icon::X, false).clicked();
        UsageBannerOutput { response: resp, upgrade: up, dismissed }
    }
}

/// A source cited by an assistant answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Citation {
    /// Page title.
    pub title: String,
    /// Domain.
    pub domain: String,
    /// Favicon color.
    pub color: Color32,
}

impl Citation {
    /// A citation.
    pub fn new(title: impl Into<String>, domain: impl Into<String>, color: Color32) -> Self {
        Self { title: title.into(), domain: domain.into(), color }
    }
}

/// Answer text with numbered citation markers: every `[n]` in the text renders as a small pill.
/// Hovering a pill (or highlighting a source from a [`CitationsFooter`]) lights it up. Returns the
/// hovered source (zero-based) so the footer can highlight its card.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let hovered = glowkit_agents::CitedText::new("Rust is memory safe [1] and fast [2].").show(ui, None);
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct CitedText<'a> {
    text: &'a str,
    size: f32,
    reveal: Option<usize>,
}

impl<'a> CitedText<'a> {
    /// Text containing `[n]` markers (1-based).
    pub fn new(text: &'a str) -> Self {
        Self { text, size: 14.5, reveal: None }
    }
    /// Font size (default 14.5).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Only show the first `n` characters (for streaming).
    pub fn reveal(mut self, n: usize) -> Self {
        self.reveal = Some(n);
        self
    }

    /// Show the text; `highlight` lights up one source's markers. Returns the hovered marker.
    pub fn show(self, ui: &mut Ui, highlight: Option<usize>) -> Option<usize> {
        let t = Theme::of(ui);
        let w = ui.available_width();
        let src = match self.reveal {
            Some(n) => take_chars(self.text, n),
            None => self.text,
        };
        // split into runs and markers
        let mut job = LayoutJob::default();
        let base = TextFormat { font_id: FontId::proportional(self.size), color: t.foreground, line_height: Some(self.size * 1.6), ..Default::default() };
        let mut markers: Vec<(usize, usize, usize)> = Vec::new(); // (char start, char end, source)
        let mut chars = 0usize;
        let mut rest = src;
        while let Some(i) = rest.find('[') {
            let (pre, after) = rest.split_at(i);
            if let Some(j) = after.find(']') {
                if let Ok(n) = after[1..j].parse::<usize>() {
                    job.append(pre, 0.0, base.clone());
                    chars += pre.chars().count();
                    let label = format!("{n}");
                    let fmt = TextFormat { font_id: FontId::proportional(self.size * 0.72), color: t.muted_foreground, valign: egui::Align::Center, ..base.clone() };
                    job.append(&format!("\u{2009}{label}\u{2009}"), 2.0, fmt);
                    let len = label.chars().count() + 2;
                    markers.push((chars, chars + len, n.saturating_sub(1)));
                    chars += len;
                    rest = &after[j + 1..];
                    continue;
                }
            }
            job.append(&rest[..i + 1], 0.0, base.clone());
            chars += rest[..i + 1].chars().count();
            rest = &rest[i + 1..];
        }
        job.append(rest, 0.0, base);
        job.wrap.max_width = w;
        let g = ui.painter().layout_job(job);
        let (rect, _) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let mut glyphs: Vec<Rect> = Vec::new();
        for row in &g.rows {
            for gl in &row.row.glyphs {
                glyphs.push(gl.logical_rect().translate(row.pos.to_vec2() + rect.min.to_vec2()));
            }
            if row.ends_with_newline {
                glyphs.push(Rect::NOTHING);
            }
        }
        let mut hovered = None;
        let p = ui.painter();
        for (k, (a, b, s)) in markers.iter().enumerate() {
            let mut r = Rect::NOTHING;
            for gr in glyphs.iter().take(b.saturating_sub(1).min(glyphs.len())).skip(a + 1) {
                r = r.union(*gr);
            }
            if !r.is_positive() {
                continue;
            }
            let pill = Rect::from_center_size(r.center(), vec2(r.width().max(16.0), self.size * 1.0));
            let over = ptr.is_some_and(|q| pill.expand(2.0).contains(q));
            if over {
                hovered = Some(*s);
            }
            let on = toggle(ui, ui.scope_id().with(("cite", k)), over || highlight == Some(*s));
            p.rect_filled(pill, cr(pill.height() / 2.0), mix(track(&t), with_alpha(t.accent, 0.9), on));
        }
        p.galley(rect.min, g.clone(), t.foreground);
        // redraw marker digits in contrasting color when lit
        for (k, (a, b, s)) in markers.iter().enumerate() {
            let on = toggle(ui, ui.scope_id().with(("cite", k)), hovered == Some(*s) || highlight == Some(*s));
            if on > 0.5 {
                let mut r = Rect::NOTHING;
                for gr in glyphs.iter().take(b.saturating_sub(1).min(glyphs.len())).skip(a + 1) {
                    r = r.union(*gr);
                }
                ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{}", s + 1), FontId::proportional(self.size * 0.72), t.accent_foreground);
            }
        }
        if hovered.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        hovered
    }
}

/// A "Sources" footer for an answer: a compact row of overlapping favicons and a count that
/// expands (on click) into a grid of numbered source cards, each lifting on hover. Pass the source
/// hovered in [`CitedText`] to highlight its card; returns the card hovered here.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{Citation, CitationsFooter};
/// let src = [Citation::new("The Rust book", "doc.rust-lang.org", egui::Color32::BROWN)];
/// let mut open = false;
/// let hovered = CitationsFooter::new(&src, &mut open).show(ui, None);
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct CitationsFooter<'a> {
    sources: &'a [Citation],
    open: &'a mut bool,
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> CitationsFooter<'a> {
    /// A footer over `sources`; `open` toggles the grid.
    pub fn new(sources: &'a [Citation], open: &'a mut bool) -> Self {
        Self { sources, open, width: None, salt: None }
    }
    /// Width (default: available).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the footer; `highlight` lights up one card. Returns the hovered card.
    pub fn show(self, ui: &mut Ui, highlight: Option<usize>) -> Option<usize> {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_cites");
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let n = self.sources.len();
        let (hr, hresp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
        let ok = toggle(ui, id.with("open"), *self.open);
        let hk = toggle(ui, id.with("hh"), hresp.hovered());
        let p = ui.painter();
        let pill_w = 20.0 + (n.min(5) as f32) * 13.0 + 96.0;
        let pill = Rect::from_min_size(hr.min, vec2(pill_w, 32.0));
        p.rect_filled(pill, cr(16.0), mix(well(&t), surface(&t), 0.3 - 0.3 * hk));
        p.rect_stroke(pill, cr(16.0), hairline(&t), StrokeKind::Inside);
        for (i, s) in self.sources.iter().take(5).enumerate().rev() {
            let spread = 13.0 + 4.0 * hk;
            let c = pos2(pill.min.x + 18.0 + i as f32 * spread, pill.center().y);
            p.circle_filled(c, 9.5, surface(&t));
            p.circle_filled(c, 8.0, s.color);
            p.text(c, Align2::CENTER_CENTER, s.domain.chars().next().unwrap_or('?').to_uppercase().to_string(), FontId::proportional(8.5), Color32::WHITE);
        }
        let tx = pill.min.x + 20.0 + n.min(5) as f32 * 13.0 + 4.0 * hk * n.min(5) as f32;
        text(p, pos2(tx, pill.center().y), Align2::LEFT_CENTER, &format!("{n} sources"), 12.5, t.foreground);
        let c = pos2(pill.max.x - 16.0, pill.center().y);
        let ang = ok * std::f32::consts::PI;
        let rot = |v: Vec2| vec2(v.x, v.y * ang.cos());
        p.line_segment([c + rot(vec2(-3.5, -1.8)), c + rot(vec2(0.0, 1.8))], Stroke::new(1.4, t.muted_foreground));
        p.line_segment([c + rot(vec2(0.0, 1.8)), c + rot(vec2(3.5, -1.8))], Stroke::new(1.4, t.muted_foreground));
        if hresp.clicked() {
            *self.open = !*self.open;
        }
        let mut hovered = None;
        if ok > 0.01 {
            let cols = 2;
            let gap = 8.0;
            let cw = (w - gap) / cols as f32;
            let ch = 58.0;
            let rows = n.div_ceil(cols);
            let full = 8.0 + rows as f32 * (ch + gap);
            let (gr, _) = ui.allocate_exact_size(vec2(w, full * ok), Sense::hover());
            for (i, s) in self.sources.iter().enumerate() {
                let (cx, cy) = (i % cols, i / cols);
                let r = Rect::from_min_size(pos2(gr.min.x + cx as f32 * (cw + gap), gr.min.y + 8.0 + cy as f32 * (ch + gap)), vec2(cw, ch));
                let k = ((ok - i as f32 * 0.05) * 1.4).clamp(0.0, 1.0);
                let resp = ui.interact(r, id.with(("c", i)), Sense::click());
                if resp.hovered() {
                    hovered = Some(i);
                }
                let lk = toggle(ui, id.with(("ch", i)), resp.hovered() || highlight == Some(i));
                let r = r.translate(vec2(0.0, (1.0 - k) * 8.0 - lk * 2.0));
                let p = ui.painter().with_clip_rect(gr.intersect(ui.clip_rect()).expand2(vec2(10.0, 0.0)));
                paint::soft_shadow(&p, r, 12.0, 6.0 + 10.0 * lk, vec2(0.0, 2.0 + 2.0 * lk), shadow(&t, (0.3 + 0.8 * lk) * k));
                p.rect_filled(r, cr(12.0), surface(&t).gamma_multiply(k));
                p.rect_stroke(r, cr(12.0), Stroke::new(1.0, mix(hairline(&t).color, t.accent, lk * 0.7).gamma_multiply(k)), StrokeKind::Inside);
                let nb = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(18.0, 18.0));
                p.rect_filled(nb, cr(5.0), mix(well(&t), t.accent, lk).gamma_multiply(k));
                p.text(nb.center(), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(10.5), mix(t.muted_foreground, t.accent_foreground, lk).gamma_multiply(k));
                p.circle_filled(pos2(nb.max.x + 12.0, nb.center().y), 6.0, s.color.gamma_multiply(k));
                p.text(pos2(nb.max.x + 22.0, nb.center().y), Align2::LEFT_CENTER, &s.domain, FontId::proportional(11.0), t.muted_foreground.gamma_multiply(k));
                let clip = Rect::from_min_max(r.min, r.max - vec2(10.0, 0.0));
                p.with_clip_rect(clip).text(pos2(r.min.x + 10.0, r.min.y + 41.0), Align2::LEFT_CENTER, &s.title, FontId::proportional(12.5), t.foreground.gamma_multiply(k));
            }
        }
        hovered
    }
}

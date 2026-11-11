use egui::{pos2, vec2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::paint::{circle_points, fill_convex_fn, linear_fn, rounded_rect_points};
use glowkit_core::Theme;

/// Initials from a name ("Ada Lovelace" → "AL").
pub(crate) fn initials(name: &str) -> String {
    let mut parts = name.split_whitespace().filter_map(|w| w.chars().find(|c| c.is_alphanumeric()));
    let a = parts.next();
    let b = name.split_whitespace().skip(1).last().and_then(|w| w.chars().find(|c| c.is_alphanumeric()));
    let mut s = String::new();
    if let Some(a) = a {
        s.extend(a.to_uppercase());
    }
    if let Some(b) = b {
        s.extend(b.to_uppercase());
    }
    s
}

/// A stable color for a name, picked from the theme's chart palette.
pub(crate) fn name_color(t: &Theme, name: &str) -> Color32 {
    let mut h: u32 = 2166136261;
    for b in name.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let c = t.chart[(h % t.chart.len() as u32) as usize];
    if t.dark {
        shade(c, -0.08)
    } else {
        c
    }
}

/// Presence indicator shown on an [`Avatar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarStatus {
    /// Green.
    Online,
    /// Amber.
    Away,
    /// Red.
    Busy,
    /// Gray.
    Offline,
}

/// How an [`Avatar`]'s fallback is colored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarStyle {
    /// Gradient fill in a color derived from the name, white initials.
    #[default]
    Solid,
    /// Tinted fill with colored initials.
    Soft,
    /// Neutral muted fill (the classic shadcn fallback).
    Muted,
}

/// A circular (or rounded-square) avatar with initials, a name-derived color, an optional ring
/// and a presence dot.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Avatar, AvatarStatus};
/// ui.add(Avatar::new("Ada Lovelace").size(40.0).status(AvatarStatus::Online));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Avatar {
    name: String,
    size: f32,
    status: Option<AvatarStatus>,
    style: AvatarStyle,
    square: bool,
    color: Option<Color32>,
    ring: Option<Color32>,
}

impl Avatar {
    /// An avatar for `name` (initials are derived from it).
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), size: 40.0, status: None, style: AvatarStyle::Solid, square: false, color: None, ring: None }
    }
    /// Diameter in points (default 40).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Presence dot in the bottom-right.
    pub fn status(mut self, s: AvatarStatus) -> Self {
        self.status = Some(s);
        self
    }
    /// Fill style.
    pub fn style(mut self, s: AvatarStyle) -> Self {
        self.style = s;
        self
    }
    /// Rounded square instead of a circle.
    pub fn square(mut self, s: bool) -> Self {
        self.square = s;
        self
    }
    /// Override the derived color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Draw a 2 pt ring in this color (used to separate overlapping avatars).
    pub fn ring(mut self, c: Color32) -> Self {
        self.ring = Some(c);
        self
    }

    /// Paint into a square rect.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let painter = ui.painter();
        let c = rect.center();
        let r = rect.width() / 2.0;
        let col = self.color.unwrap_or_else(|| name_color(&t, &self.name));
        let corner = r * 0.45;
        if let Some(ring) = self.ring {
            if self.square {
                painter.rect_filled(rect.expand(2.0), CornerRadius::same((corner + 2.0) as u8), ring);
            } else {
                painter.circle_filled(c, r + 2.0, ring);
            }
        }
        let outline: Vec<Pos2> = if self.square { rounded_rect_points(rect, corner, 8) } else { circle_points(c, r, 64) };
        let (text_col, fs_k) = match self.style {
            AvatarStyle::Solid => {
                let stops = [shade(col, 0.07), shade(col, -0.07)];
                fill_convex_fn(painter, &outline, 3, linear_fn(rect, 60.0, &stops));
                (Color32::WHITE, 0.38)
            }
            AvatarStyle::Soft => {
                let fill = mix(if t.dark { t.card } else { t.background }, col, if t.dark { 0.22 } else { 0.16 });
                fill_convex_fn(painter, &outline, 2, |_| fill);
                (if t.dark { mix(col, Color32::WHITE, 0.35) } else { shade(col, -0.12) }, 0.38)
            }
            AvatarStyle::Muted => {
                fill_convex_fn(painter, &outline, 2, |_| t.muted);
                (t.muted_foreground, 0.36)
            }
        };
        // hairline edge for crispness
        if self.square {
            painter.rect_stroke(rect, CornerRadius::same(corner as u8), Stroke::new(1.0, with_alpha(Color32::BLACK, 0.06)), StrokeKind::Inside);
        } else {
            painter.circle_stroke(c, r - 0.5, Stroke::new(1.0, with_alpha(if self.style == AvatarStyle::Muted { t.border } else { Color32::BLACK }, if self.style == AvatarStyle::Muted { 1.0 } else { 0.06 })));
        }
        painter.text(c, egui::Align2::CENTER_CENTER, initials(&self.name), FontId::proportional((rect.width() * fs_k).max(8.0)), text_col);
        if let Some(s) = self.status {
            let dot = match s {
                AvatarStatus::Online => t.success,
                AvatarStatus::Away => t.warning,
                AvatarStatus::Busy => t.destructive,
                AvatarStatus::Offline => t.muted_foreground,
            };
            let dr = (r * 0.27).max(3.5);
            let off = if self.square { r - dr * 0.3 } else { r * 0.7071 };
            let dc = c + vec2(off, off);
            painter.circle_filled(dc, dr + 2.0, self.ring.unwrap_or(t.card));
            painter.circle_filled(dc, dr, dot);
        }
    }
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect);
        }
        resp
    }
}

/// Overlapping avatars with a "+N" bubble for the rest; each lifts with a spring on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::AvatarGroup;
/// ui.add(AvatarGroup::new(&["Ada Lovelace", "Alan Turing", "Grace Hopper", "Linus Torvalds", "Margaret Hamilton"]).max(4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AvatarGroup {
    names: Vec<String>,
    max: usize,
    size: f32,
    overlap: f32,
    extra: usize,
    style: AvatarStyle,
    ring: Option<Color32>,
    accent_more: bool,
}

impl AvatarGroup {
    /// A group of avatars for these names.
    pub fn new(names: &[&str]) -> Self {
        Self { names: names.iter().map(|s| s.to_string()).collect(), max: 5, size: 36.0, overlap: 0.3, extra: 0, style: AvatarStyle::Solid, ring: None, accent_more: false }
    }
    /// Show at most this many avatars, then "+N".
    pub fn max(mut self, m: usize) -> Self {
        self.max = m.max(1);
        self
    }
    /// Avatar size.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Fraction of each avatar hidden under the next (default 0.3).
    pub fn overlap(mut self, o: f32) -> Self {
        self.overlap = o.clamp(0.0, 0.8);
        self
    }
    /// Additional people not in `names` (added to the "+N").
    pub fn extra(mut self, n: usize) -> Self {
        self.extra = n;
        self
    }
    /// Fill style of the avatars.
    pub fn style(mut self, s: AvatarStyle) -> Self {
        self.style = s;
        self
    }
    /// Ring color separating avatars (default: theme card color).
    pub fn ring(mut self, c: Color32) -> Self {
        self.ring = Some(c);
        self
    }
}

impl Widget for AvatarGroup {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let shown = self.names.len().min(self.max);
        let more = self.names.len() - shown + self.extra;
        let count = shown + usize::from(more > 0);
        let step = self.size * (1.0 - self.overlap);
        let w = if count == 0 { 0.0 } else { step * (count - 1) as f32 + self.size };
        let (rect, resp) = ui.allocate_exact_size(vec2(w + 4.0, self.size + 8.0), Sense::hover());
        let ring = self.ring.unwrap_or(t.card);
        let hover = resp.hover_pos();
        for i in 0..count {
            let x = rect.min.x + 2.0 + step * i as f32;
            let base = Rect::from_min_size(pos2(x, rect.min.y + 6.0), vec2(self.size, self.size));
            let hovered = hover.map(|p| p.x >= x && p.x < x + if i + 1 == count { self.size } else { step } && rect.contains(p)).unwrap_or(false);
            let lift = glowkit_core::anim::spring(ui, resp.id.with(("lift", i)), if hovered { -5.0 } else { 0.0 }, 400.0, 18.0);
            let r = base.translate(vec2(0.0, lift));
            if i < shown {
                Avatar::new(self.names[i].clone()).style(self.style).ring(ring).paint(ui, r);
            } else {
                let p = ui.painter();
                p.circle_filled(r.center(), self.size / 2.0 + 2.0, ring);
                let (fill, txt) = if self.accent_more { (t.primary, t.primary_foreground) } else { (t.muted, t.muted_foreground) };
                p.circle(r.center(), self.size / 2.0, fill, Stroke::new(1.0, if self.accent_more { fill } else { t.border }));
                p.text(r.center(), egui::Align2::CENTER_CENTER, format!("+{more}"), FontId::proportional(self.size * 0.32), txt);
            }
        }
        resp
    }
}

/// Magic-UI-style avatar circles: a tight overlapping row ending in a bold "+N" bubble in the
/// primary color, with hover lift.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::AvatarCircles::new(&["Ada", "Alan", "Grace", "Linus"], 99));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AvatarCircles {
    group: AvatarGroup,
}

impl AvatarCircles {
    /// Avatars for `names` followed by a "+`extra`" bubble.
    pub fn new(names: &[&str], extra: usize) -> Self {
        let mut group = AvatarGroup::new(names).max(names.len().max(1)).extra(extra).overlap(0.25).size(40.0);
        group.accent_more = true;
        Self { group }
    }
    /// Avatar size.
    pub fn size(mut self, s: f32) -> Self {
        self.group = self.group.size(s);
        self
    }
    /// Fill style of the avatars.
    pub fn style(mut self, s: AvatarStyle) -> Self {
        self.group = self.group.style(s);
        self
    }
}

impl Widget for AvatarCircles {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.add(self.group)
    }
}

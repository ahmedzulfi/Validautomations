use super::util::{avatar, bold_text, chevron, fill_star_shaped, heart_points, idle_time, rotate};
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::{
    anim::{smoothstep, stagger, Spring},
    color::with_alpha,
    hex, mix, paint,
    text::Glyphs,
    Theme,
};

/// A person shown by [`AnimatedTooltip`] and [`AnimatedTestimonials`].
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    /// Display name.
    pub name: String,
    /// Role / title line.
    pub title: String,
    /// Avatar gradient colors.
    pub colors: [Color32; 2],
}

impl Person {
    /// A person with a name, a title line and avatar gradient colors.
    pub fn new(name: impl Into<String>, title: impl Into<String>, a: Color32, b: Color32) -> Self {
        Self { name: name.into(), title: title.into(), colors: [a, b] }
    }
    /// Up to two initials from the name.
    pub fn initials(&self) -> String {
        self.name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase()
    }
}

#[derive(Clone, Copy, Debug)]
struct TipState {
    hovered: Option<usize>,
    shown: Spring,
    swing: Spring,
    shift: Spring,
}

/// A row of overlapping avatar circles. The hovered one lifts and grows, and a name tooltip pops
/// above it that swings and tilts with the pointer's horizontal position (on a loose spring).
/// When idle it cycles through the avatars by itself.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{AnimatedTooltip, Person};
/// use glowkit_core::hex;
/// let team = [Person::new("Ada Lovelace", "Engineer", hex("#8B5CF6"), hex("#EC4899"))];
/// ui.add(AnimatedTooltip::new(&team));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AnimatedTooltip<'a> {
    people: &'a [Person],
    size: f32,
    overlap: f32,
    autoplay: bool,
}

impl<'a> AnimatedTooltip<'a> {
    /// A row of avatars for `people`.
    pub fn new(people: &'a [Person]) -> Self {
        Self { people, size: 56.0, overlap: 14.0, autoplay: true }
    }
    /// Avatar diameter in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// How much neighbouring avatars overlap, in points.
    pub fn overlap(mut self, o: f32) -> Self {
        self.overlap = o;
        self
    }
    /// Cycle through the avatars automatically when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for AnimatedTooltip<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.people.len();
        let step = self.size - self.overlap;
        let w = step * n.saturating_sub(1) as f32 + self.size;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.size + 8.0), Sense::hover());
        if !ui.is_rect_visible(rect) || n == 0 {
            return resp;
        }
        let r = self.size / 2.0;
        let cy = rect.max.y - r - 4.0;
        let centers: Vec<Pos2> = (0..n).map(|i| pos2(rect.min.x + r + i as f32 * step, cy)).collect();
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let mut hovered = None;
        let mut offset = 0.0;
        if let Some(pp) = pointer.filter(|p| rect.expand(4.0).contains(*p)) {
            // last (top-most) avatar under the pointer wins
            for (i, c) in centers.iter().enumerate() {
                if c.distance(pp) <= r {
                    hovered = Some(i);
                    offset = ((pp.x - c.x) / r).clamp(-1.0, 1.0);
                }
            }
        }
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, resp.id, hovered.is_some());
        if hovered.is_none() && self.autoplay && idle > 1.0 {
            let slot = time / 1.8;
            let i = slot as usize % n;
            if slot.fract() < 0.9 {
                hovered = Some(i);
                offset = (slot.fract() * std::f32::consts::TAU * 1.3).sin() * 0.9;
            }
            glowkit_core::animate(ui);
        }
        let dt = glowkit_core::dt(ui);
        let st = glowkit_core::with_state(
            ui,
            resp.id.with("tip"),
            || TipState { hovered: None, shown: Spring::new(0.0).with(260.0, 18.0), swing: Spring::new(0.0).with(120.0, 6.0), shift: Spring::new(0.0).with(120.0, 7.0) },
            |s| {
                if hovered.is_some() && s.hovered != hovered {
                    // pop in again for a new person
                    s.shown.value = 0.0;
                    s.shown.velocity = 0.0;
                }
                if hovered.is_some() {
                    s.hovered = hovered;
                }
                s.shown.target = if hovered.is_some() { 1.0 } else { 0.0 };
                s.swing.target = offset * 0.5;
                s.shift.target = offset * 22.0;
                s.shown.step(dt);
                s.swing.step(dt);
                s.shift.step(dt);
                *s
            },
        );
        if !st.shown.is_settled() || !st.swing.is_settled() || hovered.is_some() {
            glowkit_core::animate(ui);
        }
        let p = ui.painter_at(rect.expand2(vec2(120.0, 120.0)));
        let ring = if t.dark { t.card } else { Color32::WHITE };
        let lifts: Vec<f32> = (0..n)
            .map(|i| glowkit_core::anim::spring(ui, resp.id.with(("lift", i)), if hovered == Some(i) { 1.0 } else { 0.0 }, 300.0, 18.0))
            .collect();
        // draw the hovered avatar last
        let mut order: Vec<usize> = (0..n).collect();
        if let Some(h) = st.hovered.filter(|_| hovered.is_some()) {
            order.retain(|&i| i != h);
            order.push(h);
        }
        for &i in &order {
            let c = centers[i] - vec2(0.0, lifts[i] * 4.0);
            let rr = r * (1.0 + lifts[i] * 0.08);
            paint::soft_shadow(&p, Rect::from_center_size(c + vec2(0.0, 2.0), Vec2::splat(rr * 2.0)), rr, 8.0 + lifts[i] * 8.0, Vec2::ZERO, Color32::from_black_alpha(60));
            p.circle_filled(c, rr + 2.5, ring);
            avatar(&p, c, rr, self.people[i].colors, &self.people[i].initials());
        }
        // tooltip
        if let Some(h) = st.hovered {
            let k = st.shown.value;
            if k > 0.01 {
                let person = &self.people[h];
                let c = centers[h] - vec2(0.0, lifts[h] * 4.0);
                let name_g = p.layout_no_wrap(person.name.clone(), FontId::proportional(14.0), Color32::WHITE);
                let title_g = p.layout_no_wrap(person.title.clone(), FontId::proportional(11.5), Color32::WHITE);
                let tw = name_g.size().x.max(title_g.size().x) + 28.0;
                let th = 48.0;
                let tc = pos2(c.x + st.shift.value, c.y - r - 14.0 - th / 2.0 + (1.0 - k) * 18.0);
                let scale = 0.6 + 0.4 * k.clamp(0.0, 1.2);
                let ang = st.swing.value;
                let alpha = k.clamp(0.0, 1.0);
                let box_r = Rect::from_center_size(tc, vec2(tw, th) * scale);
                let pivot = pos2(tc.x, box_r.max.y);
                let pts: Vec<Pos2> = paint::rounded_rect_points(box_r, 8.0 * scale, 6).into_iter().map(|q| rotate(q, pivot, ang)).collect();
                let bg = if t.dark { hex("#000000") } else { hex("#0A0A0A") };
                p.add(Shape::convex_polygon(pts, with_alpha(bg, alpha), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.08 * alpha))));
                // gradient accents along the bottom edge
                for (frac, col, yoff) in [(0.6, hex("#10B981"), 0.0), (0.4, hex("#0EA5E9"), 0.0)] {
                    let half = box_r.width() * frac / 2.0;
                    let a = rotate(pos2(tc.x - half, box_r.max.y + yoff), pivot, ang);
                    let b = rotate(pos2(tc.x + half, box_r.max.y + yoff), pivot, ang);
                    let line: Vec<Pos2> = (0..=16).map(|j| a.lerp(b, j as f32 / 16.0)).collect();
                    paint::gradient_polyline(&p, &line, false, 1.5, |u| with_alpha(col, alpha * (1.0 - (u * 2.0 - 1.0).abs()).powf(0.8)));
                }
                let name_c = rotate(pos2(tc.x, tc.y - 8.0 * scale), pivot, ang);
                let title_c = rotate(pos2(tc.x, tc.y + 10.0 * scale), pivot, ang);
                glowkit_core::text::paint_rotated(&p, &person.name, name_c, FontId::proportional(14.0 * scale), with_alpha(Color32::WHITE, alpha), ang);
                glowkit_core::text::paint_rotated(&p, &person.name, name_c + vec2(0.4, 0.0), FontId::proportional(14.0 * scale), with_alpha(Color32::WHITE, alpha), ang);
                glowkit_core::text::paint_rotated(&p, &person.title, title_c, FontId::proportional(11.5 * scale), with_alpha(hex("#A1A1AA"), alpha), ang);
            }
        }
        resp
    }
}

/// A testimonial for [`AnimatedTestimonials`].
#[derive(Clone, Debug, PartialEq)]
pub struct Testimonial {
    /// Who said it.
    pub person: Person,
    /// What they said.
    pub quote: String,
}

impl Testimonial {
    /// A quote by `person`.
    pub fn new(person: Person, quote: impl Into<String>) -> Self {
        Self { person, quote: quote.into() }
    }
}

#[derive(Clone, Copy, Debug)]
struct TestiState {
    active: usize,
    since: f64,
}

/// Testimonials with a stack of tilted "photo" cards: the active card rotates to the front with
/// a hop while the quote reveals word by word out of a blur. Prev / next buttons, and it
/// advances on its own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{AnimatedTestimonials, Person, Testimonial};
/// use glowkit_core::hex;
/// let items = [Testimonial::new(Person::new("Sarah Chen", "CTO at Nimbus", hex("#8B5CF6"), hex("#22D3EE")), "Incredible.")];
/// ui.add(AnimatedTestimonials::new(&items));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AnimatedTestimonials<'a> {
    items: &'a [Testimonial],
    autoplay: Option<f32>,
    size: Vec2,
}

impl<'a> AnimatedTestimonials<'a> {
    /// Testimonials to cycle through.
    pub fn new(items: &'a [Testimonial]) -> Self {
        Self { items, autoplay: Some(5.0), size: vec2(780.0, 330.0) }
    }
    /// Advance every `seconds` (`None` disables).
    pub fn autoplay(mut self, seconds: Option<f32>) -> Self {
        self.autoplay = seconds;
        self
    }
    /// Overall size in points.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
}

impl Widget for AnimatedTestimonials<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        let n = self.items.len();
        if n == 0 || !ui.is_rect_visible(rect) {
            return resp;
        }
        let now = ui.input(|i| i.time);
        let id = resp.id;
        let mut st: TestiState = glowkit_core::state(ui, id.with("st"), || TestiState { active: 0, since: now });
        let hovered = resp.hovered();
        if let Some(every) = self.autoplay {
            if !hovered && (now - st.since) as f32 > every {
                st = TestiState { active: (st.active + 1) % n, since: now };
            }
            glowkit_core::animate(ui);
        }
        let img_side = (rect.height() - 50.0).min(rect.width() * 0.4);
        let img_rect = Rect::from_min_size(pos2(rect.min.x + 30.0, rect.min.y + 34.0), Vec2::splat(img_side));
        let p = ui.painter_at(rect.expand(30.0));
        let since = (now - st.since) as f32;
        // cards, back to front
        let mut order: Vec<(f32, usize)> = (0..n)
            .map(|i| {
                let rel = (i + n - st.active) % n;
                (if rel == 0 { 100.0 } else { -(rel as f32) }, i)
            })
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, i) in order {
            let is_active = i == st.active;
            let seed_rot = (glowkit_core::rng::hash01(i as u64 * 31 + 7) - 0.5) * 0.36;
            let rot = glowkit_core::anim::spring(ui, id.with(("rot", i)), if is_active { 0.0 } else { seed_rot }, 140.0, 16.0);
            let scale = glowkit_core::anim::spring(ui, id.with(("sc", i)), if is_active { 1.0 } else { 0.94 }, 160.0, 18.0);
            let hop = if is_active { -(std::f32::consts::PI * (since / 0.55).clamp(0.0, 1.0)).sin() * 30.0 } else { 0.0 };
            let c = img_rect.center() + vec2(0.0, hop);
            let r = Rect::from_center_size(c, img_rect.size() * scale);
            let pts: Vec<Pos2> = paint::rounded_rect_points(r, 24.0 * scale, 8).into_iter().map(|q| rotate(q, c, rot)).collect();
            let person = &self.items[i].person;
            let dim = if is_active { 0.0 } else { 0.25 };
            let (a, b) = (mix(person.colors[0], t.background, dim), mix(person.colors[1], t.background, dim));
            if is_active {
                paint::soft_shadow(&p, r, 24.0, 30.0, vec2(0.0, 14.0), Color32::from_black_alpha(if t.dark { 120 } else { 40 }));
            }
            paint::fill_convex_fn(&p, &pts, 6, |q| {
                let u = ((q - r.min).dot(vec2(0.7, 0.7)) / (r.width() * 1.4)).clamp(0.0, 1.0);
                a.lerp_to_gamma(b, u)
            });
            // soft light + big initials
            let lc = rotate(r.min + r.size() * vec2(0.3, 0.25), c, rot);
            paint::glow(&p, lc, r.width() * 0.55, Color32::from_white_alpha(if is_active { 40 } else { 20 }));
            let ini = person.initials();
            glowkit_core::text::paint_rotated(&p, &ini, c, FontId::proportional(r.width() * 0.32), Color32::from_white_alpha(if is_active { 235 } else { 140 }), rot);
            glowkit_core::text::paint_rotated(&p, &ini, c + vec2(1.0, 0.0), FontId::proportional(r.width() * 0.32), Color32::from_white_alpha(if is_active { 235 } else { 140 }), rot);
        }
        // text column
        let tx = img_rect.max.x + 56.0;
        let tw = rect.max.x - tx - 10.0;
        let item = &self.items[st.active];
        let k_head = smoothstep(0.0, 0.35, since);
        let dy = (1.0 - k_head) * 12.0;
        bold_text(&p, pos2(tx, rect.min.y + 24.0 + dy), Align2::LEFT_TOP, &item.person.name, FontId::proportional(24.0), with_alpha(t.foreground, k_head));
        p.text(pos2(tx, rect.min.y + 58.0 + dy), Align2::LEFT_TOP, &item.person.title, FontId::proportional(13.5), with_alpha(t.muted_foreground, k_head));
        let font = FontId::proportional(17.0);
        let glyphs = Glyphs::layout(&p, &item.quote, font.clone(), Some(tw));
        let origin = pos2(tx, rect.min.y + 96.0);
        let words = glyphs.word_count();
        for wi in 0..words {
            let gs: Vec<_> = glyphs.glyphs.iter().filter(|g| g.word == wi && !g.ch.is_whitespace()).collect();
            let Some(first) = gs.first() else { continue };
            let word: String = gs.iter().map(|g| g.ch).collect();
            let k = stagger(since - 0.15, wi, 0.025, 0.3);
            if k <= 0.0 {
                continue;
            }
            let g = p.layout_no_wrap(word, font.clone(), Color32::PLACEHOLDER);
            let pos = origin + first.offset + vec2(0.0, (1.0 - k) * 5.0);
            let col = with_alpha(mix(t.muted_foreground, t.foreground, 0.35), k);
            paint::blurred_galley(&p, pos, &g, col, (1.0 - k) * 5.0);
        }
        // buttons
        let by = rect.max.y - 34.0;
        let mut clicked = None;
        for (j, dir) in [(0usize, -1.0f32), (1, 1.0)] {
            let c = pos2(tx + 20.0 + j as f32 * 50.0, by);
            let br = Rect::from_center_size(c, Vec2::splat(38.0));
            let b = ui.interact(br, id.with(("btn", j)), Sense::click());
            let hv = glowkit_core::anim::follow(ui, id.with(("bh", j)), if b.hovered() { 1.0 } else { 0.0 }, 14.0);
            p.circle_filled(c, 19.0, t.muted.lerp_to_gamma(t.secondary, hv));
            p.circle_stroke(c, 19.0, Stroke::new(1.0, t.border));
            chevron(&p, c + vec2(dir * (-1.5 + hv * 2.0), 0.0), 10.0, vec2(dir, 0.0), Stroke::new(2.0, t.foreground));
            if b.clicked() {
                clicked = Some(dir);
            }
            if b.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        // dots
        for i in 0..n {
            let c = pos2(tx + 130.0 + i as f32 * 14.0, by);
            let on = glowkit_core::anim::follow(ui, id.with(("dot", i)), if i == st.active { 1.0 } else { 0.0 }, 10.0);
            let w = 6.0 + on * 12.0;
            p.rect_filled(Rect::from_center_size(c + vec2(on * 6.0 + if i > st.active { 12.0 } else { 0.0 }, 0.0), vec2(w, 6.0)), 3.0, t.muted_foreground.lerp_to_gamma(t.foreground, on));
        }
        if let Some(d) = clicked {
            st.active = if d > 0.0 { (st.active + 1) % n } else { (st.active + n - 1) % n };
            st.since = now;
        }
        if since < 2.5 {
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        resp
    }
}

/// Content of a [`TweetCard`].
#[derive(Clone, Debug, PartialEq)]
pub struct Tweet {
    /// Author display name.
    pub name: String,
    /// Handle without the `@`.
    pub handle: String,
    /// Post text (`@mentions`, `#tags` and links are highlighted).
    pub text: String,
    /// Avatar gradient.
    pub avatar: [Color32; 2],
    /// Show the verified badge.
    pub verified: bool,
    /// Timestamp text, e.g. "2h" or "Mar 4".
    pub time: String,
    /// Optional media: a gradient "image" under the text.
    pub media: Option<[Color32; 2]>,
    /// Reply count.
    pub replies: u32,
    /// Repost count.
    pub reposts: u32,
    /// Like count.
    pub likes: u32,
    /// View count.
    pub views: u32,
}

impl Tweet {
    /// A post by `name` (@`handle`) with `text`.
    pub fn new(name: impl Into<String>, handle: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            handle: handle.into(),
            text: text.into(),
            avatar: [hex("#8B5CF6"), hex("#EC4899")],
            verified: true,
            time: "2h".into(),
            media: None,
            replies: 12,
            reposts: 48,
            likes: 312,
            views: 18_400,
        }
    }
}

fn compact(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}K", n as f32 / 1e3)
    } else if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

fn verified_badge(p: &egui::Painter, c: Pos2, r: f32) {
    paint::star(p, c, r, r * 0.82, 8, 0.0, hex("#1D9BF0"));
    p.circle_filled(c, r * 0.8, hex("#1D9BF0"));
    super::util::check(p, c + vec2(0.0, 0.3), r * 1.0, Stroke::new(r * 0.24, Color32::WHITE), 1.0);
}

#[derive(Clone, Copy, Debug, Default)]
struct LikeState {
    liked: bool,
    at: f64,
}

/// A social post card: avatar, name with verified badge, handle and time, rich text with
/// highlighted mentions / tags / links, optional media, and an animated like button with a
/// particle burst. Lifts on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let tweet = glowkit_blocks::Tweet::new("glowkit", "glowkit_rs", "Shipping 30 new components today ✨ #rustlang");
/// ui.add(glowkit_blocks::TweetCard::new(&tweet).width(420.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TweetCard<'a> {
    tweet: &'a Tweet,
    width: f32,
}

impl<'a> TweetCard<'a> {
    /// A card for `tweet`.
    pub fn new(tweet: &'a Tweet) -> Self {
        Self { tweet, width: 420.0 }
    }
    /// Card width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl Widget for TweetCard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let tw = self.tweet;
        let pad = 18.0;
        let inner_w = self.width - pad * 2.0;
        // rich text
        let link = if t.dark { hex("#1D9BF0") } else { hex("#1A8CD8") };
        let mut job = LayoutJob::default();
        job.wrap.max_width = inner_w;
        let fmt = |c: Color32| TextFormat { font_id: FontId::proportional(15.0), color: c, line_height: Some(21.0), ..Default::default() };
        for (i, word) in tw.text.split(' ').enumerate() {
            if i > 0 {
                job.append(" ", 0.0, fmt(t.foreground));
            }
            let hl = word.starts_with('@') || word.starts_with('#') || word.starts_with("http") || word.contains(".rs");
            job.append(word, 0.0, fmt(if hl { link } else { t.foreground }));
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let media_h = if tw.media.is_some() { inner_w * 0.5 } else { 0.0 };
        let h = pad + 44.0 + 12.0 + galley.size().y + if media_h > 0.0 { 12.0 + media_h } else { 0.0 } + 16.0 + 22.0 + pad;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let lift = glowkit_core::anim::follow(ui, id.with("lift"), if resp.hovered() { 1.0 } else { 0.0 }, 10.0);
        let rect_l = rect.translate(vec2(0.0, -3.0 * lift));
        let p = ui.painter_at(rect.expand(30.0));
        paint::soft_shadow(&p, rect_l, 16.0, 16.0 + 16.0 * lift, vec2(0.0, 4.0 + 6.0 * lift), Color32::from_black_alpha(if t.dark { (60.0 + 60.0 * lift) as u8 } else { (14.0 + 24.0 * lift) as u8 }));
        p.rect_filled(rect_l, 16.0, if t.dark { t.card } else { Color32::WHITE });
        p.rect_stroke(rect_l, 16.0, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, lift * 0.35)), StrokeKind::Inside);
        let o = rect_l.min + vec2(pad, pad);
        avatar(&p, o + vec2(22.0, 22.0), 22.0, tw.avatar, &tw.name.chars().next().map(|c| c.to_string()).unwrap_or_default());
        let nr = bold_text(&p, o + vec2(54.0, 4.0), Align2::LEFT_TOP, &tw.name, FontId::proportional(15.0), t.foreground);
        if tw.verified {
            verified_badge(&p, pos2(nr.max.x + 11.0, nr.center().y), 7.5);
        }
        p.text(o + vec2(54.0, 25.0), Align2::LEFT_TOP, format!("@{} · {}", tw.handle, tw.time), FontId::proportional(13.0), t.muted_foreground);
        // X-like logo
        let lc = pos2(rect_l.max.x - pad - 9.0, o.y + 11.0);
        let s = 6.0;
        p.line_segment([lc + vec2(-s, -s), lc + vec2(s, s)], Stroke::new(2.6, t.foreground));
        p.line_segment([lc + vec2(s, -s), lc + vec2(-s, s)], Stroke::new(1.3, t.foreground));
        let ty = o.y + 56.0;
        p.galley(pos2(o.x, ty), galley.clone(), t.foreground);
        let mut y = ty + galley.size().y;
        if let Some(m) = tw.media {
            y += 12.0;
            let mr = Rect::from_min_size(pos2(o.x, y), vec2(inner_w, media_h));
            paint::gradient_rect(&p, mr, 12.0, 30.0, &[m[0], mix(m[0], m[1], 0.5), m[1]]);
            paint::glow(&p, mr.center() + vec2(-mr.width() * 0.15, -mr.height() * 0.1), mr.width() * 0.45, Color32::from_white_alpha(40));
            // abstract shapes
            p.circle_filled(mr.center() + vec2(mr.width() * 0.22, 0.0), mr.height() * 0.22, Color32::from_white_alpha(46));
            p.rect_filled(Rect::from_center_size(mr.center() - vec2(mr.width() * 0.12, -mr.height() * 0.08), vec2(mr.width() * 0.3, mr.height() * 0.36)), 12.0, Color32::from_white_alpha(36));
            p.rect_stroke(mr, 12.0, Stroke::new(1.0, with_alpha(t.border, 0.6)), StrokeKind::Inside);
            y += media_h;
        }
        y += 16.0;
        // metrics
        let now = ui.input(|i| i.time);
        let mut like: LikeState = glowkit_core::state(ui, id.with("like"), LikeState::default);
        let mc = t.muted_foreground;
        let font = FontId::proportional(12.5);
        let cols = inner_w / 4.0;
        let row_c = |i: usize| pos2(o.x + cols * i as f32 + 9.0, y + 9.0);
        // reply bubble
        let c0 = row_c(0);
        p.circle_stroke(c0, 7.0, Stroke::new(1.4, mc));
        p.text(c0 + vec2(14.0, 0.0), Align2::LEFT_CENTER, compact(tw.replies), font.clone(), mc);
        // repost arrows
        let c1 = row_c(1);
        let st = Stroke::new(1.4, mc);
        p.add(Shape::line(vec![c1 + vec2(-7.0, 2.0), c1 + vec2(-7.0, -4.0), c1 + vec2(4.0, -4.0)], st));
        chevron(&p, c1 + vec2(4.0, -4.0), 5.0, vec2(1.0, 0.0), st);
        p.add(Shape::line(vec![c1 + vec2(7.0, -2.0), c1 + vec2(7.0, 4.0), c1 + vec2(-4.0, 4.0)], st));
        chevron(&p, c1 + vec2(-4.0, 4.0), 5.0, vec2(-1.0, 0.0), st);
        p.text(c1 + vec2(14.0, 0.0), Align2::LEFT_CENTER, compact(tw.reposts), font.clone(), mc);
        // like
        let c2 = row_c(2);
        let like_rect = Rect::from_center_size(c2 + vec2(16.0, 0.0), vec2(60.0, 26.0));
        let lr = ui.interact(like_rect, id.with("likebtn"), Sense::click());
        if lr.clicked() {
            like = LikeState { liked: !like.liked, at: now };
        }
        let le = (now - like.at) as f32;
        let red = hex("#F91880");
        let pop = if like.liked && le < 0.45 { 1.0 + (le / 0.45 * std::f32::consts::PI).sin() * 0.35 } else { 1.0 };
        let hc = c2 + vec2(0.0, 0.5);
        if like.liked {
            fill_star_shaped(&p, hc + vec2(0.0, 1.5 * pop), &heart_points(hc, 16.0 * pop), red);
            if le < 0.6 {
                let k = le / 0.6;
                for j in 0..8 {
                    let a = j as f32 / 8.0 * std::f32::consts::TAU;
                    let col = if j % 2 == 0 { red } else { hex("#FFB800") };
                    p.circle_filled(hc + vec2(a.cos(), a.sin()) * (8.0 + 14.0 * k), 2.0 * (1.0 - k), col);
                }
                p.circle_stroke(hc, 4.0 + 10.0 * k, Stroke::new(2.0 * (1.0 - k), with_alpha(red, 1.0 - k)));
                glowkit_core::animate(ui);
            }
        } else {
            let hv = if lr.hovered() { red } else { mc };
            p.add(Shape::closed_line(heart_points(hc, 16.0), Stroke::new(1.4, hv)));
        }
        let likes = tw.likes + like.liked as u32;
        p.text(c2 + vec2(14.0, 0.0), Align2::LEFT_CENTER, compact(likes), font.clone(), if like.liked { red } else { mc });
        if lr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        // views
        let c3 = row_c(3);
        for (j, hh) in [5.0, 10.0, 7.0].iter().enumerate() {
            let x = c3.x - 5.0 + j as f32 * 4.5;
            p.line_segment([pos2(x, c3.y + 5.0), pos2(x, c3.y + 5.0 - hh)], Stroke::new(1.6, mc));
        }
        p.text(c3 + vec2(14.0, 0.0), Align2::LEFT_CENTER, compact(tw.views), font, mc);
        glowkit_core::set_state(ui, id.with("like"), like);
        resp
    }
}


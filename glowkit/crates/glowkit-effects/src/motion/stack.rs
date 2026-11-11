use super::common::{paint_card_face, CardFace, CardPose};
use egui::{vec2, Color32, CursorIcon, Pos2, Response, Sense, Ui, Vec2};
use glowkit_core::anim::{ease, Easing, Spring};
use glowkit_core::{Clock, Theme, Trigger};

#[derive(Clone, Copy)]
struct CardAnim {
    x: Spring,
    y: Spring,
    angle: Spring,
    scale: Spring,
}

impl CardAnim {
    fn new(off: Vec2, angle: f32, scale: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            x: Spring::new(off.x).with(stiffness, damping),
            y: Spring::new(off.y).with(stiffness, damping),
            angle: Spring::new(angle).with(stiffness, damping),
            scale: Spring::new(scale).with(stiffness, damping),
        }
    }
    fn step(&mut self, off: Vec2, angle: f32, scale: f32, dt: f32) -> bool {
        self.x.target = off.x;
        self.y.target = off.y;
        self.angle.target = angle;
        self.scale.target = scale;
        self.x.step(dt);
        self.y.step(dt);
        self.angle.step(dt);
        self.scale.step(dt);
        self.x.is_settled() && self.y.is_settled() && self.angle.is_settled() && self.scale.is_settled()
    }
    fn off(&self) -> Vec2 {
        vec2(self.x.value, self.y.value)
    }
}

fn hash_angle(i: usize) -> f32 {
    let mut r = glowkit_core::Rng::new(i as u64 * 104_729 + 17);
    let mag = r.range(4.0, 9.0);
    if i % 2 == 0 { mag } else { -mag }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct StackState {
    order: Vec<usize>,
    anims: Vec<CardAnim>,
    drag: Vec2,
    dragging: bool,
    auto_t0: f32,
    auto_dir: f32,
}

/// A stack of cards you can drag: fling the top card past the threshold and it tucks itself
/// in at the back of the stack; let go early and it springs back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CardFace, StackCards};
/// let faces = [CardFace::new("🌊", "Ocean"), CardFace::new("🌅", "Sunset"), CardFace::new("🌲", "Forest")];
/// ui.add(StackCards::new(&faces).card_size(egui::vec2(200.0, 260.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StackCards<'a> {
    faces: &'a [CardFace],
    card_size: Vec2,
    threshold: f32,
    random_rotation: bool,
    click_to_back: bool,
    autoplay: Option<f32>,
}

impl<'a> StackCards<'a> {
    /// A draggable stack of `faces` (the first face starts on top).
    pub fn new(faces: &'a [CardFace]) -> Self {
        Self { faces, card_size: vec2(200.0, 260.0), threshold: 110.0, random_rotation: true, click_to_back: true, autoplay: None }
    }
    /// Card size in points.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Drag distance needed to send the top card to the back.
    pub fn threshold(mut self, d: f32) -> Self {
        self.threshold = d.max(10.0);
        self
    }
    /// Give each card a slight random tilt (default on).
    pub fn random_rotation(mut self, on: bool) -> Self {
        self.random_rotation = on;
        self
    }
    /// Clicking the top card also sends it to the back (default on).
    pub fn click_to_back(mut self, on: bool) -> Self {
        self.click_to_back = on;
        self
    }
    /// Fling the top card automatically every `seconds` while idle.
    pub fn autoplay(mut self, seconds: f32) -> Self {
        self.autoplay = Some(seconds.max(0.5));
        self
    }
}

impl egui::Widget for StackCards<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.faces.len();
        let size = vec2(ui.available_width(), self.card_size.y + 90.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let c = rect.center();
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let card_rect = egui::Rect::from_center_size(c, self.card_size);
        let over = response.hover_pos().map(|p| card_rect.contains(p)).unwrap_or(false);
        let interacting = response.dragged() || over;
        if over || response.dragged() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        let threshold = self.threshold;
        let random = self.random_rotation;
        let autoplay = self.autoplay;
        let st = glowkit_core::with_state(ui, id.with("st"), StackState::default, |st| {
            if st.order.len() != n {
                // last = top; first face on top
                st.order = (0..n).rev().collect();
                st.anims = (0..n).map(|_| CardAnim::new(Vec2::ZERO, 0.0, 1.0, 260.0, 22.0)).collect();
                st.auto_t0 = now;
            }
            let mut release = false;
            if response.drag_started() {
                st.dragging = true;
            }
            if st.dragging {
                st.drag += response.drag_delta();
                st.auto_t0 = now;
                if response.drag_stopped() || !response.dragged() {
                    st.dragging = false;
                    release = true;
                }
            } else if self.click_to_back && response.clicked() && over {
                st.drag = vec2(0.0, -threshold * 1.2);
                release = true;
            } else if let Some(every) = autoplay {
                if !interacting {
                    let p = (now - st.auto_t0 - every) / 0.35;
                    if p >= 0.0 {
                        if st.auto_dir == 0.0 {
                            st.auto_dir = if ((now * 13.7).sin()) > 0.0 { 1.0 } else { -1.0 };
                        }
                        let e = ease(Easing::InOutCubic, p.min(1.0));
                        st.drag = vec2(st.auto_dir * threshold * 1.4, -threshold * 0.25) * e;
                        if p >= 1.0 {
                            release = true;
                            st.auto_t0 = now;
                            st.auto_dir = 0.0;
                        }
                    }
                } else {
                    st.auto_t0 = now;
                }
            }
            if release {
                let top = *st.order.last().unwrap_or(&0);
                if st.drag.length() > threshold {
                    // send to back: keep current displacement and let the spring carry it home
                    st.anims[top].x.value = st.drag.x;
                    st.anims[top].y.value = st.drag.y;
                    st.order.pop();
                    st.order.insert(0, top);
                } else {
                    st.anims[top].x.value = st.drag.x;
                    st.anims[top].y.value = st.drag.y;
                }
                st.drag = Vec2::ZERO;
            }
            let mut settled = true;
            for (depth_from_bottom, &card) in st.order.iter().enumerate() {
                let depth = n - 1 - depth_from_bottom;
                let base_angle = if depth == 0 { 0.0 } else if random { hash_angle(card) } else { depth as f32 * 4.0 };
                let scale = 1.0 - depth as f32 * 0.04;
                let off = vec2(hash_angle(card) * depth.min(1) as f32 * 0.8, depth as f32 * -7.0);
                if depth == 0 && (st.dragging || st.drag != Vec2::ZERO) {
                    let a = &mut st.anims[card];
                    a.x.value = st.drag.x;
                    a.y.value = st.drag.y;
                    a.x.velocity = 0.0;
                    a.y.velocity = 0.0;
                    a.angle.target = st.drag.x * 0.08;
                    a.angle.step(dt);
                    a.scale.target = 1.03;
                    a.scale.step(dt);
                    settled = false;
                } else {
                    settled &= st.anims[card].step(off, base_angle, scale, dt);
                }
            }
            (st.clone(), settled)
        });
        let (st, settled) = st;
        if !settled || autoplay.is_some() {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect.expand(40.0));
        for &card in &st.order {
            let a = &st.anims[card];
            let mut pose = CardPose::new(c + a.off(), self.card_size);
            pose.angle = a.angle.value.to_radians();
            pose.scale = a.scale.value;
            paint_card_face(&painter, &t, &self.faces[card], card, pose);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct SwapState {
    order: Vec<usize>,
    slots: Vec<Spring>,
    last: f32,
    swapping: Option<(usize, f32)>,
    paused_at: Option<f32>,
}

/// A skewed 3D-looking stack of cards that cycles on its own: the front card drops away and
/// reappears at the back while the others move forward.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CardFace, CardSwap};
/// let faces = [CardFace::new("⚙", "Customizable"), CardFace::new("🛡", "Reliable"), CardFace::new("✨", "Smooth")];
/// ui.add(CardSwap::new(&faces).interval(4.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CardSwap<'a> {
    faces: &'a [CardFace],
    card_size: Vec2,
    distance: Vec2,
    skew: f32,
    interval: f32,
    pause_on_hover: bool,
}

impl<'a> CardSwap<'a> {
    /// A cycling stack of `faces`.
    pub fn new(faces: &'a [CardFace]) -> Self {
        Self { faces, card_size: vec2(300.0, 210.0), distance: vec2(48.0, 40.0), skew: 6.0, interval: 3.2, pause_on_hover: true }
    }
    /// Card size in points.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Offset between consecutive cards (x right, y up).
    pub fn distance(mut self, d: Vec2) -> Self {
        self.distance = d;
        self
    }
    /// Vertical skew in degrees for the 3D look.
    pub fn skew(mut self, deg: f32) -> Self {
        self.skew = deg;
        self
    }
    /// Seconds between swaps.
    pub fn interval(mut self, s: f32) -> Self {
        self.interval = s.max(1.0);
        self
    }
    /// Pause cycling while hovered (default on).
    pub fn pause_on_hover(mut self, on: bool) -> Self {
        self.pause_on_hover = on;
        self
    }
}

const DROP: f32 = 0.55;
const RISE: f32 = 0.75;

impl egui::Widget for CardSwap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.faces.len();
        let span = self.distance * (n.max(1) - 1) as f32;
        let size = vec2(ui.available_width(), self.card_size.y + span.y + 70.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let paused = self.pause_on_hover && response.hovered();
        let front = rect.center() + vec2(-span.x / 2.0, span.y / 2.0);
        let slot_pos = |s: f32| front + vec2(s * self.distance.x, -s * self.distance.y);
        let interval = self.interval;
        let st = glowkit_core::with_state(ui, id.with("st"), SwapState::default, |st| {
            if st.order.len() != n {
                st.order = (0..n).collect();
                st.slots = (0..n).map(|i| Spring::new(i as f32).with(170.0, 22.0)).collect();
                st.last = now;
            }
            // freeze the timer while paused
            match (paused, st.paused_at) {
                (true, None) => st.paused_at = Some(now),
                (false, Some(p)) => {
                    st.last += now - p;
                    st.paused_at = None;
                }
                _ => {}
            }
            if st.swapping.is_none() && st.paused_at.is_none() && now - st.last >= interval && n > 1 {
                let f = st.order.remove(0);
                st.order.push(f);
                st.swapping = Some((f, now));
                st.last = now;
            }
            if let Some((_, t0)) = st.swapping {
                if now - t0 > DROP + RISE {
                    st.swapping = None;
                }
            }
            for (slot, &card) in st.order.iter().enumerate() {
                let s = &mut st.slots[card];
                s.target = slot as f32;
                if st.swapping.map(|(c, _)| c == card).unwrap_or(false) {
                    s.value = slot as f32;
                    s.velocity = 0.0;
                } else if let Some((_, t0)) = st.swapping {
                    // others wait for the drop to begin before moving forward
                    if now - t0 > DROP * 0.35 {
                        s.step(dt);
                    }
                } else {
                    s.step(dt);
                }
            }
            st.clone()
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect.expand(30.0));
        let mut draw: Vec<(f32, usize, CardPose)> = Vec::with_capacity(n);
        let skew = self.skew.to_radians().tan();
        for &card in &st.order {
            let s = st.slots[card].value;
            let mut pos = slot_pos(s);
            let mut z = s;
            if let Some((c, t0)) = st.swapping {
                if c == card {
                    let e = now - t0;
                    if e < DROP {
                        pos = slot_pos(0.0) + vec2(0.0, ease(Easing::InCubic, e / DROP) * (self.card_size.y + 260.0));
                        z = -1.0;
                    } else {
                        let p = ease(Easing::OutCubic, (e - DROP) / RISE);
                        pos = slot_pos((n - 1) as f32) + vec2(0.0, (1.0 - p) * (self.card_size.y + 200.0));
                        z = n as f32 + 1.0;
                    }
                }
            }
            let mut pose = CardPose::new(pos, self.card_size);
            pose.skew_y = skew;
            pose.scale = 1.0 - s * 0.03;
            pose.shadow = 0.9;
            draw.push((z, card, pose));
        }
        draw.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (z, card, pose) in draw {
            paint_card_face(&painter, &t, &self.faces[card], card, pose);
            // depth dimming
            let dim = (z.clamp(0.0, n as f32) * 0.12).min(0.45);
            if dim > 0.01 {
                let outline = pose.outline((t.radius * 1.6).min(pose.size.min_elem() * 0.2));
                let shade = if t.dark { Color32::BLACK } else { Color32::WHITE };
                painter.add(egui::Shape::convex_polygon(outline, shade.gamma_multiply(dim), egui::Stroke::NONE));
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A fan of cards that bounce in one after another; hovering a card straightens it and pushes
/// its neighbors aside.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{BounceCards, CardFace};
/// let faces = [CardFace::new("🌊", "Ocean"), CardFace::new("🌅", "Sunset"), CardFace::new("🌲", "Forest")];
/// ui.add(BounceCards::new(&faces));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BounceCards<'a> {
    faces: &'a [CardFace],
    card_size: Vec2,
    spacing: f32,
    push: f32,
    delay: f32,
    stagger: f32,
    trigger: Trigger,
    autoplay: bool,
}

impl<'a> BounceCards<'a> {
    /// A fan of `faces`.
    pub fn new(faces: &'a [CardFace]) -> Self {
        Self { faces, card_size: vec2(150.0, 190.0), spacing: 0.62, push: 70.0, delay: 0.2, stagger: 0.08, trigger: Trigger::OnAppear, autoplay: false }
    }
    /// Card size in points.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// Horizontal distance between card centers, as a fraction of the card width.
    pub fn spacing(mut self, f: f32) -> Self {
        self.spacing = f.clamp(0.1, 1.5);
        self
    }
    /// How far neighbors are pushed aside on hover.
    pub fn push(mut self, p: f32) -> Self {
        self.push = p;
        self
    }
    /// Delay before the first card bounces in, and stagger between cards (seconds).
    pub fn timing(mut self, delay: f32, stagger: f32) -> Self {
        self.delay = delay.max(0.0);
        self.stagger = stagger.max(0.0);
        self
    }
    /// When the bounce-in animation plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Cycle a simulated hover across the cards while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BounceCards<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.faces.len();
        let size = vec2(ui.available_width(), self.card_size.y + 90.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let duration = self.delay + self.stagger * n as f32 + 1.0;
        let elapsed = Clock::triggered(ui, id, self.trigger, duration, response.hovered(), response.clicked());
        let c = rect.center();
        let dx = self.card_size.x * self.spacing;
        const ANGLES: [f32; 8] = [10.0, 5.0, -3.0, -10.0, 2.0, 7.0, -6.0, 4.0];
        let base: Vec<(Vec2, f32)> = (0..n)
            .map(|i| {
                let x = (i as f32 - (n as f32 - 1.0) / 2.0) * dx;
                (vec2(x, (i % 2) as f32 * 6.0 - 3.0), ANGLES[i % ANGLES.len()])
            })
            .collect();
        // hit test against the resting fan, front-most first
        let local = |p: Pos2, i: usize| {
            let (off, ang) = base[i];
            let d = p - (c + off);
            let (s, co) = (-ang.to_radians()).sin_cos();
            vec2(d.x * co - d.y * s, d.x * s + d.y * co)
        };
        let mut hovered = response.hover_pos().and_then(|p| {
            (0..n).rev().find(|&i| {
                let l = local(p, i);
                l.x.abs() <= self.card_size.x / 2.0 && l.y.abs() <= self.card_size.y / 2.0
            })
        });
        if hovered.is_none() && self.autoplay && response.hover_pos().is_none() {
            let now = glowkit_core::time(ui);
            let slot = (now / 1.4).floor() as usize;
            if slot % 3 != 2 {
                hovered = Some((slot * 2 + slot / 3) % n);
            }
            glowkit_core::animate(ui);
        }
        let t = Theme::of(ui);
        let painter = ui.painter_at(rect.expand(30.0));
        let mut poses = Vec::with_capacity(n);
        for (i, &(mut off, mut ang)) in base.iter().enumerate() {
            if let Some(h) = hovered {
                if h == i {
                    ang = 0.0;
                    off.y -= 6.0;
                } else {
                    off.x += if i < h { -self.push } else { self.push };
                }
            }
            let ax = glowkit_core::anim::spring(ui, id.with(("x", i)), off.x, 220.0, 17.0);
            let ay = glowkit_core::anim::spring(ui, id.with(("y", i)), off.y, 220.0, 17.0);
            let aa = glowkit_core::anim::spring(ui, id.with(("a", i)), ang, 220.0, 17.0);
            let p = ((elapsed - self.delay - i as f32 * self.stagger) / 0.9).clamp(0.0, 1.0);
            let scale = ease(Easing::OutElastic, p);
            let mut pose = CardPose::new(c + vec2(ax, ay), self.card_size);
            pose.angle = aa.to_radians();
            pose.scale = scale * if hovered == Some(i) { 1.04 } else { 1.0 };
            pose.alpha = (p * 4.0).min(1.0);
            poses.push(pose);
        }
        for (i, pose) in poses.iter().enumerate() {
            if Some(i) != hovered {
                paint_card_face(&painter, &t, &self.faces[i], i, *pose);
            }
        }
        if let Some(h) = hovered {
            paint_card_face(&painter, &t, &self.faces[h], h, poses[h]);
        }
        response
    }
}

//! Banners that drop in from the top of the screen, queue up and can be flicked away.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use glowkit_effects::MicroIcon;

/// The content of one [`TopBanner`] message.
#[derive(Clone, Debug, PartialEq)]
pub struct BannerSpec {
    /// Bold first line.
    pub title: String,
    /// Optional second line (empty for none).
    pub description: String,
    /// Optional leading icon.
    pub icon: Option<MicroIcon>,
    /// Icon badge color.
    pub accent: Color32,
    /// Optional trailing action label.
    pub action: Option<String>,
}

impl BannerSpec {
    /// A title-only banner.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), description: String::new(), icon: None, accent: hexa(0x0A, 0x84, 0xFF), action: None }
    }
    /// Second line.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    /// Leading icon in a colored badge.
    pub fn icon(mut self, i: MicroIcon, accent: Color32) -> Self {
        self.icon = Some(i);
        self.accent = accent;
        self
    }
    /// Trailing action button label.
    pub fn action(mut self, a: impl Into<String>) -> Self {
        self.action = Some(a.into());
        self
    }
}

#[derive(Clone, Default)]
struct BannerState {
    queue: Vec<BannerSpec>,
    current: Option<BannerSpec>,
    elapsed: f32,
    pos: f32,
    pos_v: f32,
    drag: f32,
    leaving: bool,
    gap_until: f32,
    g: Gesture,
    grabbed: bool,
    cycle: u32,
    auto: u32,
    pressed_action: f32,
}

/// Short banners that slide down from the top of a screen, one at a time.
///
/// Push messages with [`TopBanner::show`]; they queue and appear one after another with a short
/// pause between them. Each drops in on a spring and hides itself after a few seconds. Grab it
/// and the timer pauses: drag up and it follows your finger, drag down and it stretches with
/// rubber-band resistance; flick it up (or drag it past a small distance) and it is dismissed,
/// otherwise it springs back. Banners can carry an icon badge, a second line and an action
/// button.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{TopBanner, BannerSpec};
/// ui.add(TopBanner::new("banners").duration(4.0));
/// if ui.button("Copy link").clicked() {
///     TopBanner::show(ui.ctx(), "banners", BannerSpec::new("Link copied"));
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TopBanner {
    key: Id,
    size: Option<Vec2>,
    duration: f32,
    bounce: f32,
    top: f32,
    autoplay: bool,
}

fn key_of(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::unique(id_salt).with("top_banner")
}

impl TopBanner {
    /// A banner host addressed by `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self { key: key_of(id_salt), size: None, duration: 4.0, bounce: 35.0, top: 44.0, autoplay: false }
    }
    /// Queue a banner on the host `id_salt`.
    pub fn show(ctx: &Context, id_salt: impl std::hash::Hash + std::fmt::Debug, banner: BannerSpec) {
        let k = key_of(id_salt);
        ctx.data_mut(|d| d.get_temp_mut_or_default::<BannerState>(k).queue.push(banner));
    }
    /// Dismiss the banner currently showing on `id_salt`.
    pub fn dismiss(ctx: &Context, id_salt: impl std::hash::Hash + std::fmt::Debug) {
        let k = key_of(id_salt);
        ctx.data_mut(|d| {
            let st = d.get_temp_mut_or_default::<BannerState>(k);
            if st.current.is_some() {
                st.leaving = true;
            }
        });
    }
    /// The area banners drop into (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Seconds before a banner hides itself.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.clamp(0.8, 20.0);
        self
    }
    /// Overshoot of the drop-in, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Distance from the top of the area to the banner (below a status bar).
    pub fn top(mut self, y: f32) -> Self {
        self.top = y.max(0.0);
        self
    }
    /// Post banners and flick them away on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SCRIPT: [Stroke2; 2] = [drag(2.6, 2.85, (0.5, 0.15), (0.52, 0.02)), drag(5.2, 6.1, (0.5, 0.15), (0.5, 0.3))];

impl egui::Widget for TopBanner {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let mut st: BannerState = glowkit_core::state(ui, self.key, BannerState::default);
        let h_of = |b: &BannerSpec| if b.description.is_empty() { 50.0 } else { 64.0 };
        let cur_h = st.current.as_ref().map(h_of).unwrap_or(60.0);
        let rest = Rect::from_min_size(pos2(rect.min.x + 10.0, rect.min.y + self.top), vec2(rect.width() - 20.0, cur_h));
        let hidden_dy = -(self.top + cur_h + 30.0);
        let banner_dy = hidden_dy * (1.0 - st.pos) + st.drag;
        let banner = rest.translate(vec2(0.0, banner_dy));
        // only the banner itself takes input, so content underneath stays clickable
        let hit = ui.interact(banner, self.key.with("hit"), Sense::click_and_drag());
        let touch = touch(ui, &hit, rect, self.autoplay, &SCRIPT, 9.0);
        if touch.ghost {
            if touch.cycle != st.cycle {
                st.cycle = touch.cycle;
                st.auto = 0;
            }
            let k = [0.3, 3.4, 3.7].iter().filter(|&&x| touch.t > x).count() as u32;
            while st.auto < k {
                st.queue.push(match st.auto {
                    0 => BannerSpec::new("Link copied").description("Anyone with the link can view.").icon(MicroIcon::Link, hexa(0x0A, 0x84, 0xFF)),
                    1 => BannerSpec::new("Back online").icon(MicroIcon::Refresh, hexa(0x34, 0xC7, 0x59)),
                    _ => BannerSpec::new("Photo saved").description("Added to Recents.").icon(MicroIcon::Folder, hexa(0xFF, 0x9F, 0x0A)).action("View"),
                });
                st.auto += 1;
            }
        }
        // next in queue
        if st.current.is_none() && !st.queue.is_empty() && now >= st.gap_until {
            st.current = Some(st.queue.remove(0));
            st.elapsed = 0.0;
            st.pos = 0.0;
            st.pos_v = 0.0;
            st.drag = 0.0;
            st.leaving = false;
        }
        let g = st.g.update(&touch, now, dt);
        if g.pressed && st.current.is_some() && banner.contains(st.g.start) {
            st.grabbed = true;
            if let Some(b) = &st.current {
                if b.action.is_some() && st.g.start.x > banner.max.x - 70.0 {
                    st.pressed_action = 1.0;
                }
            }
        }
        if st.grabbed && st.g.down && !g.pressed {
            let raw = st.g.last.y - st.g.start.y;
            st.drag = if raw < 0.0 { raw } else { rubber(raw, 34.0) };
        }
        if g.released && st.grabbed {
            st.grabbed = false;
            let up = st.drag < -24.0 || st.g.vel.y < -450.0;
            if up {
                st.leaving = true;
                st.pos_v = st.g.vel.y.min(0.0) / hidden_dy.abs();
                st.pos += st.drag / -hidden_dy;
                st.drag = 0.0;
            } else if g.tap && st.pressed_action > 0.0 {
                st.leaving = true;
            }
        }
        if st.current.is_some() && !st.grabbed && !st.leaving {
            st.elapsed += dt;
            if st.elapsed > self.duration {
                st.leaving = true;
            }
        }
        // physics
        let target = if st.current.is_some() && !st.leaving { 1.0 } else { 0.0 };
        let stiff = 260.0;
        spring_step(&mut st.pos, &mut st.pos_v, target, stiff, damping_for(if st.leaving { 0.0 } else { self.bounce }, stiff), dt);
        if !st.grabbed {
            st.drag *= (1.0 - dt * 16.0).max(0.0);
        }
        st.pressed_action = (st.pressed_action - dt * 3.0).max(0.0);
        if st.leaving && st.pos < 0.01 {
            st.current = None;
            st.leaving = false;
            st.gap_until = now + 0.35;
        }
        let active = st.current.is_some() || !st.queue.is_empty();
        let s = st.clone();
        glowkit_core::set_state(ui, self.key, st);
        if active || touch.ghost {
            ui.ctx().request_repaint();
        }

        // paint
        let painter = ui.painter_at(rect);
        if let Some(b) = &s.current {
            let h = h_of(b);
            let stretch = s.drag.max(0.0) * 0.6;
            let r = Rect::from_min_size(
                pos2(rest.min.x, rest.min.y + hidden_dy * (1.0 - s.pos) + s.drag.min(0.0) + s.drag.max(0.0) * 0.4),
                vec2(rest.width(), h + stretch),
            );
            let fill = if t.dark { hexa(0x26, 0x26, 0x2B) } else { Color32::WHITE };
            card(&painter, &t, r, 18.0, 1.1, fill);
            let mut x = r.min.x + 14.0;
            if let Some(icon) = b.icon {
                let c = pos2(x + 15.0, r.center().y);
                painter.circle_filled(c, 15.0, b.accent);
                icon.paint(&painter, c, 15.0, Color32::WHITE);
                x += 40.0;
            }
            if b.description.is_empty() {
                text_bold(&painter, pos2(x, r.center().y), Align2::LEFT_CENTER, &b.title, 14.0, t.foreground);
            } else {
                text_bold(&painter, pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, &b.title, 14.0, t.foreground);
                text(&painter, pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, &b.description, 12.0, t.muted_foreground);
            }
            if let Some(a) = &b.action {
                let w = text_width(&painter, a, 13.0) + 24.0;
                let br = Rect::from_center_size(pos2(r.max.x - 14.0 - w / 2.0, r.center().y), vec2(w, 30.0) * (1.0 - 0.06 * s.pressed_action));
                painter.rect_filled(br, cr(15.0), mix(recess(&t), system_blue(&t), 0.12));
                text_bold(&painter, br.center(), Align2::CENTER_CENTER, a, 13.0, system_blue(&t));
            }
            // a small grabber hint at the bottom while stretched
            if stretch > 1.0 {
                let gy = r.max.y - 7.0;
                painter.rect_filled(
                    Rect::from_center_size(pos2(r.center().x, gy), vec2(30.0, 4.0)),
                    cr(2.0),
                    with_alpha(t.muted_foreground, (stretch / 12.0).min(0.5)),
                );
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}

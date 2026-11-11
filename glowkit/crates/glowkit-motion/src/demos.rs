//! Gallery demos for the motion engine.

use crate::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};
use glowkit_core::color::{self, with_alpha};
use glowkit_core::demo::{Category::MotionEngine as C, Demo};
use glowkit_core::{paint, state, set_state, Rng, Theme};
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------------------------
// shared helpers

/// Allocate a full-width stage with a subtle dotted background.
fn stage(ui: &mut Ui, h: f32) -> (Rect, egui::Response) {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, t.corner(), t.muted);
    let dot = with_alpha(t.muted_foreground, if t.dark { 0.13 } else { 0.18 });
    let step = 22.0;
    let mut y = rect.min.y + step / 2.0;
    while y < rect.max.y {
        let mut x = rect.min.x + step / 2.0;
        while x < rect.max.x {
            p.circle_filled(pos2(x, y), 0.9, dot);
            x += step;
        }
        y += step;
    }
    p.rect_stroke(rect, t.corner(), t.border_stroke(), StrokeKind::Inside);
    (rect, resp)
}

/// Small caption text.
fn caption(ui: &Ui, pos: Pos2, align: Align2, text: &str) {
    let t = Theme::of(ui);
    ui.painter().text(pos, align, text, FontId::proportional(11.5), t.muted_foreground);
}

/// A card-like rounded rect with a soft shadow.
fn card(painter: &egui::Painter, t: &Theme, rect: Rect, radius: f32) {
    paint::soft_shadow(painter, rect.translate(vec2(0.0, 4.0)), radius, 16.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 110 } else { 28 }));
    painter.rect_filled(rect, radius, t.card);
    painter.rect_stroke(rect, radius, t.border_stroke(), StrokeKind::Inside);
}

/// Seconds since the user last touched `rect` (pointer down inside); used to pause autoplay.
fn idle_for(ui: &Ui, id: Id, rect: Rect) -> f32 {
    let now = ui.input(|i| i.time);
    let key = id.with("demo_last_touch");
    let touched = ui.input(|i| i.pointer.primary_down() && i.pointer.latest_pos().is_some_and(|p| rect.contains(p)));
    if touched {
        set_state(ui, key, now);
    }
    let last: f64 = state(ui, key, || -100.0);
    (now - last) as f32
}

fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        add(ui);
    });
    ui.add_space(8.0);
}

// ---------------------------------------------------------------------------------------------

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "spring-playground",
            name: "Spring Playground",
            category: C,
            description: "Tune stiffness, damping and mass and watch a ball spring after the pointer (or a hopping target), with its step response plotted live.",
            covers: &["Framer Motion: useSpring", "Motion One: spring()", "React Spring: useSpring", "GSAP: elastic ease"],
            code: r##"let pos: Pos2 = animate(ui, id, target, Transition::spring(stiffness, damping).mass(mass));
// any Animatable works: f32, Vec2, Pos2, Rect, Color32, Props, or your own type"##,
            show: spring_playground,
        },
        Demo {
            id: "easing-visualizer",
            name: "Easing Visualizer",
            category: C,
            description: "Every classic easing curve plotted with a dot riding along it, so you can feel the difference.",
            covers: &["easings.net", "GSAP: Ease Visualizer", "Framer Motion: easing functions"],
            code: r##"let x = animate(ui, id, target, Transition::tween(0.6).ease(Easing::OutBack));
let k = Easing::InOutCubic.apply(progress);"##,
            show: easing_visualizer,
        },
        Demo {
            id: "keyframes",
            name: "Keyframes",
            category: C,
            description: "Values passing through several stops with per-segment easing, infinite repeats, yoyo vs mirror, and repeat delays.",
            covers: &["Framer Motion: keyframes", "Motion One: keyframes", "CSS: @keyframes", "GSAP: keyframes"],
            code: r##"let y = Keyframes::new([(0.0, 0.0_f32), (0.5, -120.0), (1.0, 0.0)])
    .ease_segments([Easing::OutQuad, Easing::InQuad])
    .duration(1.1)
    .repeat(Repeat::Infinite)
    .play(ui, id);
let x = Keyframes::evenly([0.0_f32, 0.8, 1.0]).repeat(Repeat::Infinite).mirror().repeat_delay(0.3).play(ui, id2);"##,
            show: keyframes_demo,
        },
        Demo {
            id: "timeline",
            name: "Timeline",
            category: C,
            description: "A GSAP-style timeline: tweens sequenced with relative positions, labels and overlaps; play, pause, reverse, change speed or scrub.",
            covers: &["GSAP: Timeline", "Motion One: timeline()", "Anime.js: timeline", "Framer Motion: animate sequences"],
            code: r##"let tl = Timeline::new()
    .add(Tween::new("x", -1.0_f32, 0.0).duration(0.8).ease(Easing::OutBack), ())
    .label("spin")
    .add(Tween::new("rotate", 0.0_f32, 180.0).duration(0.7), "<0.3")  // 0.3s after previous start
    .add(Tween::new("pop", 0.0_f32, 1.0).duration(0.5).ease(Easing::OutBack), "-=0.2") // overlap
    .add(Tween::new("title", 0.0_f32, 1.0), "+=0.1");                  // gap
let pb = Playback::update_with(ui, id, tl.duration(), || Playback::new().looping(true, 0.6));
let x: f32 = tl.value_or("x", pb.time, 0.0);
let mut pb2 = pb;
ui.add(Scrubber::new(&tl, &mut pb2));"##,
            show: timeline_demo,
        },
        Demo {
            id: "motion-box",
            name: "Motion Box",
            category: C,
            description: "A container like motion.div: enter from initial props, hover and tap states, and spring to new targets. Its children stay fully interactive.",
            covers: &["Framer Motion: motion.div", "Framer Motion: whileHover / whileTap", "Motion One: hover / press"],
            code: r##"MotionBox::new(id)
    .initial(Props::new().opacity(0.0).y(30.0).scale(0.9))
    .animate(Props::IDENTITY)
    .while_hover(Props::new().scale(1.06).y(-6.0))
    .while_tap(Props::new().scale(0.94))
    .transition(Transition::BOUNCY)
    .show(ui, |ui| { ui.button("Still clickable"); });"##,
            show: motion_box_demo,
        },
        Demo {
            id: "stagger-variants",
            name: "Variants & Stagger",
            category: C,
            description: "Switch one variant name and every child follows, staggered one after another (from the first, or from the center outward).",
            covers: &["Framer Motion: variants", "Framer Motion: staggerChildren / delayChildren", "Motion One: stagger()", "GSAP: stagger"],
            code: r##"MotionGroup::new(id, Variants::fade(vec2(-24.0, 0.0)))
    .initial("hidden")
    .animate(if open { "visible" } else { "hidden" })
    .stagger_children(0.06)
    .delay_children(0.1)
    .show(ui, |group, ui| {
        for row in &rows {
            group.child(ui, |ui| { ui.label(row); });
        }
    });"##,
            show: stagger_demo,
        },
        Demo {
            id: "presence",
            name: "Presence",
            category: C,
            description: "Content that animates out before it disappears: auto-dismissing toasts and a keyed list where items enter, exit and the rest slide into place.",
            covers: &["Framer Motion: AnimatePresence", "Framer Motion: exit animations", "Sonner: toast stack", "React Transition Group"],
            code: r##"Presence::new(id, visible)
    .initial(Props::new().opacity(0.0).y(-12.0))
    .exit(Props::new().opacity(0.0).x(40.0))
    .show(ui, |ui| ui.label("Saved!"));

PresenceList::new(id)
    .initial(Props::new().opacity(0.0).x(-30.0))
    .exit(Props::new().opacity(0.0).scale(0.8))
    .show(ui, &items, |item| item.id, |ui, item| { ui.label(&item.name); });"##,
            show: presence_demo,
        },
        Demo {
            id: "layout-animation",
            name: "Layout Animation",
            category: C,
            description: "Items glide between layouts (grid ↔ list, shuffle) instead of jumping: FLIP-style layout animation keyed by identity.",
            covers: &["Framer Motion: layout prop", "Auto Animate", "GSAP: Flip plugin", "React Flip Toolkit"],
            code: r##"LayoutAnim::new(id).show(ui, |layout, ui| {
    for (i, item) in items.iter().enumerate() {
        let target = if grid { grid_cell(i) } else { list_row(i) };
        layout.item_at(ui, item.id, target, |ui| draw_item(ui, item));
    }
});"##,
            show: layout_demo,
        },
        Demo {
            id: "shared-element",
            name: "Shared Element",
            category: C,
            description: "A thumbnail expands into a detail view and back: the same key used in two places animates between them (layoutId).",
            covers: &["Framer Motion: layoutId", "Framer Motion: shared layout animations", "View Transitions API", "Aceternity: Expandable Card"],
            code: r##"LayoutAnim::new(id).show(ui, |layout, ui| {
    for (i, album) in albums.iter().enumerate() {
        // the same key in the grid or in the detail view: it flies between them
        let rect = if open == Some(i) { detail_rect } else { grid_rect(i) };
        if layout.item_at(ui, i, rect, |ui| cover(ui, album)).clicked() {
            open = if open == Some(i) { None } else { Some(i) };
        }
    }
});"##,
            show: shared_element_demo,
        },
        Demo {
            id: "drag",
            name: "Drag & Snap",
            category: C,
            description: "Throw a puck around with momentum and elastic edges, snap a knob to stops along an axis, and pull a card that springs home.",
            covers: &["Framer Motion: drag", "Framer Motion: dragConstraints / dragElastic / dragMomentum", "GSAP: Draggable + Inertia", "React Use Gesture"],
            code: r##"Draggable::new(id)
    .constraints(area)          // stay inside
    .elastic(0.35)              // rubber-band past the edges
    .momentum(true)             // glide on release, bounce back in
    .while_drag(Props::new().scale(1.12))
    .show(ui, |ui| puck(ui));

Draggable::new(id2).axis(Axis::X).snap_points([vec2(-120.0, 0.0), Vec2::ZERO, vec2(120.0, 0.0)]).show(ui, knob);"##,
            show: drag_demo,
        },
        Demo {
            id: "reorder",
            name: "Reorder List",
            category: C,
            description: "Drag rows to reorder them; the others slide out of the way and the dragged row lifts above them.",
            covers: &["Framer Motion: Reorder.Group", "dnd kit: Sortable", "SortableJS", "React Beautiful DnD"],
            code: r##"Reorder::new(id).item_height(52.0).show(ui, &mut tasks, |t| t.id, |ui, task, state| {
    draw_row(ui, task, state.dragging);
});"##,
            show: reorder_demo,
        },
        Demo {
            id: "scroll-linked",
            name: "Scroll-Linked Motion",
            category: C,
            description: "A progress bar tied to scroll position, parallax layers at different speeds, values mapped with transform(), and cards that animate in when scrolled into view.",
            covers: &["Framer Motion: useScroll / useTransform", "Framer Motion: whileInView", "GSAP: ScrollTrigger", "Aceternity: Parallax Scroll"],
            code: r##"let out = ScrollArea::vertical().show(ui, |ui| {
    let scroll = scroll_offset(ui).y;
    let y = parallax(scroll, 0.5);                       // half speed
    let hue: Color32 = transform(scroll, &[0.0, 400.0], &[a, b]);
    let seen = InView::new(id).amount(0.4).check(ui, card_rect);
});
let progress = scroll_progress(&out).y;"##,
            show: scroll_demo,
        },
        Demo {
            id: "motion-path",
            name: "Motion Path",
            category: C,
            description: "Objects travel along a smooth path, turning to follow its tangent, with a comet trail and a staggered convoy.",
            covers: &["GSAP: MotionPathPlugin", "CSS: offset-path", "Framer Motion: path animations"],
            code: r##"let path = MotionPath::smooth(&points, true);
let p = path.sample(progress);            // position + tangent angle
draw_arrow(painter, p.pos, p.angle);
path.paint_range(painter, progress - 0.15, progress, 4.0, |k| with_alpha(color, k));"##,
            show: motion_path_demo,
        },
        Demo {
            id: "path-drawing",
            name: "Path Drawing",
            category: C,
            description: "Any polyline drawn progressively with a gradient stroke and a glowing pen tip, then erased: like animating SVG pathLength.",
            covers: &["Framer Motion: pathLength", "GSAP: DrawSVG", "Vivus", "Magic UI: Animated Beam (drawing)"],
            code: r##"let path = MotionPath::smooth(&points, false);
let k = Keyframes::new([(0.0, 0.0_f32), (0.6, 1.0), (1.0, 1.0)]).repeat(Repeat::Infinite).play(ui, id);
path.paint_progress(painter, k, 4.0, |t| gradient(&colors, t));"##,
            show: path_drawing_demo,
        },
        Demo {
            id: "lanyard",
            name: "Lanyard",
            category: C,
            description: "An ID badge hanging on a strap simulated with a verlet rope. Grab it, swing it, fling it.",
            covers: &["React Bits: Lanyard", "Vercel Ship badge"],
            code: r##"Lanyard::new(id).card_size(vec2(170.0, 220.0)).rope_length(150.0).show(ui, |ui| {
    ui.heading("Alex Rivera");
    ui.label("Motion Engineer");
});"##,
            show: lanyard_demo,
        },
        Demo {
            id: "shape-morph",
            name: "Shape Morph",
            category: C,
            description: "Outlines morph smoothly between circle, square, star, heart and blob: resampled, aligned and interpolated point by point.",
            covers: &["GSAP: MorphSVG", "Flubber", "Framer Motion: path morphing"],
            code: r##"ui.add(MorphShape::new().size(220.0));
let mid = morph_polygon(&star, &heart, t);   // any two polygons"##,
            show: morph_demo,
        },
        Demo {
            id: "spring-chain",
            name: "Spring Chain",
            category: C,
            description: "A follow-the-leader chain of springs: each dot chases the one in front, making whippy trails behind the pointer.",
            covers: &["React Bits: Splash/Blob Cursor", "Framer Motion: useSpring chains", "Codrops: elastic trails"],
            code: r##"let mut chain = SpringChain::new(20, start).stiffness(320.0).damping(20.0).falloff(0.93);
chain.update(pointer, dt);
for (i, p) in chain.points().iter().enumerate() { painter.circle_filled(*p, 14.0 - i as f32 * 0.6, color); }"##,
            show: spring_chain_demo,
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// 1. spring playground

#[derive(Clone, Copy)]
struct SpringParams {
    k: f32,
    d: f32,
    m: f32,
    hop_at: f64,
}

fn spring_playground(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("spring_playground");
    let mut s = state(ui, id, || SpringParams { k: 180.0, d: 10.0, m: 1.0, hop_at: 0.0 });
    toolbar(ui, |ui| {
        ui.add(egui::Slider::new(&mut s.k, 20.0..=600.0).text("stiffness").max_decimals(0));
        ui.add(egui::Slider::new(&mut s.d, 1.0..=60.0).text("damping").max_decimals(1));
        ui.add(egui::Slider::new(&mut s.m, 0.2..=4.0).text("mass").max_decimals(2));
        ui.separator();
        for (name, k, d, m) in [("Bouncy", 280.0, 9.0, 1.0), ("Snappy", 420.0, 34.0, 1.0), ("Gentle", 90.0, 19.0, 1.0), ("Heavy", 160.0, 14.0, 3.0)] {
            if ui.button(name).clicked() {
                (s.k, s.d, s.m) = (k, d, m);
            }
        }
    });
    let (rect, resp) = stage(ui, 300.0);
    let split = rect.min.x + rect.width() * 0.66;
    let area = Rect::from_min_max(rect.min, pos2(split, rect.max.y)).shrink(30.0);
    let chart = Rect::from_min_max(pos2(split, rect.min.y), rect.max).shrink2(vec2(22.0, 34.0));
    let now = ui.input(|i| i.time);

    // target: the pointer while hovering, otherwise hop around a ring
    let hops = 5;
    let period = 1.4;
    let hop = ((now / period).floor() as i64).rem_euclid(hops) as f32;
    let auto = area.center() + vec2((hop * 2.4).cos() * area.width() * 0.38, (hop * 2.4).sin() * area.height() * 0.4);
    let pointer = resp.hover_pos().filter(|p| area.expand(30.0).contains(*p));
    let target = pointer.unwrap_or(auto);
    if pointer.is_none() {
        ui.ctx().request_repaint();
    }
    let rel = target - area.min;
    let tr = Transition::spring(s.k, s.d).mass(s.m);
    let pos = area.min + animate(ui, id.with("ball"), rel, tr);
    let vel = motion_value::<Vec2>(ui, id.with("ball")).map_or(Vec2::ZERO, |m| m.velocity());
    let hop_now = (now / period).floor();
    if pointer.is_none() && hop_now != s.hop_at {
        s.hop_at = hop_now;
    }
    set_state(ui, id, s);

    let p = ui.painter_at(rect);
    // target marker
    p.circle_stroke(target, 14.0, Stroke::new(1.5, with_alpha(t.accent, 0.6)));
    p.line_segment([target - vec2(6.0, 0.0), target + vec2(6.0, 0.0)], Stroke::new(1.5, with_alpha(t.accent, 0.6)));
    p.line_segment([target - vec2(0.0, 6.0), target + vec2(0.0, 6.0)], Stroke::new(1.5, with_alpha(t.accent, 0.6)));
    // a coil between target and ball
    let d = pos - target;
    let len = d.length();
    if len > 2.0 {
        let dir = d / len;
        let nrm = dir.rot90();
        let coils = 10;
        let mut pts = vec![target];
        for i in 1..coils * 2 {
            let k = i as f32 / (coils * 2) as f32;
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            pts.push(target + dir * len * k + nrm * side * 6.0);
        }
        pts.push(pos);
        p.add(egui::Shape::line(pts, Stroke::new(1.5, with_alpha(t.muted_foreground, 0.7))));
    }
    // ball, stretched a little along its velocity
    let speed = vel.length();
    paint::glow(&p, pos, 46.0 + speed * 0.02, with_alpha(t.gradient[0], if t.dark { 0.35 } else { 0.2 }));
    paint::soft_shadow(&p, Rect::from_center_size(pos + vec2(0.0, 26.0), vec2(36.0, 6.0)), 3.0, 10.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 120 } else { 40 }));
    paint::gradient_circle(&p, pos, 20.0, &[color::shade(t.gradient[1], 0.12), t.gradient[0]]);
    p.circle_filled(pos - vec2(6.0, 7.0), 5.0, with_alpha(Color32::WHITE, 0.35));
    caption(ui, area.left_top() - vec2(10.0, 18.0), Align2::LEFT_TOP, "hover to steer the target");

    // step response chart
    p.line_segment([pos2(split, rect.min.y + 16.0), pos2(split, rect.max.y - 16.0)], Stroke::new(1.0, t.border));
    caption(ui, chart.left_top() - vec2(0.0, 22.0), Align2::LEFT_TOP, "step response (2 s)");
    let mut sp = glowkit_core::Spring { value: 0.0, velocity: 0.0, target: 1.0, stiffness: s.k, damping: s.d, mass: s.m };
    let n = 120;
    let mut max_v: f32 = 1.0;
    let vals: Vec<f32> = (0..=n)
        .map(|i| {
            if i > 0 {
                sp.step(2.0 / n as f32);
            }
            max_v = max_v.max(sp.value);
            sp.value
        })
        .collect();
    let max_v = max_v.min(2.2);
    let map = |i: usize, v: f32| pos2(chart.min.x + chart.width() * i as f32 / n as f32, chart.max.y - chart.height() * (v / max_v).clamp(-0.2, 1.0));
    let one = map(0, 1.0).y;
    p.line_segment([pos2(chart.min.x, chart.max.y), pos2(chart.max.x, chart.max.y)], Stroke::new(1.0, t.border));
    let mut x = chart.min.x;
    while x < chart.max.x {
        p.line_segment([pos2(x, one), pos2((x + 4.0).min(chart.max.x), one)], Stroke::new(1.0, with_alpha(t.muted_foreground, 0.5)));
        x += 8.0;
    }
    let pts: Vec<Pos2> = vals.iter().enumerate().map(|(i, v)| map(i, *v)).collect();
    let grad = t.gradient;
    paint::gradient_polyline(&p, &pts, false, 2.5, |k| color::gradient(&grad, k));
    // a dot replaying the response
    let since = ((now % 2.6) / 2.0).min(1.0) as f32;
    let idx = ((since * n as f32) as usize).min(n);
    p.circle_filled(pts[idx], 5.0, t.foreground);
    p.circle_filled(pts[idx], 2.5, t.gradient[1]);
    let zeta = s.d / (2.0 * (s.k * s.m).sqrt());
    let label = if zeta < 0.999 { format!("ζ = {zeta:.2}  underdamped") } else if zeta < 1.05 { format!("ζ = {zeta:.2}  critical") } else { format!("ζ = {zeta:.2}  overdamped") };
    caption(ui, chart.left_bottom() + vec2(0.0, 10.0), Align2::LEFT_TOP, &label);
    ui.ctx().request_repaint();
}

// ---------------------------------------------------------------------------------------------
// 2. easing visualizer

fn easing_visualizer(ui: &mut Ui) {
    let t = Theme::of(ui);
    let list = [
        (Easing::Linear, "Linear"),
        (Easing::OutQuad, "OutQuad"),
        (Easing::InOutCubic, "InOutCubic"),
        (Easing::OutQuart, "OutQuart"),
        (Easing::InOutSine, "InOutSine"),
        (Easing::OutExpo, "OutExpo"),
        (Easing::InOutCirc, "InOutCirc"),
        (Easing::InBack, "InBack"),
        (Easing::OutBack, "OutBack"),
        (Easing::InOutBack, "InOutBack"),
        (Easing::OutElastic, "OutElastic"),
        (Easing::OutBounce, "OutBounce"),
    ];
    let w = ui.available_width();
    let cols = ((w / 160.0).floor() as usize).clamp(3, 6);
    let rows = list.len().div_ceil(cols);
    let gap = 12.0;
    let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let ch = 118.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (ch + gap) - gap), Sense::hover());
    let p = ui.painter_at(rect.expand(8.0));
    let time = glowkit_core::time(ui);
    let cycle = 2.4;
    let prog = ((time % cycle) / 1.7).min(1.0);
    for (i, (e, name)) in list.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let cell = Rect::from_min_size(pos2(rect.min.x + c as f32 * (cw + gap), rect.min.y + r as f32 * (ch + gap)), vec2(cw, ch));
        p.rect_filled(cell, t.corner(), t.muted);
        p.rect_stroke(cell, t.corner(), t.border_stroke(), StrokeKind::Inside);
        let plot = Rect::from_min_max(cell.min + vec2(14.0, 30.0), cell.max - vec2(30.0, 14.0));
        p.text(cell.min + vec2(12.0, 9.0), Align2::LEFT_TOP, *name, FontId::monospace(11.5), t.foreground);
        // plot frame (with room for overshoot)
        let y_of = |v: f32| plot.max.y - (v * 0.72 + 0.14) * plot.height();
        for v in [0.0, 1.0] {
            p.line_segment([pos2(plot.min.x, y_of(v)), pos2(plot.max.x, y_of(v))], Stroke::new(1.0, with_alpha(t.border, 0.9)));
        }
        let pts: Vec<Pos2> = (0..=48).map(|k| {
            let x = k as f32 / 48.0;
            pos2(plot.min.x + x * plot.width(), y_of(e.apply(x)))
        }).collect();
        let col = t.chart[i % t.chart.len()];
        paint::gradient_polyline(&p, &pts, false, 2.0, |k| with_alpha(col, 0.35 + 0.65 * k));
        let v = e.apply(prog);
        let dot = pos2(plot.min.x + prog * plot.width(), y_of(v));
        p.circle_filled(dot, 7.0, with_alpha(col, 0.25));
        p.circle_filled(dot, 4.0, col);
        // value rail on the right
        let rail_x = cell.max.x - 15.0;
        p.line_segment([pos2(rail_x, y_of(0.0)), pos2(rail_x, y_of(1.0))], Stroke::new(3.0, with_alpha(t.border, 0.9)));
        p.circle_filled(pos2(rail_x, y_of(v)), 5.0, t.foreground);
    }
    glowkit_core::animate(ui);
}

// ---------------------------------------------------------------------------------------------
// 3. keyframes

fn keyframes_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("keyframes");
    let (rect, _) = stage(ui, 320.0);
    let p = ui.painter_at(rect);
    let left = Rect::from_min_max(rect.min, pos2(rect.min.x + rect.width() * 0.38, rect.max.y));
    // bouncing ball: squash & stretch with per-segment easing
    let y: f32 = Keyframes::new([(0.0, 0.0), (0.5, -150.0), (1.0, 0.0)])
        .ease_segments([Easing::OutQuad, Easing::InQuad])
        .duration(1.1)
        .repeat(Repeat::Infinite)
        .play(ui, id.with("y"));
    let squash: Vec2 = Keyframes::new([(0.0, vec2(1.3, 0.72)), (0.1, vec2(0.92, 1.1)), (0.5, vec2(1.0, 1.0)), (0.9, vec2(0.95, 1.08)), (1.0, vec2(1.3, 0.72))])
        .ease(Easing::OutQuad)
        .duration(1.1)
        .repeat(Repeat::Infinite)
        .play(ui, id.with("sq"));
    let hue: Color32 = Keyframes::evenly([t.chart[0], t.chart[1], t.chart[2], t.chart[0]]).ease(Easing::Linear).duration(3.3).repeat(Repeat::Infinite).play(ui, id.with("hue"));
    let floor = left.max.y - 52.0;
    let r = 26.0;
    let c = pos2(left.center().x, floor - r * squash.y + y);
    let h = (-y / 150.0).clamp(0.0, 1.0);
    p.add(egui::Shape::ellipse_filled(pos2(c.x, floor + 3.0), vec2(30.0 - 12.0 * h, 5.0 - 2.0 * h), Color32::from_black_alpha(if t.dark { 140 } else { 45 })));
    p.line_segment([pos2(left.min.x + 40.0, floor + 3.0), pos2(left.max.x - 40.0, floor + 3.0)], Stroke::new(1.0, t.border));
    paint::glow(&p, c, 70.0, with_alpha(hue, 0.25));
    p.add(egui::Shape::ellipse_filled(c, vec2(r * squash.x, r * squash.y), hue));
    p.add(egui::Shape::ellipse_filled(c - vec2(8.0, 9.0) * squash, vec2(7.0, 5.0), with_alpha(Color32::WHITE, 0.35)));
    caption(ui, pos2(left.center().x, left.max.y - 30.0), Align2::CENTER_TOP, "y, squash & color keyframes");

    // repeat types side by side
    let right = Rect::from_min_max(pos2(left.max.x, rect.min.y), rect.max).shrink2(vec2(36.0, 44.0));
    p.line_segment([pos2(left.max.x, rect.min.y + 20.0), pos2(left.max.x, rect.max.y - 20.0)], Stroke::new(1.0, t.border));
    let frames = [(0.0, 0.0_f32), (0.35, 0.75), (1.0, 1.0)];
    let rows: [(&str, RepeatType); 3] = [("loop", RepeatType::Loop), ("yoyo", RepeatType::Yoyo), ("mirror", RepeatType::Mirror)];
    for (i, (name, rt)) in rows.iter().enumerate() {
        let yy = right.min.y + 18.0 + i as f32 * (right.height() - 36.0) / 2.0;
        let x0 = right.min.x + 70.0;
        let x1 = right.max.x - 10.0;
        // sample one iteration ahead so the repeat types already differ on the first run
        let kf = Keyframes::new(frames).ease_segments([Easing::OutCubic, Easing::InOutSine]).duration(1.4).repeat(Repeat::Infinite).repeat_type(*rt).repeat_delay(0.35);
        let k: f32 = kf.sample(glowkit_core::Clock::elapsed(ui, id.with("rows")) + 1.75);
        let col = t.chart[(i + 3) % t.chart.len()];
        p.text(pos2(right.min.x, yy), Align2::LEFT_CENTER, *name, FontId::monospace(12.0), t.foreground);
        p.line_segment([pos2(x0, yy), pos2(x1, yy)], Stroke::new(4.0, with_alpha(t.border, 1.0)));
        let x = x0 + (x1 - x0) * k;
        p.line_segment([pos2(x0, yy), pos2(x, yy)], Stroke::new(4.0, with_alpha(col, 0.55)));
        for (o, _) in frames {
            let kx = x0 + (x1 - x0) * Keyframes::new(frames).ease(Easing::Linear).sample_progress(o);
            p.circle_filled(pos2(kx, yy), 3.0, t.muted_foreground);
        }
        p.circle_filled(pos2(x, yy), 11.0, with_alpha(col, 0.25));
        p.circle_filled(pos2(x, yy), 7.0, col);
    }
    caption(ui, pos2(right.center().x, rect.max.y - 26.0), Align2::CENTER_TOP, "same keyframes, three repeat types (dots = stops)");
    glowkit_core::animate(ui);
}

// ---------------------------------------------------------------------------------------------
// 4. timeline

fn demo_timeline() -> Timeline {
    Timeline::new()
        .add(Tween::new("slide", -1.0_f32, 0.0).duration(0.8).ease(Easing::OutBack).name("slide in"), ())
        .label("spin")
        .add(Tween::new("rotate", 0.0_f32, 180.0).duration(0.7).ease(Easing::InOutCubic).name("rotate"), "<0.35")
        .add(Tween::new("pop", 0.0_f32, 1.0).duration(0.55).ease(Easing::OutBack).name("pop"), "-=0.25")
        .add(Tween::new("orbit", 0.0_f32, 1.0).duration(1.0).ease(Easing::InOutSine).name("orbit"), "<0.1")
        .add(Tween::new("title", 0.0_f32, 1.0).duration(0.5).name("title"), "+=0.05")
        .add(Tween::new("bar", 0.0_f32, 1.0).duration(0.7).ease(Easing::OutCubic).name("bar"), "<")
}

fn timeline_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("timeline_demo");
    let tl = demo_timeline();
    let dur = tl.duration();
    let pb = Playback::update_with(ui, id, dur, || Playback::new().looping(true, 0.7));
    toolbar(ui, |ui| {
        if ui.button(if pb.playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
            Playback::modify(ui, id, |p| p.toggle());
        }
        if ui.button("⇄ Reverse").clicked() {
            Playback::modify(ui, id, |p| p.reverse());
        }
        if ui.button("⏮ Restart").clicked() {
            Playback::modify(ui, id, |p| p.restart());
        }
        ui.separator();
        for sp in [0.5_f32, 1.0, 2.0] {
            if ui.add(egui::Button::selectable((pb.speed - sp).abs() < 1e-3, format!("{sp}×"))).clicked() {
                Playback::modify(ui, id, |p| p.speed = sp);
            }
        }
        ui.separator();
        let mut looping = pb.looping;
        if ui.checkbox(&mut looping, "loop (yoyo)").changed() {
            Playback::modify(ui, id, |p| {
                p.looping = looping;
                p.yoyo = looping;
            });
        }
        ui.label(egui::RichText::new(format!("{:.2}s / {:.2}s", pb.time, dur)).monospace().color(t.muted_foreground));
    });
    let time = pb.time;
    let (rect, _) = stage(ui, 210.0);
    let p = ui.painter_at(rect);
    let slide: f32 = tl.value_or("slide", time, -1.0);
    let rot: f32 = tl.value_or("rotate", time, 0.0);
    let pop: f32 = tl.value_or("pop", time, 0.0);
    let orbit: f32 = tl.value_or("orbit", time, 0.0);
    let title: f32 = tl.value_or("title", time, 0.0);
    let bar: f32 = tl.value_or("bar", time, 0.0);
    let cx = rect.min.x + rect.width() * 0.3;
    let cy = rect.center().y - 10.0;
    // square: slides in, then rotates
    let sq_c = pos2(cx + slide * (cx - rect.min.x + 60.0), cy);
    let sq = egui::epaint::RectShape::filled(Rect::from_center_size(sq_c, Vec2::splat(74.0)), 16, t.chart[0]).with_angle(rot.to_radians());
    paint::glow(&p, sq_c, 80.0, with_alpha(t.chart[0], 0.25));
    p.add(sq);
    // circle: pops in and orbits the square
    if pop > 0.001 {
        let a = -TAU * 0.25 + orbit * TAU;
        let oc = sq_c + vec2(a.cos(), a.sin()) * 78.0;
        p.circle_filled(oc, 16.0 * pop, t.chart[2]);
        p.circle_filled(oc, 7.0 * pop, with_alpha(Color32::WHITE, 0.6));
    }
    // title fades & rises, bar fills
    let tx = rect.min.x + rect.width() * 0.55;
    let col = with_alpha(t.foreground, title);
    p.text(pos2(tx, cy - 26.0 + (1.0 - title) * 14.0), Align2::LEFT_CENTER, "Sequenced with a timeline", FontId::proportional(24.0), col);
    p.text(pos2(tx, cy + 6.0 + (1.0 - title) * 14.0), Align2::LEFT_CENTER, "labels · overlaps · reverse · scrub", FontId::proportional(14.0), with_alpha(t.muted_foreground, title));
    let track = Rect::from_min_size(pos2(tx, cy + 36.0), vec2((rect.max.x - tx - 40.0).max(60.0), 8.0));
    p.rect_filled(track, 4, t.border);
    let grad = t.gradient;
    paint::gradient_rect(&p, Rect::from_min_size(track.min, vec2(track.width() * bar, track.height())), 4.0, 0.0, &grad);
    ui.add_space(10.0);
    let mut pb2 = pb;
    ui.add(Scrubber::new(&tl, &mut pb2));
    if pb2 != pb {
        Playback::modify(ui, id, |p| *p = pb2);
    }
}

// ---------------------------------------------------------------------------------------------
// 5. motion box

fn motion_card(ui: &mut Ui, w: f32, icon: &str, title: &str, sub: &str, accent: Color32) {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 150.0), Sense::hover());
    let p = ui.painter();
    card(p, &t, rect, 16.0);
    let ic = Rect::from_min_size(rect.min + vec2(18.0, 18.0), vec2(42.0, 42.0));
    p.rect_filled(ic, 12, with_alpha(accent, 0.16));
    p.text(ic.center(), Align2::CENTER_CENTER, icon, FontId::proportional(20.0), accent);
    p.text(rect.min + vec2(18.0, 78.0), Align2::LEFT_TOP, title, FontId::proportional(16.0), t.foreground);
    p.text(rect.min + vec2(18.0, 102.0), Align2::LEFT_TOP, sub, FontId::proportional(12.0), t.muted_foreground);
}

fn motion_box_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("motion_box_demo");
    let mut replay = false;
    toolbar(ui, |ui| {
        replay = ui.button("⟲ Replay enter").clicked();
        ui.label(egui::RichText::new("hover and press the cards").color(t.muted_foreground));
    });
    let (rect, _) = stage(ui, 260.0);
    let n = 4;
    let gap = 22.0;
    let w = ((rect.width() - 60.0 - gap * 3.0) / n as f32).clamp(120.0, 200.0);
    let total = w * n as f32 + gap * (n - 1) as f32;
    let start = pos2(rect.center().x - total / 2.0, rect.center().y - 75.0);
    let now = ui.input(|i| i.time);
    let breathe = ((now / 1.6) as i64) % 2 == 0;
    let cards: [(&str, &str, &str, Props, Props); 4] = [
        ("↑", "Hover lift", "while_hover", Props::new().scale(1.06).y(-8.0), Props::IDENTITY),
        ("●", "Press", "while_tap", Props::IDENTITY, Props::new().scale(0.9)),
        ("✦", "Hover tilt", "rotate + scale", Props::new().scale(1.05).rotate(-4.0), Props::new().scale(0.95)),
        ("∿", "Animate", "springs between targets", Props::IDENTITY, Props::IDENTITY),
    ];
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(start, vec2(total, 160.0))).layout(egui::Layout::left_to_right(egui::Align::Min)));
    child.spacing_mut().item_spacing.x = gap;
    for (i, (icon, title, sub, hover, tap)) in cards.iter().enumerate() {
        let bid = id.with(i);
        let initial = Props::new().opacity(0.0).y(40.0).scale(0.85);
        if replay {
            MotionBox::reset(ui, bid, initial);
        }
        let mut b = MotionBox::new(bid).initial(initial).transition(Transition::BOUNCY.delay(0.08 * i as f32 + 0.1));
        if i == 3 {
            b = b.animate(if breathe { Props::new().y(-14.0).rotate(6.0).scale(1.04) } else { Props::new().y(10.0).rotate(-6.0) }).transition(Transition::spring(160.0, 9.0));
        }
        if *hover != Props::IDENTITY {
            b = b.while_hover(*hover);
        }
        if *tap != Props::IDENTITY {
            b = b.while_tap(*tap);
        }
        let accent = t.chart[i % t.chart.len()];
        b.show(&mut child, |ui| motion_card(ui, w, icon, title, sub, accent));
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
}

// ---------------------------------------------------------------------------------------------
// 6. variants & stagger

#[derive(Clone, Copy)]
struct Toggle {
    on: bool,
    auto: bool,
    last: f64,
}

/// A boolean that flips by itself every `period` seconds until the user takes over.
fn auto_toggle(ui: &Ui, id: Id, period: f64, clicked: bool) -> bool {
    let now = ui.input(|i| i.time);
    let mut s = state(ui, id, || Toggle { on: false, auto: true, last: now - period + 0.4 });
    if clicked {
        s.on = !s.on;
        s.auto = false;
    } else if s.auto && now - s.last > period {
        s.on = !s.on;
        s.last = now;
    }
    if s.auto {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
    }
    set_state(ui, id, s);
    s.on
}

fn stagger_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("stagger_demo");
    let mut clicked = false;
    let on_now: Toggle = state(ui, id.with("toggle"), || Toggle { on: false, auto: true, last: -10.0 });
    toolbar(ui, |ui| {
        clicked = ui.button(if on_now.on { "Hide all" } else { "Show all" }).clicked();
        ui.label(egui::RichText::new("one variant name drives every child").color(t.muted_foreground));
    });
    let on = auto_toggle(ui, id.with("toggle"), 2.6, clicked);
    let (rect, _) = stage(ui, 300.0);
    let variant = if on { "visible" } else { "hidden" };

    // list: slides in from the left, first to last; exits last to first
    let list_rect = Rect::from_min_size(rect.min + vec2(28.0, 26.0), vec2((rect.width() * 0.45).min(360.0), 250.0));
    let mut lui = ui.new_child(UiBuilder::new().max_rect(list_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    lui.spacing_mut().item_spacing.y = 8.0;
    let names = ["Inbox zero, finally", "Design review at 3pm", "Deploy finished", "New comment on PR #42", "Coffee with Sam"];
    MotionGroup::new(id.with("list"), Variants::fade(vec2(-36.0, 0.0)))
        .initial("hidden")
        .animate(variant)
        .stagger_children(0.07)
        .exit_order(StaggerOrder::Last)
        .transition(Transition::spring(260.0, 22.0))
        .show(&mut lui, |g, ui| {
            for (i, name) in names.iter().enumerate() {
                g.child(ui, |ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(list_rect.width(), 42.0), Sense::hover());
                    let p = ui.painter();
                    card(p, &t, r, 12.0);
                    let col = t.chart[i % t.chart.len()];
                    p.circle_filled(r.left_center() + vec2(22.0, 0.0), 11.0, with_alpha(col, 0.9));
                    p.text(r.left_center() + vec2(42.0, 0.0), Align2::LEFT_CENTER, *name, FontId::proportional(13.5), t.foreground);
                    p.rect_filled(Rect::from_min_size(r.right_center() + vec2(-58.0, -4.0), vec2(40.0, 8.0)), 4, t.muted);
                });
            }
        });

    // grid: pops from the center outward
    let gx = list_rect.max.x + 40.0;
    let grid_rect = Rect::from_min_max(pos2(gx, rect.min.y + 26.0), rect.max - vec2(24.0, 20.0));
    let cols = 5;
    let rows = 3;
    let cell = ((grid_rect.width() - 10.0 * (cols - 1) as f32) / cols as f32).min((grid_rect.height() - 10.0 * (rows - 1) as f32) / rows as f32).max(20.0);
    let gw = cell * cols as f32 + 10.0 * (cols - 1) as f32;
    let grid_rect = Rect::from_min_size(pos2(grid_rect.center().x - gw / 2.0, grid_rect.min.y), vec2(gw, grid_rect.height()));
    let tiles = Variants::new().with("hidden", Props::new().opacity(0.0).scale(0.3).rotate(-30.0)).with("visible", Props::IDENTITY);
    let mut gui = ui.new_child(UiBuilder::new().max_rect(grid_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    gui.spacing_mut().item_spacing = vec2(10.0, 10.0);
    MotionGroup::new(id.with("grid"), tiles)
        .initial("hidden")
        .animate(variant)
        .stagger_children(0.045)
        .delay_children(0.15)
        .stagger_order(StaggerOrder::Center)
        .transition(Transition::BOUNCY)
        .show(&mut gui, |g, ui| {
            for r in 0..rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for c in 0..cols {
                        let k = (r * cols + c) as f32 / (rows * cols) as f32;
                        g.child(ui, |ui| {
                            let (rr, _) = ui.allocate_exact_size(Vec2::splat(cell), Sense::hover());
                            let col = color::gradient(&t.gradient, k);
                            paint::gradient_rect(ui.painter(), rr, 12.0, 135.0, &[col, color::shade(col, -0.08)]);
                        });
                    }
                });
            }
        });
}

// ---------------------------------------------------------------------------------------------
// 7. presence

#[derive(Clone)]
struct Toast {
    id: u64,
    born: f64,
    kind: usize,
}

#[derive(Clone)]
struct Chip {
    id: u64,
    color: usize,
}

#[derive(Clone)]
struct PresenceDemo {
    toasts: Vec<Toast>,
    next_toast: f64,
    chips: Vec<Chip>,
    next_id: u64,
    next_op: f64,
    auto: bool,
    rng: Rng,
}

fn presence_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("presence_demo");
    let now = ui.input(|i| i.time);
    let mut s: PresenceDemo = state(ui, id, || PresenceDemo {
        toasts: Vec::new(),
        next_toast: now + 0.3,
        chips: (0..4).map(|i| Chip { id: i, color: i as usize }).collect(),
        next_id: 100,
        next_op: now + 1.0,
        auto: true,
        rng: Rng::new(9),
    });
    toolbar(ui, |ui| {
        if ui.button("＋ Add").clicked() {
            let c = Chip { id: s.next_id, color: s.next_id as usize };
            let at = s.rng.below(s.chips.len() + 1);
            s.chips.insert(at, c);
            s.next_id += 1;
            s.auto = false;
        }
        if ui.button("－ Remove").clicked() && !s.chips.is_empty() {
            let i = s.rng.below(s.chips.len());
            s.chips.remove(i);
            s.auto = false;
        }
        if ui.button("⇅ Shuffle").clicked() {
            for i in (1..s.chips.len()).rev() {
                let j = s.rng.below(i + 1);
                s.chips.swap(i, j);
            }
            s.auto = false;
        }
        if ui.button("🔔 Toast").clicked() {
            s.toasts.push(Toast { id: s.next_id, born: now, kind: s.next_id as usize % 3 });
            s.next_id += 1;
        }
    });
    // autoplay
    if now > s.next_toast {
        s.toasts.push(Toast { id: s.next_id, born: now, kind: s.next_id as usize % 3 });
        s.next_id += 1;
        s.next_toast = now + 1.5;
    }
    s.toasts.retain(|x| now - x.born < 3.6);
    if s.auto && now > s.next_op {
        s.next_op = now + 1.1;
        let n = s.chips.len();
        let op = if n <= 2 { 0 } else if n >= 6 { 1 } else { s.rng.below(3) };
        match op {
            0 => {
                let at = s.rng.below(n + 1);
                s.chips.insert(at, Chip { id: s.next_id, color: s.next_id as usize });
                s.next_id += 1;
            }
            1 => {
                let i = s.rng.below(n);
                s.chips.remove(i);
            }
            _ => {
                let i = s.rng.below(n);
                let c = s.chips.remove(i);
                let j = s.rng.below(s.chips.len() + 1);
                s.chips.insert(j, c);
            }
        }
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    let (rect, _) = stage(ui, 330.0);
    let half = rect.width() / 2.0;

    // keyed list
    let lw = (half - 60.0).min(320.0);
    let list_rect = Rect::from_min_size(pos2(rect.min.x + half / 2.0 - lw / 2.0, rect.min.y + 34.0), vec2(lw, rect.height() - 40.0));
    caption(ui, list_rect.left_top() - vec2(0.0, 22.0), Align2::LEFT_TOP, "PresenceList: add · remove · reorder");
    let mut lui = ui.new_child(UiBuilder::new().max_rect(list_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    lui.spacing_mut().item_spacing.y = 6.0;
    let names = ["Aurora", "Basil", "Cobalt", "Dune", "Ember", "Fjord", "Glacier", "Harbor", "Indigo", "Juniper"];
    PresenceList::new(id.with("chips"))
        .initial(Props::new().opacity(0.0).x(-40.0).scale(0.9))
        .exit(Props::new().opacity(0.0).scale(0.7))
        .show(&mut lui, &s.chips, |c| c.id, |ui, c| {
            let (r, _) = ui.allocate_exact_size(vec2(lw, 38.0), Sense::hover());
            let p = ui.painter();
            card(p, &t, r, 10.0);
            let col = t.chart[c.color % t.chart.len()];
            p.rect_filled(Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(18.0, 18.0)), 6, col);
            p.text(r.left_center() + vec2(40.0, 0.0), Align2::LEFT_CENTER, names[c.id as usize % names.len()], FontId::proportional(13.5), t.foreground);
            p.text(r.right_center() - vec2(12.0, 0.0), Align2::RIGHT_CENTER, format!("#{}", c.id), FontId::monospace(11.0), t.muted_foreground);
        });

    // toasts
    let p = ui.painter_at(rect);
    p.line_segment([pos2(rect.center().x, rect.min.y + 20.0), pos2(rect.center().x, rect.max.y - 20.0)], Stroke::new(1.0, t.border));
    let tw = (half - 60.0).min(300.0);
    let toast_rect = Rect::from_min_size(pos2(rect.center().x + half / 2.0 - tw / 2.0, rect.min.y + 34.0), vec2(tw, rect.height() - 40.0));
    caption(ui, toast_rect.left_top() - vec2(0.0, 22.0), Align2::LEFT_TOP, "Toasts that exit before unmounting");
    let mut tui = ui.new_child(UiBuilder::new().max_rect(toast_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    tui.spacing_mut().item_spacing.y = 8.0;
    let kinds = [("✔", "Changes saved", "Your profile is up to date", t.success), ("⬆", "Upload complete", "3 files · 12.4 MB", t.info), ("⚠", "Storage almost full", "92% of 50 GB used", t.warning)];
    let mut toasts = s.toasts.clone();
    toasts.reverse(); // newest on top
    PresenceList::new(id.with("toasts"))
        .initial(Props::new().opacity(0.0).y(-24.0).scale(0.92))
        .exit(Props::new().opacity(0.0).x(80.0))
        .exit_transition(Transition::tween(0.3).ease(Easing::InCubic))
        .show(&mut tui, &toasts, |x| x.id, |ui, x| {
            let (r, _) = ui.allocate_exact_size(vec2(tw, 58.0), Sense::hover());
            let (icon, title, sub, col) = kinds[x.kind];
            let p = ui.painter();
            paint::soft_shadow(p, r.translate(vec2(0.0, 6.0)), 12.0, 18.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 120 } else { 30 }));
            p.rect_filled(r, 12, t.popover);
            p.rect_stroke(r, 12, t.border_stroke(), StrokeKind::Inside);
            p.circle_filled(r.left_center() + vec2(24.0, 0.0), 13.0, with_alpha(col, 0.16));
            p.text(r.left_center() + vec2(24.0, 0.0), Align2::CENTER_CENTER, icon, FontId::proportional(13.0), col);
            p.text(r.min + vec2(48.0, 12.0), Align2::LEFT_TOP, title, FontId::proportional(13.5), t.foreground);
            p.text(r.min + vec2(48.0, 31.0), Align2::LEFT_TOP, sub, FontId::proportional(11.5), t.muted_foreground);
            // life bar
            let life = ((ui.input(|i| i.time) - x.born) / 3.6).clamp(0.0, 1.0) as f32;
            let bar = Rect::from_min_size(pos2(r.min.x + 12.0, r.max.y - 4.0), vec2((r.width() - 24.0) * (1.0 - life), 2.0));
            p.rect_filled(bar, 1, with_alpha(col, 0.6));
        });
    set_state(ui, id, s);
}

// ---------------------------------------------------------------------------------------------
// 8. layout animation

#[derive(Clone)]
struct LayoutDemo {
    order: Vec<usize>,
    grid: bool,
    auto: bool,
    next: f64,
    step: u32,
    rng: Rng,
}

fn layout_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("layout_demo");
    let now = ui.input(|i| i.time);
    let mut s: LayoutDemo = state(ui, id, || LayoutDemo { order: (0..8).collect(), grid: true, auto: true, next: now + 1.2, step: 0, rng: Rng::new(4) });
    toolbar(ui, |ui| {
        if ui.add(egui::Button::selectable(s.grid, "▦ Grid")).clicked() {
            s.grid = true;
            s.auto = false;
        }
        if ui.add(egui::Button::selectable(!s.grid, "☰ List")).clicked() {
            s.grid = false;
            s.auto = false;
        }
        if ui.button("⇄ Shuffle").clicked() {
            for i in (1..s.order.len()).rev() {
                let j = s.rng.below(i + 1);
                s.order.swap(i, j);
            }
            s.auto = false;
        }
        if ui.button("⇅ Sort").clicked() {
            s.order.sort();
            s.auto = false;
        }
    });
    if s.auto && now > s.next {
        s.next = now + 1.8;
        s.step += 1;
        if s.step % 2 == 1 {
            s.grid = !s.grid;
        } else {
            for i in (1..s.order.len()).rev() {
                let j = s.rng.below(i + 1);
                s.order.swap(i, j);
            }
        }
    }
    if s.auto {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    let (rect, _) = stage(ui, 330.0);
    let inner = rect.shrink2(vec2(24.0, 20.0));
    let names = ["Nebula", "Lagoon", "Canyon", "Meadow", "Tundra", "Savanna", "Reef", "Volcano"];
    let icons = ["✦", "≈", "▲", "❀", "◆", "✚", "◉", "★"];
    LayoutAnim::new(id.with("layout")).transition(Transition::spring(300.0, 28.0)).show(ui, |layout, ui| {
        for (slot, &item) in s.order.iter().enumerate() {
            let target = if s.grid {
                let cols = 4;
                let gap = 14.0;
                let cw = (inner.width() - gap * (cols - 1) as f32) / cols as f32;
                let ch = (inner.height() - gap) / 2.0;
                Rect::from_min_size(inner.min + vec2((slot % cols) as f32 * (cw + gap), (slot / cols) as f32 * (ch + gap)), vec2(cw, ch))
            } else {
                let gap = 6.0;
                let rh = (inner.height() - gap * 7.0) / 8.0;
                let w = inner.width().min(520.0);
                Rect::from_min_size(pos2(inner.center().x - w / 2.0, inner.min.y + slot as f32 * (rh + gap)), vec2(w, rh))
            };
            layout.item_at(ui, item, target, |ui| {
                let r = ui.max_rect();
                let p = ui.painter();
                let col = t.chart[item % t.chart.len()];
                let rad = (r.height() * 0.18).clamp(6.0, 16.0);
                paint::soft_shadow(p, r.translate(vec2(0.0, 3.0)), rad, 12.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 90 } else { 22 }));
                paint::gradient_rect(p, r, rad, 135.0, &[with_alpha(col, 0.95), color::shade(col, -0.12)]);
                // content adapts to the animated rect: icon grows with the card
                let tall = (r.height() / 110.0).clamp(0.0, 1.0);
                let icon_size = 14.0 + 22.0 * tall;
                let ink = color::contrast_text(col);
                if tall > 0.5 {
                    p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, icons[item], FontId::proportional(icon_size), with_alpha(ink, 0.95));
                    p.text(r.center() + vec2(0.0, icon_size * 0.6 + 4.0), Align2::CENTER_CENTER, names[item], FontId::proportional(13.0), ink);
                } else {
                    p.text(r.left_center() + vec2(14.0, 0.0), Align2::LEFT_CENTER, icons[item], FontId::proportional(icon_size), ink);
                    p.text(r.left_center() + vec2(40.0, 0.0), Align2::LEFT_CENTER, names[item], FontId::proportional(13.0), ink);
                    p.text(r.right_center() - vec2(14.0, 0.0), Align2::RIGHT_CENTER, format!("#{item}"), FontId::monospace(11.0), with_alpha(ink, 0.8));
                }
            });
        }
    });
    set_state(ui, id, s);
}

// ---------------------------------------------------------------------------------------------
// 9. shared element

#[derive(Clone, Copy)]
struct SharedDemo {
    open: Option<usize>,
    auto: bool,
    next: f64,
    step: usize,
}

fn shared_element_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("shared_demo");
    let now = ui.input(|i| i.time);
    let mut s = state(ui, id, || SharedDemo { open: None, auto: true, next: now + 0.9, step: 0 });
    if s.auto && now > s.next {
        s.step += 1;
        s.open = if s.open.is_some() { None } else { Some((s.step / 2) % 4) };
        s.next = now + if s.open.is_some() { 2.0 } else { 1.0 };
    }
    if s.auto {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    toolbar(ui, |ui| {
        ui.label(egui::RichText::new("click an album to open it, click again to close").color(t.muted_foreground));
    });
    let (rect, _) = stage(ui, 330.0);
    let albums = [("Midnight Drive", "Neon Coast", 0usize), ("Paper Moons", "Lumen", 2), ("Low Tide", "Harbor Lights", 1), ("Static Bloom", "Vela", 3)];
    let inner = rect.shrink(22.0);
    let thumbs_w = (inner.width() * 0.42).min(330.0);
    let cell = ((thumbs_w - 14.0) / 2.0).min((inner.height() - 14.0) / 2.0);
    let detail = Rect::from_min_max(pos2(inner.min.x + cell * 2.0 + 14.0 + 30.0, inner.min.y), inner.max);
    let open_k: f32 = animate(ui, id.with("open_k"), if s.open.is_some() { 1.0 } else { 0.0 }, Transition::tween(0.3));
    // empty-state hint behind the detail area
    {
        let p = ui.painter_at(rect);
        let hint = with_alpha(t.muted_foreground, 1.0 - open_k);
        p.rect_stroke(detail, t.corner_scaled(1.6), Stroke::new(1.0, with_alpha(t.border, 1.0 - open_k * 0.7)), StrokeKind::Inside);
        p.text(detail.center(), Align2::CENTER_CENTER, "Pick an album", FontId::proportional(15.0), hint);
    }
    let mut clicked = None;
    LayoutAnim::new(id.with("layout")).transition(Transition::spring(260.0, 26.0)).show(ui, |layout, ui| {
        for (i, (title, artist, ci)) in albums.iter().enumerate() {
            let is_open = s.open == Some(i);
            let target = if is_open {
                // the cover sits at the left of the detail panel
                let side = detail.height().min(detail.width() * 0.5);
                Rect::from_min_size(detail.min, Vec2::splat(side))
            } else {
                Rect::from_min_size(inner.min + vec2((i % 2) as f32 * (cell + 14.0), (i / 2) as f32 * (cell + 14.0)), Vec2::splat(cell))
            };
            let resp = layout.item_at(ui, ("album", i), target, |ui| {
                let r = ui.max_rect();
                let p = ui.painter();
                let base = t.chart[*ci % t.chart.len()];
                let rad = (r.width() * 0.08).clamp(8.0, 20.0);
                paint::soft_shadow(p, r.translate(vec2(0.0, 6.0)), rad, 20.0, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 120 } else { 35 }));
                paint::gradient_rect(p, r, rad, 135.0, &[base, color::mix(base, t.gradient[1], 0.5), color::shade(base, -0.2)]);
                // concentric "vinyl" rings scale with the cover
                let c = r.center();
                for k in 1..5 {
                    p.circle_stroke(c, r.width() * 0.08 * k as f32, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.10)));
                }
                p.circle_filled(c, r.width() * 0.06, with_alpha(Color32::WHITE, 0.85));
                let ink = color::contrast_text(base);
                p.text(r.left_bottom() + vec2(10.0, -10.0), Align2::LEFT_BOTTOM, *title, FontId::proportional((r.width() * 0.075).clamp(11.0, 20.0)), ink);
            });
            if resp.clicked() {
                clicked = Some(i);
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if is_open {
                // details fade in next to the cover
                let side = detail.height().min(detail.width() * 0.5);
                let tx = detail.min.x + side + 24.0;
                let p = ui.painter_at(rect);
                let a = open_k;
                let dy = (1.0 - a) * 12.0;
                p.text(pos2(tx, detail.min.y + 18.0 + dy), Align2::LEFT_TOP, *title, FontId::proportional(24.0), with_alpha(t.foreground, a));
                p.text(pos2(tx, detail.min.y + 52.0 + dy), Align2::LEFT_TOP, *artist, FontId::proportional(15.0), with_alpha(t.muted_foreground, a));
                for (k, track) in ["Intro", "Afterglow", "Signals", "Long Way Home"].iter().enumerate() {
                    let y = detail.min.y + 96.0 + k as f32 * 30.0 + dy * (1.0 + k as f32 * 0.5);
                    let aa = (a * 1.6 - k as f32 * 0.15).clamp(0.0, 1.0);
                    p.text(pos2(tx, y), Align2::LEFT_TOP, format!("{:02}   {track}", k + 1), FontId::proportional(13.5), with_alpha(t.foreground, aa * 0.9));
                }
            }
        }
    });
    if let Some(i) = clicked {
        s.open = if s.open == Some(i) { None } else { Some(i) };
        s.auto = false;
    }
    set_state(ui, id, s);
}

// ---------------------------------------------------------------------------------------------
// 10. drag

#[derive(Clone, Copy)]
struct DragDemo {
    next: f64,
    step: u32,
    rng: Rng,
}

fn drag_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("drag_demo");
    let now = ui.input(|i| i.time);
    let mut s = state(ui, id, || DragDemo { next: now + 0.8, step: 0, rng: Rng::new(3) });
    let (rect, _) = stage(ui, 330.0);
    let idle = idle_for(ui, id, rect);
    let third = rect.width() / 3.0;
    let zone_a = Rect::from_min_size(rect.min, vec2(third * 1.3, rect.height())).shrink(18.0);
    let zone_b = Rect::from_min_max(pos2(zone_a.max.x + 18.0, rect.min.y), pos2(rect.max.x, rect.center().y)).shrink(18.0);
    let zone_c = Rect::from_min_max(pos2(zone_a.max.x + 18.0, rect.center().y), rect.max).shrink(18.0);
    let p = ui.painter_at(rect);
    for z in [zone_a, zone_b, zone_c] {
        p.rect_stroke(z, t.corner(), Stroke::new(1.0, with_alpha(t.muted_foreground, 0.25)), StrokeKind::Inside);
    }
    caption(ui, zone_a.left_top() + vec2(12.0, 10.0), Align2::LEFT_TOP, "constraints · elastic · momentum");
    caption(ui, zone_b.left_top() + vec2(12.0, 10.0), Align2::LEFT_TOP, "axis X · snap points");
    caption(ui, zone_c.left_top() + vec2(12.0, 10.0), Align2::LEFT_TOP, "snap to origin");

    let (ida, idb, idc) = (id.with("a"), id.with("b"), id.with("c"));
    // autoplay: throw things around while nobody is touching
    if idle > 2.5 && now > s.next {
        s.next = now + 1.6;
        s.step += 1;
        let a_off = vec2(s.rng.range(-0.42, 0.42) * zone_a.width(), s.rng.range(-0.36, 0.36) * zone_a.height());
        Draggable::animate_to(ui, ida, a_off, Transition::spring(120.0, 9.0));
        let snaps = [-1.0, 0.0, 1.0];
        Draggable::animate_to(ui, idb, vec2(snaps[(s.step % 3) as usize] * zone_b.width() * 0.32, 0.0), Transition::spring(380.0, 26.0));
        let pull = if s.step % 2 == 0 { vec2(s.rng.range(-80.0, 80.0), s.rng.range(-26.0, 26.0)) } else { Vec2::ZERO };
        Draggable::animate_to(ui, idc, pull, Transition::spring(240.0, 12.0));
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    set_state(ui, id, s);

    // (a) puck
    let mut a_ui = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(zone_a.center(), Vec2::splat(76.0))));
    let r = Draggable::new(ida)
        .constraints(zone_a.shrink(6.0))
        .elastic(0.4)
        .momentum(true)
        .decay(0.35)
        .while_hover(Props::new().scale(1.05))
        .while_drag(Props::new().scale(1.12))
        .show(&mut a_ui, |ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(76.0), Sense::hover());
            let p = ui.painter();
            paint::glow(p, r.center(), 70.0, with_alpha(t.gradient[0], 0.3));
            paint::gradient_circle(p, r.center(), 38.0, &[t.gradient[1], t.gradient[0]]);
            p.text(r.center(), Align2::CENTER_CENTER, "throw", FontId::proportional(13.0), Color32::WHITE);
        });
    let _ = r.velocity;

    // (b) segmented snap knob
    let track = Rect::from_center_size(zone_b.center() + vec2(0.0, 10.0), vec2(zone_b.width() * 0.64 + 56.0, 56.0));
    p.rect_filled(track, 28, with_alpha(t.border, 0.8));
    for k in [-1.0, 0.0, 1.0] {
        p.circle_filled(pos2(track.center().x + k * zone_b.width() * 0.32, track.center().y), 4.0, t.muted_foreground);
    }
    let mut b_ui = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(track.center(), Vec2::splat(48.0))));
    let rb = Draggable::new(idb)
        .axis(Axis::X)
        .constraints(track.shrink(4.0))
        .elastic(0.2)
        .snap_points([vec2(-zone_b.width() * 0.32, 0.0), Vec2::ZERO, vec2(zone_b.width() * 0.32, 0.0)])
        .while_drag(Props::new().scale(1.1))
        .show(&mut b_ui, |ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
            let p = ui.painter();
            paint::soft_shadow(p, r, 24.0, 12.0, vec2(0.0, 3.0), Color32::from_black_alpha(60));
            p.circle_filled(r.center(), 24.0, t.primary);
            p.circle_filled(r.center(), 8.0, t.accent);
        });
    let k = (rb.offset.x / (zone_b.width() * 0.32)).clamp(-1.0, 1.0);
    let labels = ["Low", "Mid", "High"];
    let idx = (k.round() as i32 + 1) as usize;
    caption(ui, pos2(track.center().x, track.max.y + 8.0), Align2::CENTER_TOP, &format!("{}  ·  offset {:+.0}", labels[idx], rb.offset.x));

    // (c) rubber card that snaps home
    let mut c_ui = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(zone_c.center() + vec2(0.0, 8.0), vec2(170.0, 64.0))));
    let rc = Draggable::new(idc)
        .snap_to_origin(true)
        .elastic(0.5)
        .snap_transition(Transition::spring(300.0, 14.0))
        .while_drag(Props::new().rotate(-3.0).scale(1.04))
        .show(&mut c_ui, |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(170.0, 64.0), Sense::hover());
            let p = ui.painter();
            card(p, &t, r, 14.0);
            p.text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "pull me", FontId::proportional(14.0), t.foreground);
            p.text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "I spring back", FontId::proportional(11.0), t.muted_foreground);
        });
    // a rubber band from home to the card
    let home = zone_c.center() + vec2(0.0, 8.0);
    if rc.offset.length() > 2.0 {
        p.line_segment([home, home + rc.offset], Stroke::new(2.0, with_alpha(t.accent, 0.5)));
        p.circle_filled(home, 4.0, t.accent);
    }
}

// ---------------------------------------------------------------------------------------------
// 11. reorder

#[derive(Clone)]
struct Task {
    id: u32,
    icon: &'static str,
    title: &'static str,
    sub: &'static str,
}

#[derive(Clone)]
struct ReorderDemo {
    tasks: Vec<Task>,
    next: f64,
    rng: Rng,
}

fn reorder_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("reorder_demo");
    let now = ui.input(|i| i.time);
    let mut s: ReorderDemo = state(ui, id, || ReorderDemo {
        tasks: vec![
            Task { id: 0, icon: "●", title: "Morning coffee", sub: "07:30 · 15 min" },
            Task { id: 1, icon: "@", title: "Answer email", sub: "08:00 · 30 min" },
            Task { id: 2, icon: "✎", title: "Write the spec", sub: "09:00 · 2 h" },
            Task { id: 3, icon: "◆", title: "Standup call", sub: "11:00 · 15 min" },
            Task { id: 4, icon: "★", title: "Lunch", sub: "12:30 · 45 min" },
        ],
        next: now + 1.0,
        rng: Rng::new(11),
    });
    let (rect, _) = stage(ui, 330.0);
    let idle = idle_for(ui, id, rect);
    if idle > 2.5 && now > s.next {
        s.next = now + 1.4;
        let n = s.tasks.len();
        let i = s.rng.below(n);
        let mut j = s.rng.below(n);
        if j == i {
            j = (i + 2) % n;
        }
        let x = s.tasks.remove(i);
        s.tasks.insert(j, x);
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    let w = (rect.width() - 60.0).min(420.0);
    let list = Rect::from_min_size(pos2(rect.center().x - w / 2.0, rect.min.y + 26.0), vec2(w, rect.height() - 40.0));
    let mut lui = ui.new_child(UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)));
    Reorder::new(id.with("list")).item_height(50.0).gap(9.0).show(&mut lui, &mut s.tasks, |x| x.id, |ui, task, st| {
        let r = ui.max_rect();
        let p = ui.painter();
        let lift = if st.dragging { 1.0 } else { 0.0 };
        paint::soft_shadow(p, r.translate(vec2(0.0, 3.0 + 8.0 * lift)), 12.0, 12.0 + 18.0 * lift, Vec2::ZERO, Color32::from_black_alpha(if t.dark { 100 + (80.0 * lift) as u8 } else { 22 + (30.0 * lift) as u8 }));
        p.rect_filled(r, 12, t.card);
        p.rect_stroke(r, 12, Stroke::new(1.0, if st.dragging { t.accent } else { t.border }), StrokeKind::Inside);
        let col = t.chart[task.id as usize % t.chart.len()];
        let ic = Rect::from_center_size(r.left_center() + vec2(28.0, 0.0), Vec2::splat(32.0));
        p.rect_filled(ic, 9, with_alpha(col, 0.16));
        p.text(ic.center(), Align2::CENTER_CENTER, task.icon, FontId::proportional(15.0), col);
        p.text(r.left_center() + vec2(56.0, -8.0), Align2::LEFT_CENTER, task.title, FontId::proportional(14.0), t.foreground);
        p.text(r.left_center() + vec2(56.0, 10.0), Align2::LEFT_CENTER, task.sub, FontId::proportional(11.5), t.muted_foreground);
        // grip
        let g = r.right_center() - vec2(22.0, 0.0);
        for dx in [-3.0, 3.0] {
            for dy in [-6.0, 0.0, 6.0] {
                p.circle_filled(g + vec2(dx, dy), 1.6, t.muted_foreground);
            }
        }
        p.text(r.right_center() - vec2(44.0, 0.0), Align2::RIGHT_CENTER, format!("{}", st.index + 1), FontId::monospace(11.0), t.muted_foreground);
    });
    set_state(ui, id, s);
}

// ---------------------------------------------------------------------------------------------
// 12. scroll-linked

fn scroll_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("scroll_demo");
    let now = glowkit_core::time(ui);
    let view_h = 300.0;
    let (bar_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    let content_h: f32 = state(ui, id.with("content_h"), || 1200.0);
    let last_rect: Rect = state(ui, id.with("rect"), || Rect::NOTHING);
    let idle = idle_for(ui, id, last_rect);
    let hovered = ui.rect_contains_pointer(last_rect);
    let auto = !hovered && idle > 2.5;
    let mut sa = egui::ScrollArea::vertical().id_salt(id.with("scroll")).max_height(view_h).auto_shrink([false, false]);
    if auto {
        let max = (content_h - view_h).max(0.0);
        let o = ((now * 0.42 - std::f32::consts::FRAC_PI_2).sin() * 0.5 + 0.5) * max;
        sa = sa.vertical_scroll_offset(o);
        ui.ctx().request_repaint();
    }
    let frame = egui::Frame::new().fill(t.muted).corner_radius(t.corner()).stroke(t.border_stroke()).inner_margin(egui::Margin::same(0));
    let out = frame
        .show(ui, |ui| {
            sa.show(ui, |ui| {
                let scroll = scroll_offset(ui).y;
                let w = ui.available_width();
                // hero with parallax layers (each drifts at its own speed)
                let (hero, _) = ui.allocate_exact_size(vec2(w, 300.0), Sense::hover());
                let p = ui.painter().with_clip_rect(hero.intersect(ui.clip_rect()));
                let sky_top = color::mix(t.muted, t.gradient[0], if t.dark { 0.18 } else { 0.10 });
                let sky_bot = color::mix(t.muted, t.gradient[2], if t.dark { 0.22 } else { 0.16 });
                paint::gradient_rect(&p, hero, 0.0, 90.0, &[sky_top, sky_bot]);
                let sun = pos2(hero.min.x + w * 0.72, hero.min.y + 96.0 + parallax(scroll, 0.85));
                paint::glow(&p, sun, 120.0, with_alpha(t.gradient[2], if t.dark { 0.35 } else { 0.3 }));
                p.circle_filled(sun, 30.0, color::mix(t.gradient[2], t.gradient[1], 0.25));
                let ridges: [(f32, f32, f32, f32); 3] = [(0.65, 150.0, 0.004, 0.45), (0.38, 190.0, 0.007, 0.7), (0.12, 232.0, 0.011, 1.0)];
                for (li, (factor, base, freq, strength)) in ridges.iter().enumerate() {
                    let off = parallax(scroll, *factor);
                    let pts: Vec<Pos2> = (0..=60)
                        .map(|k| {
                            let x = hero.min.x + w * k as f32 / 60.0;
                            let u = x * freq + li as f32 * 1.7;
                            let y = hero.min.y + base + off - 34.0 * (u.sin() * 0.6 + (u * 2.3).sin() * 0.3 + (u * 5.1).sin() * 0.1) * (1.0 + li as f32 * 0.2);
                            pos2(x, y)
                        })
                        .collect();
                    let col = color::mix(t.muted, color::mix(t.gradient[0], t.gradient[1], li as f32 * 0.4), *strength * if t.dark { 0.55 } else { 0.45 });
                    paint::area_fill(&p, &pts, hero.max.y + 2.0, col, color::shade(col, -0.04));
                }
                let fade: f32 = transform(scroll, &[0.0, 200.0], &[1.0, 0.0]);
                let ty = hero.min.y + 92.0 + parallax(scroll, 0.5);
                let tc: Color32 = transform(scroll, &[0.0, 200.0], &[t.foreground, t.accent]);
                p.text(pos2(hero.min.x + 40.0, ty), Align2::LEFT_CENTER, "Scroll-linked motion", FontId::proportional(32.0), with_alpha(tc, fade));
                p.text(pos2(hero.min.x + 40.0, ty + 34.0), Align2::LEFT_CENTER, "parallax · transform() · whileInView", FontId::proportional(15.0), with_alpha(t.muted_foreground, fade));
                // cards animate in when they scroll into view
                ui.add_space(10.0);
                let feats = [
                    ("Progress", "A bar bound to scroll progress."),
                    ("Parallax", "Layers drift at their own speed."),
                    ("Transform", "Map scroll to color, size or angle."),
                    ("In view", "Cards animate as they appear."),
                    ("Springs", "Every transition is physical."),
                    ("Pure Rust", "No JavaScript, just egui."),
                ];
                for (i, (title, sub)) in feats.iter().enumerate() {
                    let cid = id.with(("card", i));
                    let probe = Rect::from_min_size(ui.cursor().min, vec2(w, 84.0));
                    let seen = InView::new(cid).amount(0.35).check(ui, probe);
                    let side = if i % 2 == 0 { -70.0 } else { 70.0 };
                    let target = if seen { Props::IDENTITY } else { Props::new().opacity(0.0).x(side).scale(0.94) };
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        MotionBox::new(cid).initial(Props::new().opacity(0.0).x(side)).animate(target).transition(Transition::spring(200.0, 22.0)).show(ui, |ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(w - 48.0, 76.0), Sense::hover());
                            let p = ui.painter();
                            card(p, &t, r, 14.0);
                            let col = t.chart[i % t.chart.len()];
                            p.circle_filled(r.left_center() + vec2(34.0, 0.0), 18.0, with_alpha(col, 0.2));
                            p.text(r.left_center() + vec2(34.0, 0.0), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(15.0), col);
                            p.text(r.left_center() + vec2(66.0, -10.0), Align2::LEFT_CENTER, *title, FontId::proportional(16.0), t.foreground);
                            p.text(r.left_center() + vec2(66.0, 12.0), Align2::LEFT_CENTER, *sub, FontId::proportional(12.5), t.muted_foreground);
                        });
                    });
                }
                ui.add_space(40.0);
            })
        })
        .inner;
    set_state(ui, id.with("content_h"), out.content_size.y);
    set_state(ui, id.with("rect"), out.inner_rect);
    // progress bar bound to scroll progress
    let prog = scroll_progress(&out).y;
    let p = ui.painter();
    let track = Rect::from_min_size(bar_rect.min + vec2(0.0, 4.0), vec2(bar_rect.width() - 70.0, 6.0));
    p.rect_filled(track, 3, t.border);
    let grad = t.gradient;
    paint::gradient_rect(p, Rect::from_min_size(track.min, vec2(track.width() * prog, track.height())), 3.0, 0.0, &grad);
    let ring_c = pos2(bar_rect.max.x - 50.0, track.center().y);
    let ang: f32 = transform(prog, &[0.0, 1.0], &[0.0_f32, TAU]);
    paint::arc(p, ring_c, 8.0, -TAU / 4.0, -TAU / 4.0 + ang.max(0.01), 2.5, t.accent);
    p.text(pos2(bar_rect.max.x, track.center().y), Align2::RIGHT_CENTER, format!("{:>3.0}%", prog * 100.0), FontId::monospace(11.5), t.muted_foreground);
}

// ---------------------------------------------------------------------------------------------
// 13. motion path

fn arrow(painter: &egui::Painter, pos: Pos2, angle: f32, size: f32, color: Color32) {
    let rot = egui::emath::Rot2::from_angle(angle);
    let pts = [vec2(size, 0.0), vec2(-size * 0.7, size * 0.62), vec2(-size * 0.35, 0.0), vec2(-size * 0.7, -size * 0.62)];
    let pts: Vec<Pos2> = pts.iter().map(|v| pos + rot * *v).collect();
    painter.add(egui::Shape::convex_polygon(vec![pts[0], pts[1], pts[2]], color, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(vec![pts[0], pts[2], pts[3]], color, Stroke::NONE));
}

fn motion_path_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("motion_path");
    let mut speed: f32 = state(ui, id.with("speed"), || 1.0);
    toolbar(ui, |ui| {
        ui.add(egui::Slider::new(&mut speed, 0.2..=2.5).text("speed").max_decimals(1));
    });
    set_state(ui, id.with("speed"), speed);
    let (rect, _) = stage(ui, 320.0);
    let c = rect.center();
    let (w, h) = (rect.width() * 0.4, rect.height() * 0.36);
    // a looping figure-eight through a few control points
    let ctrl: Vec<Pos2> = (0..10)
        .map(|i| {
            let a = i as f32 / 10.0 * TAU;
            c + vec2(a.sin() * w, (2.0 * a).sin() * h * 0.9 + (a * 3.0).cos() * 8.0)
        })
        .collect();
    let path = MotionPath::smooth(&ctrl, true);
    let dt = glowkit_core::dt(ui);
    let prog = glowkit_core::with_state(ui, id.with("prog"), || 0.0_f32, |p| {
        *p = (*p + dt * speed * 0.12) % 1.0;
        *p
    });
    let p = ui.painter_at(rect);
    // faint dashed path
    let pts = path.points();
    for (i, w2) in pts.windows(2).enumerate() {
        if i % 4 < 2 {
            p.line_segment([w2[0], w2[1]], Stroke::new(1.5, with_alpha(t.muted_foreground, 0.35)));
        }
    }
    // convoy of dots at staggered offsets
    for k in 1..6 {
        let s = path.sample(prog - k as f32 * 0.035 - 0.05);
        p.circle_filled(s.pos, 5.0 - k as f32 * 0.6, with_alpha(t.chart[1], 0.9 - k as f32 * 0.13));
    }
    // comet trail behind the arrow
    let col = t.gradient[0];
    let tail = 0.16;
    let (a, b) = (prog - tail, prog);
    let draw_seg = |from: f32, to: f32, base: f32| {
        if to > from {
            path.paint_range(&p, from, to, 5.0, |k| with_alpha(col, ((base + k * (to - from)) / tail).clamp(0.0, 1.0) * 0.8));
        }
    };
    if a < 0.0 {
        draw_seg(1.0 + a, 1.0, 0.0);
        draw_seg(0.0, b, -a);
    } else {
        draw_seg(a, b, 0.0);
    }
    let head = path.sample(prog);
    paint::glow(&p, head.pos, 46.0, with_alpha(col, 0.35));
    arrow(&p, head.pos, head.angle, 16.0, t.foreground);
    // readout
    caption(ui, rect.left_bottom() + vec2(16.0, -14.0), Align2::LEFT_BOTTOM, &format!("t = {:.2}   angle = {:>4.0}°   length = {:.0} pt", prog, head.angle.to_degrees(), path.length()));
    glowkit_core::animate(ui);
}

// ---------------------------------------------------------------------------------------------
// 14. path drawing

fn path_drawing_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("path_draw");
    let (rect, _) = stage(ui, 300.0);
    let n = 4;
    let gap = 16.0;
    let cw = (rect.width() - 32.0 - gap * (n - 1) as f32) / n as f32;
    let p = ui.painter_at(rect);
    let draw: f32 = Keyframes::new([(0.0, 0.0_f32), (0.45, 1.0), (0.75, 1.0), (1.0, 1.0)]).ease_segments([Easing::InOutCubic, Easing::Linear, Easing::Linear]).duration(3.2).repeat(Repeat::Infinite).play(ui, id.with("draw"));
    let erase: f32 = Keyframes::new([(0.0, 0.0_f32), (0.75, 0.0), (1.0, 1.0)]).ease_segments([Easing::Linear, Easing::InOutCubic]).duration(3.2).repeat(Repeat::Infinite).play(ui, id.with("erase"));
    let labels = ["checkmark", "signature", "star", "chart"];
    for i in 0..n {
        let cell = Rect::from_min_size(pos2(rect.min.x + 16.0 + i as f32 * (cw + gap), rect.min.y + 20.0), vec2(cw, rect.height() - 60.0));
        p.rect_filled(cell, t.corner(), t.card);
        p.rect_stroke(cell, t.corner(), t.border_stroke(), StrokeKind::Inside);
        let c = cell.center();
        let s = cell.width().min(cell.height()) * 0.36;
        let path = match i {
            0 => {
                let mut pts: Vec<Pos2> = (0..=60).map(|k| {
                    let a = -TAU * 0.3 + k as f32 / 60.0 * TAU * 0.98;
                    c + vec2(a.cos(), a.sin()) * s
                }).collect();
                pts.extend([c + vec2(-s * 0.45, 0.0), c + vec2(-s * 0.1, s * 0.35), c + vec2(s * 0.5, -s * 0.35)]);
                MotionPath::new(pts, false)
            }
            1 => {
                let ctrl: Vec<Pos2> = [(-1.0, 0.3), (-0.75, -0.5), (-0.55, 0.45), (-0.3, -0.1), (-0.1, 0.35), (0.05, -0.55), (0.2, 0.4), (0.45, -0.2), (0.6, 0.3), (0.8, -0.35), (1.0, 0.1)]
                    .iter()
                    .map(|(x, y)| c + vec2(*x * s * 1.2, *y * s))
                    .collect();
                MotionPath::smooth(&ctrl, false)
            }
            2 => {
                let pts: Vec<Pos2> = (0..10).map(|k| {
                    let a = k as f32 / 10.0 * TAU - TAU / 4.0;
                    let r = if k % 2 == 0 { s } else { s * 0.45 };
                    c + vec2(a.cos(), a.sin()) * r
                }).collect();
                MotionPath::new(pts, true)
            }
            _ => {
                let vals = [0.2, 0.5, 0.35, 0.7, 0.55, 0.9, 0.75, 1.0];
                let pts: Vec<Pos2> = vals.iter().enumerate().map(|(k, v)| pos2(cell.min.x + 20.0 + k as f32 * (cell.width() - 40.0) / 7.0, cell.max.y - 24.0 - v * (cell.height() - 60.0))).collect();
                MotionPath::new(paint::monotone(&pts, 10), false)
            }
        };
        // ghost
        let ghost = path.points().to_vec();
        p.add(egui::Shape::line(ghost, Stroke::new(1.0, with_alpha(t.muted_foreground, 0.18))));
        let stops = [t.chart[i % t.chart.len()], t.gradient[(i + 1) % 3]];
        let local_draw = (draw * 1.15 - i as f32 * 0.05).clamp(0.0, 1.0);
        let local_erase = (erase * 1.15 - i as f32 * 0.05).clamp(0.0, 1.0);
        path.paint_range(&p, local_erase, local_draw, 4.0, |k| color::gradient(&stops, local_erase + k * (local_draw - local_erase)));
        if local_draw > local_erase + 0.002 && local_draw < 0.999 {
            let tip = path.point_at(local_draw);
            paint::glow(&p, tip, 22.0, with_alpha(stops[1], 0.6));
            p.circle_filled(tip, 3.5, Color32::WHITE);
        }
        p.text(pos2(cell.center().x, cell.max.y + 12.0), Align2::CENTER_TOP, labels[i], FontId::monospace(11.0), t.muted_foreground);
    }
}

// ---------------------------------------------------------------------------------------------
// 15. lanyard

fn lanyard_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("lanyard");
    Lanyard::new(id).height(380.0).card_size(vec2(176.0, 230.0)).rope_length(140.0).show(ui, |ui| {
        let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
        let p = ui.painter();
        paint::gradient_circle(p, r.center(), 34.0, &[t.gradient[1], t.gradient[0]]);
        p.text(r.center(), Align2::CENTER_CENTER, "AR", FontId::proportional(24.0), Color32::WHITE);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Alex Rivera").size(17.0).strong().color(t.foreground));
        ui.label(egui::RichText::new("Motion Engineer").size(12.5).color(t.muted_foreground));
        ui.add_space(10.0);
        let (bar, _) = ui.allocate_exact_size(vec2(128.0, 26.0), Sense::hover());
        let p = ui.painter();
        // a little barcode
        let mut rng = Rng::new(42);
        let mut x = bar.min.x;
        while x < bar.max.x {
            let w = rng.range(1.0, 3.5);
            p.rect_filled(Rect::from_min_size(pos2(x, bar.min.y), vec2(w, bar.height())), 0, with_alpha(t.foreground, 0.75));
            x += w + rng.range(1.5, 3.0);
        }
    });
    caption(ui, ui.min_rect().left_bottom() + vec2(4.0, -20.0), Align2::LEFT_BOTTOM, "grab the badge and fling it");
}

// ---------------------------------------------------------------------------------------------
// 16. shape morph

fn morph_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let (rect, _) = stage(ui, 320.0);
    let mut big = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(pos2(rect.min.x + rect.width() * 0.34, rect.center().y), Vec2::splat(240.0))));
    big.add(MorphShape::new().size(240.0));
    // small variants
    let sets: [(&[ShapeKind], [Color32; 2], f32); 3] = [
        (&[ShapeKind::Polygon(3), ShapeKind::Polygon(4), ShapeKind::Polygon(5), ShapeKind::Polygon(6), ShapeKind::Circle], [t.chart[1], t.chart[5]], 0.3),
        (&[ShapeKind::Star(4), ShapeKind::Star(5), ShapeKind::Star(6), ShapeKind::Star(8)], [t.chart[3], t.chart[2]], 0.7),
        (&[ShapeKind::Blob(1), ShapeKind::Blob(2), ShapeKind::Blob(5), ShapeKind::Blob(9)], [t.chart[4], t.chart[1]], 1.1),
    ];
    let names = ["polygons", "stars", "blobs"];
    let col_x = rect.min.x + rect.width() * 0.68;
    for (i, (shapes, cols, phase)) in sets.iter().enumerate() {
        let c = pos2(col_x + (i as f32 - 1.0) * 120.0, rect.center().y - 20.0);
        let mut cu = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(c, Vec2::splat(100.0))));
        cu.add(MorphShape::new().shapes(shapes).size(100.0).colors(cols).phase(*phase).hold(0.5).duration(0.7).easing(Easing::OutBack));
        caption(ui, c + vec2(0.0, 64.0), Align2::CENTER_TOP, names[i]);
    }
}

// ---------------------------------------------------------------------------------------------
// 17. spring chain

#[derive(Clone)]
struct ChainDemo {
    dots: SpringChain,
    ribbon: SpringChain,
}

fn spring_chain_demo(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("spring_chain");
    let (rect, resp) = stage(ui, 330.0);
    let time = glowkit_core::time(ui);
    let dt = glowkit_core::dt(ui);
    let mut s: ChainDemo = state(ui, id, || ChainDemo {
        dots: SpringChain::new(16, rect.center()).stiffness(700.0).damping(26.0).falloff(0.97),
        ribbon: SpringChain::new(28, rect.center()).stiffness(520.0).damping(30.0).falloff(0.99),
    });
    let c = rect.center();
    let auto = c + vec2((time * 0.9).sin() * rect.width() * 0.36, (time * 1.5).sin() * rect.height() * 0.3);
    let head = resp.hover_pos().unwrap_or(auto);
    let auto2 = c + vec2((time * 0.7 + 2.0).cos() * rect.width() * 0.3, (time * 1.2).sin() * rect.height() * 0.32);
    s.dots.update(head, dt);
    s.ribbon.update(if resp.hovered() { head } else { auto2 }, dt);
    let p = ui.painter_at(rect);
    // ribbon: a smooth tapered trail
    let pts = paint::catmull_rom(s.ribbon.points(), 3);
    let n = pts.len();
    let grad = t.gradient;
    for i in (0..n - 1).rev() {
        let k = i as f32 / (n - 1) as f32;
        let w = 16.0 * (1.0 - k) + 2.0;
        p.line_segment([pts[i], pts[i + 1]], Stroke::new(w, with_alpha(color::gradient(&grad, k), 0.9 * (1.0 - k) + 0.1)));
        p.circle_filled(pts[i], w / 2.0, with_alpha(color::gradient(&grad, k), 0.9 * (1.0 - k) + 0.1));
    }
    // dots: each one chases the one in front
    let dots = s.dots.points();
    let m = dots.len();
    for (i, d) in dots.iter().enumerate().rev() {
        let k = i as f32 / (m - 1) as f32;
        let col = t.chart[1].lerp_to_gamma(t.chart[2], k);
        p.circle_filled(*d, 13.0 * (1.0 - k) + 3.0, with_alpha(col, 1.0 - k * 0.7));
    }
    p.circle_filled(dots[0], 5.0, Color32::WHITE);
    caption(ui, rect.left_bottom() + vec2(16.0, -14.0), Align2::LEFT_BOTTOM, "move the pointer here");
    set_state(ui, id, s);
    glowkit_core::animate(ui);
}

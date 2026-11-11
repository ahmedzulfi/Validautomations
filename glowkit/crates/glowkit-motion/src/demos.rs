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

/// Monospace tag drawn on a pill.
fn tag(painter: &egui::Painter, t: &Theme, pos: Pos2, align: Align2, text: &str, color: Color32) {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::monospace(10.5), color);
    let r = align.anchor_size(pos, galley.size() + vec2(14.0, 6.0));
    painter.rect_filled(r, 99, with_alpha(color, 0.12));
    painter.rect_stroke(r, 99, Stroke::new(1.0, with_alpha(color, 0.35)), StrokeKind::Inside);
    painter.galley(r.min + vec2(7.0, 3.0), galley, color);
    let _ = t;
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

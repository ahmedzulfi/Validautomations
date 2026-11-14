//! Generated typographic posters, printed with [`Ink`] so they can live on bent paper.

use super::common::{lerp_c, Ink};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Shape, Vec2};
use glowkit_core::hex;
use std::f32::consts::TAU;

/// Number of built-in poster designs.
pub(crate) const POSTERS: usize = 8;

/// Title of poster `k`.
pub(crate) fn poster_title(k: usize) -> &'static str {
    ["Crease", "Paper Plane", "Soft Hours", "Keep It Light", "Fold", "Scribble", "Heavy Stock", "Rings"][k % POSTERS]
}

/// Background color of poster `k` at `(u, v)`.
pub(crate) fn poster_bg(k: usize, u: f32, v: f32) -> Color32 {
    match k % POSTERS {
        0 => lerp_c(hex("#151515"), hex("#222222"), v),
        1 => lerp_c(hex("#E3262B"), hex("#C81E25"), v),
        2 => lerp_c(hex("#F25A24"), hex("#E8461C"), v * 0.6 + u * 0.4),
        3 => hex("#F1ECDF"),
        4 => hex("#F4F3EF"),
        5 => lerp_c(hex("#B9F24A"), hex("#9FE02E"), v),
        6 => lerp_c(hex("#3B1E8C"), hex("#2A1468"), v),
        _ => lerp_c(hex("#14453A"), hex("#0D3029"), v),
    }
}

/// Print poster `k` (laid out in `0..size`).
pub(crate) fn poster_ink(ink: &mut Ink, k: usize, size: Vec2) {
    let (w, h) = (size.x, size.y);
    let s = w / 200.0;
    let small = FontId::proportional(7.0 * s);
    match k % POSTERS {
        0 => {
            for i in 0..5 {
                let c = lerp_c(Color32::WHITE, hex("#8A8A8A"), i as f32 / 4.0);
                ink.text(pos2(w * 0.5, h * 0.14 + i as f32 * h * 0.17), Align2::CENTER_TOP, "CREASE", FontId::proportional(38.0 * s), c);
            }
        }
        1 => {
            let p = |x: f32, y: f32| pos2(x * w, y * h);
            ink.poly(vec![p(0.16, 0.34), p(0.84, 0.22), p(0.47, 0.44)], Color32::WHITE);
            ink.poly(vec![p(0.47, 0.44), p(0.84, 0.22), p(0.53, 0.6)], hex("#F3D2D2"));
            ink.text(pos2(w * 0.1, h * 0.86), Align2::LEFT_TOP, "Take off from", small.clone(), Color32::WHITE);
            ink.text(pos2(w * 0.1, h * 0.86 + 9.0 * s), Align2::LEFT_TOP, "any flat surface.", small.clone(), Color32::WHITE);
            ink.text(pos2(w * 0.9, h * 0.05), Align2::RIGHT_TOP, "/ 07", small, Color32::WHITE);
        }
        2 => {
            let c = pos2(w * 0.5, h * 0.42);
            let r = w * 0.3;
            ink.circle(c, r, hex("#FCE9D2"));
            for i in 0..6 {
                let y = c.y + r * 0.15 + i as f32 * r * 0.15;
                let hh = 1.0 + i as f32 * 0.9;
                ink.rect(egui::Rect::from_min_size(pos2(c.x - r - 2.0, y), vec2(2.0 * r + 4.0, hh * s)), 0.0, hex("#F25A24"));
            }
            ink.text(pos2(w * 0.1, h * 0.84), Align2::LEFT_TOP, "Soft Hours", FontId::proportional(22.0 * s), hex("#2B1306"));
            ink.text(pos2(w * 0.9, h * 0.05), Align2::RIGHT_TOP, "06:12 - 07:40", small, hex("#2B1306"));
        }
        3 => {
            let c = pos2(w * 0.5, h * 0.4);
            let r = w * 0.28;
            ink.ring(c, r, 1.6 * s, hex("#222222"));
            ink.line(vec![c + vec2(-r * 0.32, -r * 0.2), c + vec2(-r * 0.32, -r * 0.05)], 2.0 * s, hex("#222222"));
            ink.line(vec![c + vec2(r * 0.32, -r * 0.2), c + vec2(r * 0.32, -r * 0.05)], 2.0 * s, hex("#222222"));
            let smile: Vec<Pos2> =
                (0..=16).map(|i| {
                    let a = 0.5 + i as f32 / 16.0 * 2.1;
                    c + vec2(a.cos() * r * 0.55, a.sin() * r * 0.5 + r * 0.02)
                }).collect();
            ink.line(smile, 1.8 * s, hex("#222222"));
            ink.text(pos2(w * 0.5, h * 0.8), Align2::CENTER_TOP, "keep it light", FontId::proportional(17.0 * s), hex("#222222"));
        }
        4 => {
            let g = ink.galley("FOLD", FontId::proportional(76.0 * s), hex("#111111"));
            let gs = g.size();
            let pos = pos2(w * 0.5 + gs.y * 0.5, h * 0.5 - gs.x * 0.5);
            ink.shapes.push(Shape::Text(egui::epaint::TextShape::new(pos, g, hex("#111111")).with_angle(TAU / 4.0)));
            ink.text(pos2(w * 0.08, h * 0.05), Align2::LEFT_TOP, "VOL. 2", small, hex("#111111"));
        }
        5 => {
            let c = pos2(w * 0.5, h * 0.5);
            let pts: Vec<Pos2> = (0..120)
                .map(|i| {
                    let t = i as f32 / 119.0;
                    let a = t * TAU * 3.2;
                    let r = w * (0.08 + 0.28 * t) * (1.0 + 0.18 * (a * 2.3).sin());
                    c + vec2(a.cos() * r, a.sin() * r * 1.25)
                })
                .collect();
            ink.line(pts, 7.0 * s, hex("#0D1A06"));
            ink.text(pos2(w * 0.08, h * 0.05), Align2::LEFT_TOP, "& more", small, hex("#0D1A06"));
        }
        6 => {
            for i in 0..7 {
                let x = if i % 2 == 0 { w * 0.08 } else { w * 0.2 };
                ink.text(pos2(x, h * 0.1 + i as f32 * h * 0.115), Align2::LEFT_TOP, "300 gsm", FontId::proportional(26.0 * s), hex("#D8CCFF"));
            }
        }
        _ => {
            let c = pos2(w * 0.5, h * 0.44);
            for i in 0..7 {
                ink.ring(c, w * (0.05 + i as f32 * 0.055), 2.0 * s, lerp_c(hex("#BFF5D6"), hex("#3FB58A"), i as f32 / 6.0));
            }
            ink.text(pos2(w * 0.1, h * 0.84), Align2::LEFT_TOP, "Rings", FontId::proportional(24.0 * s), hex("#E9FFF4"));
            ink.text(pos2(w * 0.9, h * 0.05), Align2::RIGHT_TOP, "02 / 08", small, hex("#E9FFF4"));
        }
    }
}

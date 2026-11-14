use super::common::{cr, surface_hover};
use super::*;
use egui::{pos2, vec2, Align, Align2, FontId, Layout, Rect, Sense, Stroke, Ui, UiBuilder, Vec2};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::{paint, Rng, Theme};

const C: Category = Category::MicroInteractions;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Center whatever `add` lays out (left to right) inside a full-width box at least `height` tall.
/// The content size is measured on the previous frame.
fn centered(ui: &mut Ui, key: &str, height: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("micro_b_center", key));
    let size: Vec2 = glowkit_core::state(ui, id, || Vec2::ZERO);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height.max(size.y)), Sense::hover());
    let inner = Rect::from_center_size(rect.center(), size);
    let r = ui.scope_builder(
        UiBuilder::new().max_rect(Rect::from_min_max(inner.min, rect.max)).layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(28.0, 18.0);
            add(ui);
            ui.min_rect().size()
        },
    );
    if (r.inner - size).length() > 0.5 {
        glowkit_core::set_state(ui, id, r.inner);
        ui.ctx().request_repaint();
    }
}

fn crypto() -> Vec<SwapAsset> {
    vec![
        SwapAsset::new("BTC", "Bitcoin", hex("#F7931A"), 97_700.0).glyph("₿"),
        SwapAsset::new("ETH", "Ether", hex("#627EEA"), 3_412.0).glyph("Ξ"),
        SwapAsset::new("SOL", "Solana", hex("#14B8A6"), 182.4),
        SwapAsset::new("DOT", "Polkadot", hex("#E6007A"), 6.82),
        SwapAsset::new("XLM", "Stellar", hex("#18181B"), 0.42).glyph("✦"),
    ]
}

fn fiat() -> Vec<SwapAsset> {
    vec![
        SwapAsset::new("USD", "US Dollar", hex("#16A34A"), 1.0).glyph("$"),
        SwapAsset::new("EUR", "Euro", hex("#2563EB"), 1.08).glyph("€"),
        SwapAsset::new("GBP", "Pound sterling", hex("#7C3AED"), 1.27).glyph("£"),
        SwapAsset::new("JPY", "Yen", hex("#DC2626"), 0.0066).glyph("¥"),
        SwapAsset::new("CHF", "Swiss franc", hex("#EA580C"), 1.13).glyph("₣"),
    ]
}

fn people() -> Vec<Assignee> {
    vec![
        Assignee::new("Ines Calder", "Design", [hex("#F472B6"), hex("#A855F7")]),
        Assignee::new("Tomas Reyes", "Research", [hex("#60A5FA"), hex("#6366F1")]),
        Assignee::new("Wren Okafor", "Engineering", [hex("#4ADE80"), hex("#14B8A6")]),
        Assignee::new("Juno Patel", "Product", [hex("#FB923C"), hex("#EF4444")]),
    ]
}

/// Painted price card used inside the pull-to-refresh demo.
fn price_card(ui: &mut Ui, rect: Rect, seed: u64) {
    let t = Theme::of(ui);
    let p = ui.painter();
    let mut rng = Rng::new(seed * 7919 + 3);
    let price = 58_000.0 + rng.range(-1500.0, 2500.0) as f64;
    let change = rng.range(0.4, 3.2) * if rng.bool(0.7) { 1.0 } else { -1.0 };
    let x0 = rect.min.x + 16.0;
    let whole = format!("${}", asset_swap_money(price.floor()));
    let r = p.text(pos2(x0, rect.min.y + 30.0), Align2::LEFT_CENTER, &whole, FontId::proportional(24.0), t.foreground);
    let cents = format!(".{:02}", ((price.fract()) * 100.0) as u32);
    p.text(pos2(r.max.x, rect.min.y + 32.0), Align2::LEFT_CENTER, cents, FontId::proportional(15.0), t.muted_foreground);
    let up = change >= 0.0;
    let col = if up { t.success } else { t.destructive };
    let abs = price * change as f64 / 100.0;
    let line = format!("{}{} · {:.1}%", if up { "+" } else { "-" }, asset_swap_money(abs.abs()), change.abs());
    let r2 = p.text(pos2(x0, rect.min.y + 54.0), Align2::LEFT_CENTER, line, FontId::proportional(11.5), col);
    p.text(pos2(r2.max.x + 6.0, rect.min.y + 54.0), Align2::LEFT_CENTER, "today", FontId::proportional(11.5), t.muted_foreground);
    // chart
    let chart = Rect::from_min_max(pos2(x0, rect.min.y + 70.0), pos2(rect.max.x - 16.0, rect.max.y - 44.0));
    let n = 28;
    let mut v = 0.0f32;
    let pts: Vec<egui::Pos2> = (0..n)
        .map(|i| {
            v += rng.range(-0.9, 1.0) + if up { 0.25 } else { -0.2 };
            pos2(i as f32, v)
        })
        .collect();
    let (lo, hi) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), q| (a.min(q.y), b.max(q.y)));
    let pts: Vec<egui::Pos2> = pts
        .iter()
        .map(|q| pos2(chart.min.x + q.x / (n - 1) as f32 * chart.width(), chart.max.y - (q.y - lo) / (hi - lo).max(1e-3) * chart.height()))
        .collect();
    let smooth = paint::catmull_rom(&pts, 6);
    paint::area_fill(p, &smooth, chart.max.y, with_alpha(col, 0.16), with_alpha(col, 0.0));
    p.add(egui::Shape::line(smooth, Stroke::new(1.8, col)));
    if let Some(last) = pts.last() {
        p.circle(*last, 3.5, t.card, Stroke::new(1.6, col));
    }
    // range tabs
    let tabs = ["1H", "4H", "1D"];
    let tw = (rect.width() - 32.0) / 3.0;
    for (i, s) in tabs.iter().enumerate() {
        let tr = Rect::from_min_size(pos2(x0 + i as f32 * tw, rect.max.y - 36.0), vec2(tw, 24.0));
        if i == 2 {
            p.rect_filled(tr.shrink2(vec2(4.0, 0.0)), cr(8.0), surface_hover(&t));
        }
        p.text(tr.center(), Align2::CENTER_CENTER, *s, FontId::proportional(11.0), if i == 2 { t.foreground } else { t.muted_foreground });
    }
}

fn asset_swap_money(v: f64) -> String {
    asset_swap::money(v).trim_end_matches(".00").to_string()
}

pub(crate) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "label-input",
            name: "Label Input",
            category: C,
            description: "A field whose label rests inside like a placeholder and lifts into the top edge, opening a notch in the outline, when you focus it.",
            covers: &["Bencho: Label input"],
            code: r##"let mut email = String::new();
ui.add(LabelInput::new(&mut email, "Email").corner(20.0));
ui.add(LabelInput::new(&mut password, "Password").password(true));"##,
            show: |ui| {
                let mut email: String = mem(ui, "email", String::new);
                let mut pass: String = mem(ui, "pass", String::new);
                let touched: bool = mem(ui, "touched", || false);
                // while untouched, type an address now and then so the label shows its lift
                let time = glowkit_core::time(ui);
                let auto = !touched;
                if auto {
                    glowkit_core::animate(ui);
                    let demo = "hello@glowkit.rs";
                    let ph = time % 6.0;
                    let n = if ph < 1.0 { 0 } else { (((ph - 1.0) / 0.08) as usize).min(demo.len()) };
                    email = if ph > 4.8 { String::new() } else { demo[..n].to_string() };
                }
                let mut focused = false;
                centered(ui, "fields", 312.0, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 22.0;
                        let lit = auto && !email.is_empty() && (time % 6.0) < 4.8;
                        let r = ui.add(LabelInput::new(&mut email, "Email").highlight(lit));
                        let r2 = ui.add(LabelInput::new(&mut pass, "Password").password(true));
                        if (r.has_focus() || r2.has_focus()) && !touched {
                            focused = true;
                        }
                    });
                });
                if focused {
                    // the visitor took over: drop the demo text
                    set_mem(ui, "touched", true);
                    email.clear();
                }
                set_mem(ui, "email", email);
                set_mem(ui, "pass", pass);
            },
        },
        Demo {
            id: "step-player",
            name: "Step Player",
            category: C,
            description: "Press play and each step fills as a bar, then eases on to the next dot.",
            covers: &["Bencho: Step player"],
            code: r##"ui.add(StepPlayer::new(5).pace(3.0));
ui.add(StepPlayer::new(4).dark(true).pace(1.6).looping(true));"##,
            show: |ui| {
                centered(ui, "players", 312.0, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 28.0;
                        ui.add(StepPlayer::new(5).pace(2.2));
                        ui.add(StepPlayer::new(4).dark(true).pace(1.4).bar_length(44.0).looping(true));
                        ui.add(StepPlayer::new(7).stroke(true).pace(0.9).bar_length(22.0));
                    });
                });
            },
        },
        Demo {
            id: "glass-bubble",
            name: "Glass Bubble",
            category: C,
            description: "Drag a ball of glass over a picture: it magnifies the middle, bends the rim and splits the colours at the edge.",
            covers: &["Bencho: Glass bubble"],
            code: r##"ui.add(GlassBubble::new().size(300.0).bend(0.6).fringe(0.3));
ui.add(GlassBubble::new().texture(my_texture, 1.5)); // your own picture"##,
            show: |ui| {
                centered(ui, "bubbles", 320.0, |ui| {
                    ui.add(GlassBubble::new().size(290.0).scene(4));
                    ui.add(GlassBubble::new().size(290.0).scene(1).bend(0.85).fringe(0.7).lens_radius(70.0).corner(36.0));
                });
            },
        },
        Demo {
            id: "slide-confirm",
            name: "Slide to Confirm",
            category: C,
            description: "Push the handle across; past the mark it takes over, finishes itself and turns into a check.",
            covers: &["Bencho: Slide to confirm"],
            code: r##"if ui.add(SlideToConfirm::new("Slide to confirm")).changed() {
    // confirmed
}
ui.add(SlideToConfirm::new("Slide to pay").dark(true).confirmed_label("Paid"));"##,
            show: |ui| {
                centered(ui, "slides", 312.0, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 26.0;
                        ui.add(SlideToConfirm::new("Slide to confirm"));
                        ui.add(SlideToConfirm::new("Slide to pay").dark(true).confirmed_label("Paid").speed(0.8));
                    });
                });
            },
        },
        Demo {
            id: "pull",
            name: "Pull to Refresh",
            category: C,
            description: "Pull the card down and it stretches while a ring of dots fills in; let go past the line to refresh.",
            covers: &["Bencho: Pull to refresh"],
            code: r##"let out = PullToRefresh::new().size(egui::vec2(300.0, 196.0)).show(ui, |ui, rect| {
    // paint or lay out the card content in `rect`
});
if out.refreshed { /* reload */ }"##,
            show: |ui| {
                let seed: u64 = mem(ui, "seed", || 1);
                let mut next = seed;
                centered(ui, "pull", 312.0, |ui| {
                    let out = PullToRefresh::new().size(vec2(300.0, 196.0)).max_pull(110.0).show(ui, |ui, rect| price_card(ui, rect, seed));
                    if out.refreshed {
                        next = seed + 1;
                    }
                });
                set_mem(ui, "seed", next);
            },
        },
        Demo {
            id: "escape",
            name: "Escape Button",
            category: C,
            description: "A button that slides away from your cursor a few times, and then gives up.",
            covers: &["Bencho: Escape button"],
            code: r##"if ui.add(EscapeButton::new("Touch me").patience(4).radius(120.0)).clicked() {
    // finally
}"##,
            show: |ui| {
                ui.add(EscapeButton::new("Touch me").height(312.0));
            },
        },
        Demo {
            id: "liq-create",
            name: "Create Menu",
            category: C,
            description: "Press the pill and it pours open into the create menu it was all along, items rising in one by one.",
            covers: &["Bencho: Create menu"],
            code: r##"let picked = CreateMenu::new("Create")
    .item(MicroIcon::Doc, "Document")
    .item(MicroIcon::Sheet, "Spreadsheet")
    .item(MicroIcon::Board, "Board")
    .item(MicroIcon::Folder, "Folder")
    .show(ui);"##,
            show: |ui| {
                centered(ui, "menus", 312.0, |ui| {
                    let menu = |dark: bool| {
                        CreateMenu::new("Create")
                            .item(MicroIcon::Doc, "Document")
                            .item(MicroIcon::Sheet, "Spreadsheet")
                            .item(MicroIcon::Board, "Board")
                            .item(MicroIcon::Folder, "Folder")
                            .dark(dark)
                    };
                    let _ = menu(false).show(ui);
                    ui.push_id("dark", |ui| {
                        let _ = menu(true).corner(14.0).show(ui);
                    });
                });
            },
        },
        Demo {
            id: "slingshot",
            name: "Slingshot",
            category: C,
            description: "Pull the ball back: the band stretches, locks and trembles at its limit; let go and it flies, bounces off the walls and comes home.",
            covers: &["Bencho: Slingshot"],
            code: r##"ui.add(Slingshot::new().power(1.0).bounce(0.3));"##,
            show: |ui| {
                ui.add(Slingshot::new().height(312.0));
            },
        },
        Demo {
            id: "asset-swap",
            name: "Asset Swap",
            category: C,
            description: "Flip the arrow to reverse the swap on a bouncy spring; tap a coin to pick another asset.",
            covers: &["Bencho: Asset swap"],
            code: r##"let assets = vec![
    SwapAsset::new("BTC", "Bitcoin", hex("#F7931A"), 97_700.0),
    SwapAsset::new("ETH", "Ether", hex("#627EEA"), 3_412.0),
];
ui.add(AssetSwap::new(assets).amount(0.05));"##,
            show: |ui| {
                centered(ui, "swaps", 312.0, |ui| {
                    ui.add(AssetSwap::new(crypto()).amount(0.05));
                    ui.add(AssetSwap::new(fiat()).amount(250.0).bounce(0.7));
                });
            },
        },
        Demo {
            id: "like",
            name: "Like",
            category: C,
            description: "Press the heart: it pops with a burst while the count rolls up a digit.",
            covers: &["Bencho: Like"],
            code: r##"let mut liked = false;
ui.add(LikeButton::new(&mut liked, 1_299));
ui.add(LikeButton::new(&mut liked, 99).burst(LikeBurst::Firework).color(theme.foreground));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let mut l: [bool; 3] = mem(ui, "liked", || [false; 3]);
                centered(ui, "likes", 312.0, |ui| {
                    ui.add(LikeButton::new(&mut l[0], 1_299).autoplay(true));
                    ui.push_id(1, |ui| ui.add(LikeButton::new(&mut l[1], 99).burst(LikeBurst::Firework).color(t.foreground).stroke(true).autoplay(true)));
                    ui.push_id(2, |ui| ui.add(LikeButton::new(&mut l[2], 48_999).dark(true).burst_size(1.0).autoplay(true)));
                });
                set_mem(ui, "liked", l);
            },
        },
        Demo {
            id: "eye-tracker",
            name: "Eye Tracker",
            category: C,
            description: "Two eyes on a soft cube turn to follow your cursor around the block, and blink now and then.",
            covers: &["Bencho: Eye tracker"],
            code: r##"ui.add(EyeTracker::new().size(100.0));
ui.add(EyeTracker::new().shape(EyeShape::Slant).dark(true));"##,
            show: |ui| {
                ui.columns(3, |cols| {
                    cols[0].add(EyeTracker::new().height(312.0).size(96.0));
                    cols[1].add(EyeTracker::new().height(312.0).size(96.0).shape(EyeShape::Slant).dark(true).bounce(0.7));
                    cols[2].add(EyeTracker::new().height(312.0).size(84.0).shape(EyeShape::Squares).follow(0.9));
                });
            },
        },
        Demo {
            id: "todo-tower",
            name: "Todo Tower",
            category: C,
            description: "A to-do list stacked like bricks; tick one off and it slides out while the rest drop into the gap.",
            covers: &["Bencho: Todo tower"],
            code: r##"ui.add(TodoTower::new(["Water the plants", "Ship the release", "Call the bank"]).gravity(0.5));"##,
            show: |ui| {
                ui.add(
                    TodoTower::new([
                        "Water the plants",
                        "Ship the release",
                        "Book the dentist",
                        "Reply to Sam",
                        "Renew the passport",
                        "Back up the photos",
                        "Fix the bike light",
                    ])
                    .count(6)
                    .height(312.0),
                );
            },
        },
        Demo {
            id: "action-node",
            name: "Action Node",
            category: C,
            description: "An automation step card with its controls folded into the corner; hover and they fan out one by one.",
            covers: &["Bencho: Action node"],
            code: r##"if let Some(i) = ActionNode::new("Node 07", "Reads the thread, writes a summary.").show(ui) {
    // action button i was pressed
}"##,
            show: |ui| {
                centered(ui, "nodes", 312.0, |ui| {
                    let _ = ActionNode::new("Digest", "Collects the day's messages, writes a short summary and posts it to the team channel.").show(ui);
                    let _ = ActionNode::new("Triage", "Labels new issues by area and pings whoever is on call.")
                        .dark(true)
                        .buttons(3)
                        .bounce(0.7)
                        .show(ui);
                });
            },
        },
        Demo {
            id: "picker",
            name: "Assignees",
            category: C,
            description: "Assign people from a list and their faces stack up in the pill above it.",
            covers: &["Bencho: Assignees"],
            code: r##"ui.add(AssigneePicker::new(&people, &mut chosen).layout(AvatarLayout::Stack).overlap(10.0));"##,
            show: |ui| {
                let ps = people();
                let mut sel: Vec<Vec<bool>> = mem(ui, "sel", || vec![vec![true, true, false, false], vec![false, true, true, false], vec![true, false, true, true]]);
                centered(ui, "pickers", 312.0, |ui| {
                    ui.add(AssigneePicker::new(&ps, &mut sel[0]).autoplay(true));
                    ui.push_id(1, |ui| ui.add(AssigneePicker::new(&ps, &mut sel[1]).layout(AvatarLayout::Row).width(220.0).autoplay(true)));
                    ui.push_id(2, |ui| ui.add(AssigneePicker::new(&ps, &mut sel[2]).layout(AvatarLayout::Grid).width(220.0).dark(true).autoplay(true)));
                });
                set_mem(ui, "sel", sel);
            },
        },
        Demo {
            id: "palette",
            name: "Palette",
            category: C,
            description: "A set of colours that go together; press for a new palette and it washes across, swatch by swatch.",
            covers: &["Bencho: Palette"],
            code: r##"ui.add(PaletteSwatches::new().colors(4).corner(14.0));"##,
            show: |ui| {
                centered(ui, "palettes", 312.0, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 24.0;
                        ui.add(PaletteSwatches::new());
                        ui.add(PaletteSwatches::new().colors(6).swatch_size(vec2(40.0, 44.0)).corner(10.0).dark(true).morph(0.9));
                    });
                });
            },
        },
        Demo {
            id: "drag-ball",
            name: "Dragging Ball",
            category: C,
            description: "A ball you grab: it swells under the pointer, squeezes as you hold it and wobbles home when you let go.",
            covers: &["Bencho: Dragging ball"],
            code: r##"ui.add(DragBall::new().size(56.0).stretch(0.36).give(0.5));"##,
            show: |ui| {
                ui.add(DragBall::new().height(312.0).size(64.0));
            },
        },
    ]
}

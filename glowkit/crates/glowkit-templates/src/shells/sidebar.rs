//! The shell sidebar: workspace switcher, grouped navigation with icons, badges and nested
//! collapsible items, a springy active indicator, an upgrade card and the user card. Collapses
//! to an icon rail as `c` goes from 0 to 1.

use egui::{pos2, vec2, Align2, Color32, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui};
use glowkit_components::{DropdownMenu, LineIcon, MenuEntry, Placement, Tooltip, UiIcon};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::icon::AppIcon;
use super::kit;
use super::nav::{NavSection, ShellUser, Workspace};

/// Width of the collapsed icon rail.
pub(crate) const RAIL_W: f32 = 52.0;

pub(crate) struct SideCfg<'a> {
    pub id: Id,
    pub nav: &'a mut [NavSection],
    pub route: &'a mut String,
    pub workspaces: &'a [Workspace],
    pub workspace: &'a mut usize,
    pub user: Option<&'a ShellUser>,
    pub upgrade: bool,
    pub upgrade_dismissed: &'a mut bool,
    pub bg: Color32,
    pub bounds: Rect,
    /// Only show this section (dual layout secondary panel).
    pub only_section: Option<usize>,
    pub header: bool,
    pub footer_user: bool,
    pub titles: bool,
}

#[derive(Default)]
pub(crate) struct SideOut {
    pub menu: Option<String>,
    pub navigated: bool,
    pub workspace_changed: bool,
}

/// Paint a chevron (">" shape) centered at `c`, rotated by `angle` radians.
pub(crate) fn chevron(p: &Painter, c: Pos2, size: f32, angle: f32, color: Color32) {
    let (s, co) = angle.sin_cos();
    let rot = |x: f32, y: f32| pos2(c.x + (x * co - y * s) * size, c.y + (x * s + y * co) * size);
    p.add(Shape::line(vec![rot(-0.22, -0.45), rot(0.23, 0.0), rot(-0.22, 0.45)], Stroke::new(1.5, color)));
}

pub(crate) fn sidebar(ui: &mut Ui, rect: Rect, c: f32, cfg: SideCfg<'_>) -> SideOut {
    let t = Theme::of(ui);
    let id = cfg.id;
    let mut out = SideOut::default();
    let p = ui.painter_at(rect);
    let la = (1.0 - c * 1.8).clamp(0.0, 1.0);
    let mut lp = p.clone();
    lp.multiply_opacity(la);
    let pad = 8.0;
    let x0 = rect.min.x + pad;
    let inner_w = rect.width() - pad * 2.0;
    let mut y = rect.min.y + pad;
    let r6 = t.radius * 0.65;

    // ---- workspace switcher -------------------------------------------------------------
    if cfg.header && !cfg.workspaces.is_empty() {
        let ws_i = (*cfg.workspace).min(cfg.workspaces.len() - 1);
        let ws = &cfg.workspaces[ws_i];
        let hr = Rect::from_min_size(pos2(x0, y), vec2(inner_w, 48.0));
        let (resp, _) = kit::row(ui, hr, id.with("ws"), r6);
        let lx = x0 + 8.0 - 6.0 * c;
        let lr = Rect::from_min_size(pos2(lx, hr.center().y - 16.0), vec2(32.0, 32.0));
        kit::logo(&p, lr, &t, &ws.name.chars().next().unwrap_or('A').to_string());
        let tw = inner_w - 42.0 - 30.0;
        kit::text_at(&lp, pos2(lr.max.x + 10.0, hr.center().y - 8.0), Align2::LEFT_CENTER, &ws.name, 14.0, t.foreground, tw);
        kit::text_at(&lp, pos2(lr.max.x + 10.0, hr.center().y + 9.0), Align2::LEFT_CENTER, &ws.plan, 12.0, t.muted_foreground, tw);
        UiIcon::ChevronsUpDown.paint(&lp, Rect::from_center_size(pos2(hr.max.x - 14.0, hr.center().y), vec2(15.0, 15.0)), t.muted_foreground);
        if c > 0.6 {
            Tooltip::new(ws.name.clone()).placement(Placement::Right).delay(0.15).show(&resp);
        }
        let mut entries = vec![MenuEntry::label("Workspaces")];
        for (i, w) in cfg.workspaces.iter().enumerate() {
            entries.push(MenuEntry::radio(w.name.clone(), i == ws_i));
        }
        entries.push(MenuEntry::separator());
        entries.push(MenuEntry::item("Add workspace").icon(LineIcon::Plus));
        if let Some(sel) = DropdownMenu::new(entries).width(220.0).within(cfg.bounds).show(&resp) {
            if let Some(i) = cfg.workspaces.iter().position(|w| w.name == sel) {
                if i != ws_i {
                    *cfg.workspace = i;
                    out.workspace_changed = true;
                }
            } else {
                out.menu = Some(sel);
            }
        }
        y += 48.0 + 8.0;
    }

    // ---- footer heights -----------------------------------------------------------------
    let user_h = if cfg.footer_user && cfg.user.is_some() { 52.0 } else { 0.0 };
    let show_up = cfg.upgrade && !*cfg.upgrade_dismissed;
    let up_e = kit::follow(ui.ctx(), id.with("up_e"), if show_up && c < 0.5 { 1.0 } else { 0.0 }, 14.0);
    let up_h = 150.0 * up_e;
    let foot_top = rect.max.y - pad - user_h - if up_h > 0.5 { up_h + 8.0 } else { 0.0 };

    // ---- navigation list ----------------------------------------------------------------
    let list_rect = Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, foot_top - 4.0));
    let nav = cfg.nav;
    let route = cfg.route;
    let only = cfg.only_section;
    let bounds = cfg.bounds;
    let bg = cfg.bg;
    let cfg_titles = cfg.titles;
    kit::child(ui, list_rect, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let origin = ui.cursor().min.y;
                let slot = ui.painter().add(Shape::Noop);
                let mut active: Option<Rect> = None;
                let mut nav_to: Option<String> = None;
                for (si, sec) in nav.iter_mut().enumerate() {
                    if only.is_some_and(|o| o != si) {
                        continue;
                    }
                    // group title (a hairline in rail mode)
                    let th = if cfg_titles { 30.0 * (1.0 - c) + 10.0 * c } else { 6.0 };
                    let (tr, _) = ui.allocate_exact_size(vec2(rect.width(), th), Sense::hover());
                    if la > 0.0 && cfg_titles && !sec.title.is_empty() {
                        let mut tp = ui.painter().clone();
                        tp.multiply_opacity(la);
                        kit::text_at(&tp, pos2(x0 + 8.0, tr.center().y + 2.0), Align2::LEFT_CENTER, &sec.title, 12.0, t.muted_foreground, inner_w - 16.0);
                    }
                    if c > 0.5 && si > 0 {
                        ui.painter().line_segment([pos2(x0 + 8.0, tr.center().y), pos2(x0 + inner_w - 8.0, tr.center().y)], Stroke::new(1.0, with_alpha(t.border, (c - 0.5) * 2.0)));
                    }
                    for (ii, item) in sec.items.iter_mut().enumerate() {
                        let (rr, _) = ui.allocate_exact_size(vec2(rect.width(), 34.0), Sense::hover());
                        let r = Rect::from_min_size(pos2(x0, rr.min.y + 1.0), vec2(inner_w, 32.0));
                        let rid = id.with(("item", si, ii));
                        let resp = ui.interact(r, rid, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                        let hv = kit::hover(ui, rid, resp.hovered());
                        let has_kids = !item.children.is_empty();
                        let kids_visible = item.expanded && c < 0.5;
                        let is_active = *route == item.route || (has_kids && !kids_visible && item.contains(route));
                        if is_active {
                            active = Some(r);
                        } else if hv > 0.01 {
                            ui.painter().rect_filled(r, r6, kit::hover_fill(&t, hv * 1.2));
                        }
                        let pnt = ui.painter().clone();
                        let col = if is_active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
                        let ic = pos2(rect.min.x + RAIL_W / 2.0, r.center().y);
                        if let Some(icon) = item.icon {
                            icon.paint(&pnt, Rect::from_center_size(ic, vec2(16.0, 16.0)), if is_active { t.accent } else { col });
                        }
                        let mut lab = pnt.clone();
                        lab.multiply_opacity(la);
                        let mut right = r.max.x - 8.0;
                        if has_kids {
                            let e = kit::follow(ui.ctx(), rid.with("chev"), if item.expanded { 1.0 } else { 0.0 }, 16.0);
                            chevron(&lab, pos2(right - 6.0, r.center().y), 9.0, e * std::f32::consts::FRAC_PI_2, t.muted_foreground);
                            right -= 20.0;
                        }
                        if let Some(b) = &item.badge {
                            let br = kit::pill(&lab, pos2(right, r.center().y), true, b, if is_active { t.accent } else { t.muted_foreground }, if is_active { with_alpha(t.accent, 0.16) } else { t.muted }, 11.0);
                            right = br.min.x - 6.0;
                            if c > 0.5 {
                                pnt.circle_filled(ic + vec2(8.0, -7.0), 4.5, bg);
                                pnt.circle_filled(ic + vec2(8.0, -7.0), 3.0, t.accent);
                            }
                        }
                        kit::text_at(&lab, pos2(r.min.x + 34.0, r.center().y), Align2::LEFT_CENTER, &item.label, 13.5, col, right - r.min.x - 40.0);
                        if c > 0.6 {
                            Tooltip::new(item.label.clone()).placement(Placement::Right).delay(0.1).show(&resp);
                        }
                        if resp.clicked() {
                            if has_kids {
                                if c > 0.5 {
                                    item.expanded = true;
                                    nav_to = Some(item.children[0].route.clone());
                                } else {
                                    item.expanded = !item.expanded;
                                }
                            } else {
                                nav_to = Some(item.route.clone());
                            }
                        }
                        // nested children
                        if has_kids {
                            let e = kit::follow(ui.ctx(), rid.with("exp"), if kids_visible { 1.0 } else { 0.0 }, 16.0);
                            if e > 0.001 {
                                let full = item.children.len() as f32 * 30.0 + 4.0;
                                let (cr, _) = ui.allocate_exact_size(vec2(rect.width(), full * e), Sense::hover());
                                let clip = cr.intersect(ui.clip_rect());
                                let cp = ui.painter().with_clip_rect(clip);
                                let mut cpl = cp.clone();
                                cpl.multiply_opacity(e * la);
                                cpl.line_segment([pos2(ic.x, cr.min.y), pos2(ic.x, cr.min.y + full - 6.0)], Stroke::new(1.0, t.border));
                                for (ci, ch) in item.children.iter().enumerate() {
                                    let crr = Rect::from_min_size(pos2(ic.x + 10.0, cr.min.y + ci as f32 * 30.0), vec2(r.max.x - ic.x - 10.0, 28.0));
                                    let cid = rid.with(("child", ci));
                                    let ch_active = *route == ch.route;
                                    let hv2 = if e > 0.6 && clip.intersects(crr) {
                                        let cresp = ui.interact(crr.intersect(clip), cid, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                                        if cresp.clicked() {
                                            nav_to = Some(ch.route.clone());
                                        }
                                        kit::hover(ui, cid, cresp.hovered())
                                    } else {
                                        0.0
                                    };
                                    if ch_active {
                                        active = Some(crr);
                                    } else if hv2 > 0.01 {
                                        cpl.rect_filled(crr, r6, kit::hover_fill(&t, hv2 * 1.2));
                                    }
                                    let ccol = if ch_active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv2) };
                                    let mut cr_right = crr.max.x - 8.0;
                                    if let Some(b) = &ch.badge {
                                        let br = kit::pill(&cpl, pos2(cr_right, crr.center().y), true, b, t.muted_foreground, t.muted, 10.5);
                                        cr_right = br.min.x - 6.0;
                                    }
                                    kit::text_at(&cpl, pos2(crr.min.x + 12.0, crr.center().y), Align2::LEFT_CENTER, &ch.label, 13.0, ccol, cr_right - crr.min.x - 14.0);
                                }
                            }
                        }
                    }
                    ui.add_space(4.0);
                }
                // springy active indicator, painted beneath the rows
                if let Some(ar) = active {
                    let ay = kit::spring(ui.ctx(), id.with("ind_y"), ar.min.y - origin, 380.0, 30.0) + origin;
                    let ax = kit::follow(ui.ctx(), id.with("ind_x"), ar.min.x, 22.0);
                    let aw = kit::follow(ui.ctx(), id.with("ind_w"), ar.width(), 22.0);
                    let ah = kit::follow(ui.ctx(), id.with("ind_h"), ar.height(), 22.0);
                    let r = Rect::from_min_size(pos2(ax, ay), vec2(aw, ah));
                    let bar = Rect::from_min_size(pos2(rect.min.x + 1.0, ay + ah / 2.0 - 8.0), vec2(3.0, 16.0));
                    ui.painter().set(
                        slot,
                        Shape::Vec(vec![
                            Shape::rect_filled(r, r6, kit::active_fill(&t, bg)),
                            Shape::rect_filled(bar, 2.0, t.accent),
                        ]),
                    );
                }
                if let Some(rt) = nav_to {
                    if *route != rt {
                        *route = rt;
                        out.navigated = true;
                    }
                }
            });
    });
    let _ = bounds;
    // soft fade where the list scrolls under the footer
    let fade = Rect::from_min_max(pos2(rect.min.x + 1.0, list_rect.max.y - 18.0), pos2(rect.max.x - 1.0, list_rect.max.y));
    kit::vgrad(&p, fade, with_alpha(bg, 0.0), bg);

    // ---- upgrade card -------------------------------------------------------------------
    if up_h > 0.5 {
        let ur = Rect::from_min_size(pos2(x0, foot_top), vec2(inner_w, up_h));
        let full = Rect::from_min_size(ur.min, vec2(inner_w, 150.0));
        let cp = p.with_clip_rect(ur.intersect(p.clip_rect()));
        let mut cp = cp;
        cp.multiply_opacity(up_e * la);
        let fill = mix(t.card, t.accent, if t.dark { 0.06 } else { 0.035 });
        cp.rect_filled(full, t.radius, fill);
        let g = t.gradient;
        paint::gradient_border(&cp, full, t.radius, 1.0, |u| with_alpha(gradient(&[g[0], g[1], g[2], g[0]], u), 0.7));
        paint::glow(&cp, full.left_top() + vec2(22.0, 22.0), 28.0, with_alpha(t.accent, 0.18));
        AppIcon::Ui(UiIcon::Sparkles).paint(&cp, Rect::from_center_size(full.left_top() + vec2(22.0, 22.0), vec2(16.0, 16.0)), t.accent);
        kit::text_at(&cp, full.left_top() + vec2(38.0, 22.0), Align2::LEFT_CENTER, "Upgrade to Pro", 13.5, t.foreground, inner_w - 70.0);
        let body = kit::wrapped(&cp, "Unlimited projects, SSO and priority support.", 12.0, t.muted_foreground, inner_w - 28.0, 2);
        cp.galley(full.left_top() + vec2(14.0, 38.0), body, t.muted_foreground);
        let by = full.min.y + 82.0;
        kit::text_at(&cp, pos2(full.min.x + 14.0, by), Align2::LEFT_CENTER, "Storage", 11.5, t.muted_foreground, 80.0);
        kit::text_at(&cp, pos2(full.max.x - 14.0, by), Align2::RIGHT_CENTER, "7.2 of 10 GB", 11.5, t.foreground, 100.0);
        let track = Rect::from_min_size(pos2(full.min.x + 14.0, by + 10.0), vec2(inner_w - 28.0, 5.0));
        cp.rect_filled(track, 3.0, with_alpha(t.foreground, 0.08));
        paint::gradient_rect(&cp, Rect::from_min_size(track.min, vec2(track.width() * 0.72, 5.0)), 2.5, 0.0, &[g[0], g[1]]);
        if up_e > 0.95 {
            let btn = Rect::from_min_size(pos2(full.min.x + 12.0, full.max.y - 40.0), vec2(inner_w - 24.0, 28.0));
            if kit::button_at(ui, btn, "Upgrade plan", glowkit_components::ButtonVariant::Default).clicked() {
                out.menu = Some("Upgrade plan".into());
            }
            let xr = Rect::from_center_size(full.right_top() + vec2(-16.0, 16.0), vec2(20.0, 20.0));
            let xresp = kit::icon_button(ui, xr, id.with("up_x"), LineIcon::X.into(), false);
            kit::tip(&xresp, "Dismiss", None);
            if xresp.clicked() {
                *cfg.upgrade_dismissed = true;
            }
        }
    }

    // ---- user card ----------------------------------------------------------------------
    if let (true, Some(user)) = (cfg.footer_user, cfg.user) {
        let fr = Rect::from_min_size(pos2(x0, rect.max.y - pad - 48.0), vec2(inner_w, 48.0));
        let (resp, _) = kit::row(ui, fr, id.with("user"), r6);
        let ac = pos2(rect.min.x + RAIL_W / 2.0, fr.center().y);
        kit::avatar(ui, Rect::from_center_size(ac, vec2(32.0, 32.0)), &user.name);
        let tw = inner_w - 64.0;
        kit::text_at(&lp, pos2(ac.x + 24.0, fr.center().y - 8.0), Align2::LEFT_CENTER, &user.name, 13.5, t.foreground, tw);
        kit::text_at(&lp, pos2(ac.x + 24.0, fr.center().y + 9.0), Align2::LEFT_CENTER, &user.email, 12.0, t.muted_foreground, tw);
        UiIcon::ChevronsUpDown.paint(&lp, Rect::from_center_size(pos2(fr.max.x - 14.0, fr.center().y), vec2(15.0, 15.0)), t.muted_foreground);
        if c > 0.6 {
            Tooltip::new(user.name.clone()).placement(Placement::Right).delay(0.15).show(&resp);
        }
        if let Some(sel) = DropdownMenu::new(user_menu(user)).width(230.0).within(cfg.bounds).show(&resp) {
            out.menu = Some(sel);
        }
    }
    out
}

/// The standard account menu.
pub(crate) fn user_menu(user: &ShellUser) -> Vec<MenuEntry> {
    vec![
        MenuEntry::label(format!("{}  ·  {}", user.name, user.email)),
        MenuEntry::separator(),
        MenuEntry::item("Account").icon(LineIcon::User).shortcut("Ctrl+Shift+P"),
        MenuEntry::item("Billing").icon(LineIcon::CreditCard),
        MenuEntry::item("Notifications").icon(LineIcon::Bell),
        MenuEntry::item("Settings").icon(LineIcon::Settings).shortcut("Ctrl+,"),
        MenuEntry::separator(),
        MenuEntry::item("Log out").icon(LineIcon::LogOut).destructive(),
    ]
}

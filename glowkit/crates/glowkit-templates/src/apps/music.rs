//! Music player template: library, track list, queue and a now-playing bar.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, Key, Rect, Sense, Stroke, Ui};
use glowkit_components::{toast, LineIcon, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// A track in the [`MusicApp`] library.
#[derive(Clone, Debug, PartialEq)]
pub struct MusicTrack {
    /// Title.
    pub title: String,
    /// Artist.
    pub artist: String,
    /// Album.
    pub album: String,
    /// Length in seconds.
    pub secs: f32,
    /// Liked.
    pub liked: bool,
    /// Cover art seed.
    pub art: usize,
}

/// A music player inside an [`AppShell`]: library sidebar, a playlist hero with play / shuffle,
/// album shelf, a track list with live equalizer bars on the playing track, an up-next queue
/// and a now-playing bar with a draggable progress bar, volume, shuffle and repeat. Space
/// plays / pauses, ← / → skip.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::MusicApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct MusicApp {
    /// Active route.
    pub route: String,
    /// Library navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Tracks.
    pub tracks: Vec<MusicTrack>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    current: usize,
    playing: bool,
    pos: f32,
    last_t: f64,
    volume: f32,
    shuffle: bool,
    repeat: bool,
}

impl Default for MusicApp {
    fn default() -> Self {
        let tr = |t: &str, a: &str, al: &str, s: f32, art: usize| MusicTrack { title: t.into(), artist: a.into(), album: al.into(), secs: s, liked: art % 3 == 0, art };
        Self {
            route: "playlist/late-night".into(),
            nav: vec![
                NavSection::new("Discover").item(NavItem::new("listen", "Listen now").icon(AppIcon::Play)).item(NavItem::new("browse", "Browse").icon(LineIcon::Grid)).item(NavItem::new("radio", "Radio").icon(AppIcon::Radio)),
                NavSection::new("Library")
                    .item(NavItem::new("songs", "Songs").icon(LineIcon::Music))
                    .item(NavItem::new("albums", "Albums").icon(AppIcon::Disc))
                    .item(NavItem::new("artists", "Artists").icon(AppIcon::Mic))
                    .item(NavItem::new("made", "Made for you").icon(LineIcon::Heart).badge("New")),
                NavSection::new("Playlists")
                    .item(NavItem::new("playlist/late-night", "Late night coding").icon(AppIcon::Library))
                    .item(NavItem::new("playlist/focus", "Deep focus").icon(AppIcon::Library))
                    .item(NavItem::new("playlist/run", "Morning run").icon(AppIcon::Library)),
            ],
            notifications: demo_notes(),
            tracks: vec![
                tr("Neon Tides", "Lumen", "Afterglow", 214.0, 0),
                tr("Glass Cathedral", "Mira Vale", "Prisms", 251.0, 1),
                tr("Compile & Chill", "Byte Theory", "Runtime", 187.0, 2),
                tr("Slow Orbit", "Kaito", "Satellites", 302.0, 3),
                tr("Paper Lanterns", "Mira Vale", "Prisms", 198.0, 4),
                tr("Undertow", "The Quiet Hours", "Low Tide", 243.0, 5),
                tr("Borrowed Light", "Lumen", "Afterglow", 226.0, 0),
                tr("Night Shift", "Byte Theory", "Runtime", 275.0, 2),
                tr("Velvet Static", "Kaito", "Satellites", 209.0, 3),
            ],
            scoped_theme: false,
            current: 2,
            playing: true,
            pos: 61.0,
            last_t: -1.0,
            volume: 0.7,
            shuffle: false,
            repeat: false,
        }
    }
}

fn mmss(s: f32) -> String {
    let s = s.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Generated cover art.
fn cover(p: &egui::Painter, r: Rect, seed: usize, t: &Theme) {
    let a = t.chart[seed % 6];
    let b = t.chart[(seed + 3) % 6];
    paint::gradient_rect(p, r, (r.width() * 0.08).min(10.0), 30.0 + seed as f32 * 40.0, &[a, mix(a, b, 0.5), b]);
    let c = p.with_clip_rect(r.intersect(p.clip_rect()));
    for k in 0..3 {
        c.circle_stroke(r.center() + vec2((seed as f32 * 7.0 + k as f32 * 9.0) % 20.0 - 10.0, 0.0), r.width() * (0.18 + 0.12 * k as f32), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.22)));
    }
}

impl MusicApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let now = ui.input(|i| i.time);
        if self.playing {
            if self.last_t >= 0.0 {
                self.pos += (now - self.last_t) as f32;
            }
            ui.ctx().request_repaint();
        }
        self.last_t = now;
        if self.pos >= self.tracks[self.current].secs {
            self.skip(1);
        }
        let typing = ui.memory(|m| m.focused().is_some());
        if !typing {
            let (sp, l, r) = ui.input(|i| (i.key_pressed(Key::Space), i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight)));
            if sp {
                self.playing = !self.playing;
            }
            if l {
                self.skip(-1);
            }
            if r {
                self.skip(1);
            }
        }
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(vec![Workspace::new("Soundwave", "Premium · Family")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("music_app")
            .show(ui, |ui, sc| {
                let bar = Rect::from_min_max(pos2(sc.rect.min.x, sc.rect.max.y - 80.0), sc.rect.max);
                let body = Rect::from_min_max(sc.rect.min, pos2(sc.rect.max.x, bar.min.y));
                if sc.route.starts_with("playlist/") {
                    self.playlist(ui, body);
                } else {
                    placeholder(ui, body.shrink(24.0), sc.route);
                }
                self.now_playing(ui, bar);
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn skip(&mut self, d: i32) {
        let n = self.tracks.len() as i32;
        if self.pos > 3.0 && d < 0 {
            self.pos = 0.0;
            return;
        }
        self.current = if self.shuffle { (self.current * 7 + 3) % n as usize } else if self.repeat && d > 0 && self.pos >= self.tracks[self.current].secs { self.current } else { (self.current as i32 + d).rem_euclid(n) as usize };
        self.pos = 0.0;
    }

    fn playlist(&mut self, ui: &mut Ui, body: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("music");
        let loading = kit::loading(ui, id, 0.5);
        let queue_w = if body.width() > 900.0 { 280.0 } else { 0.0 };
        let parts = kit::split_x(body, &[0.0, queue_w], 0.0);
        let main = parts[0].shrink(24.0);
        let p = ui.painter().clone();
        // hero
        let art = Rect::from_min_size(main.min, vec2(150.0, 150.0));
        if loading {
            kit::skel_r(ui, art, 12.0);
        } else {
            paint::soft_shadow(&p, art, 12.0, 20.0, vec2(0.0, 8.0), with_alpha(t.chart[0], 0.3));
            cover(&p, art, 0, &t);
            AppIcon::Ui(UiIcon::Code).paint(&p, Rect::from_center_size(art.center(), vec2(46.0, 46.0)), Color32::WHITE);
        }
        let tx = art.max.x + 22.0;
        kit::text_at(&p, pos2(tx, art.min.y + 20.0), Align2::LEFT_CENTER, "PLAYLIST", 11.5, t.muted_foreground, 200.0);
        kit::text_at(&p, pos2(tx, art.min.y + 56.0), Align2::LEFT_CENTER, "Late night coding", 30.0, t.foreground, main.max.x - tx);
        let total: f32 = self.tracks.iter().map(|x| x.secs).sum();
        kit::text_at(&p, pos2(tx, art.min.y + 90.0), Align2::LEFT_CENTER, &format!("Ada Lovelace · {} songs · {} min", self.tracks.len(), (total / 60.0) as u32), 13.0, t.muted_foreground, main.max.x - tx);
        // play + shuffle
        let pb = Rect::from_center_size(pos2(tx + 24.0, art.max.y - 22.0), vec2(44.0, 44.0));
        let presp = ui.interact(pb, id.with("hero_play"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hv = kit::hover(ui, id.with("hero_play"), presp.hovered());
        p.circle_filled(pb.center(), 22.0 + hv * 1.5, t.accent);
        (if self.playing { AppIcon::Pause } else { AppIcon::Play }).paint(&p, Rect::from_center_size(pb.center() + vec2(if self.playing { 0.0 } else { 1.5 }, 0.0), vec2(18.0, 18.0)), t.accent_foreground);
        kit::tip(&presp, if self.playing { "Pause" } else { "Play" }, Some("Space"));
        if presp.clicked() {
            self.playing = !self.playing;
        }
        let sb = Rect::from_center_size(pos2(pb.max.x + 26.0, pb.center().y), vec2(34.0, 34.0));
        let s = kit::icon_button(ui, sb, id.with("hero_shuffle"), AppIcon::Shuffle, self.shuffle);
        kit::tip(&s, "Shuffle", None);
        if s.clicked() {
            self.shuffle = !self.shuffle;
            toast(ui.ctx(), Toast::new(if self.shuffle { "Shuffle on" } else { "Shuffle off" }));
        }
        // track list
        let list = Rect::from_min_max(pos2(main.min.x, art.max.y + 24.0), main.max);
        let cols = [36.0, 0.0, 0.0, 40.0, 56.0];
        let head = Rect::from_min_size(list.min, vec2(list.width(), 28.0));
        for (c, h) in kit::split_x(head.shrink2(vec2(8.0, 0.0)), &cols, 10.0).iter().zip(["#", "Title", "Album", "", "Time"]) {
            kit::text_at(&p, if h == "Time" { c.right_center() } else { c.left_center() }, if h == "Time" { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, h, 12.0, t.muted_foreground, c.width());
        }
        kit::hline(&p, list.min.x, list.max.x, head.max.y, &t);
        let rows = Rect::from_min_max(pos2(list.min.x, head.max.y + 6.0), list.max);
        let mut play = None;
        let time = glowkit_core::time(ui);
        kit::child(ui, rows, |ui| {
            kit::scroll(ui, "music_tracks", |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, tr) in self.tracks.iter_mut().enumerate() {
                    let r = kit::alloc_row(ui, 52.0);
                    if loading {
                        kit::skel_r(ui, Rect::from_min_size(r.min + vec2(52.0, 8.0), vec2(36.0, 36.0)), 6.0);
                        kit::skel(ui, Rect::from_min_size(r.min + vec2(100.0, 14.0), vec2(160.0, 9.0)));
                        continue;
                    }
                    let rid = id.with(("t", i));
                    let (resp, hv) = kit::row(ui, r, rid, t.radius * 0.6);
                    let cur = i == self.current;
                    let pp = ui.painter().clone();
                    if cur {
                        pp.rect_filled(r, t.radius * 0.6, with_alpha(t.accent, 0.08));
                    }
                    let cells = kit::split_x(r.shrink2(vec2(8.0, 0.0)), &cols, 10.0);
                    if cur && self.playing {
                        for k in 0..3 {
                            let h = 4.0 + 10.0 * (0.5 + 0.5 * (time * (5.0 + k as f32 * 1.7) + k as f32).sin());
                            let x = cells[0].min.x + 4.0 + k as f32 * 5.0;
                            pp.rect_filled(Rect::from_min_max(pos2(x, r.center().y + 7.0 - h), pos2(x + 3.0, r.center().y + 7.0)), 1.0, t.accent);
                        }
                    } else if hv > 0.5 {
                        AppIcon::Play.paint(&pp, Rect::from_center_size(pos2(cells[0].min.x + 9.0, r.center().y), vec2(12.0, 12.0)), t.foreground);
                    } else {
                        kit::text_at(&pp, pos2(cells[0].min.x + 4.0, r.center().y), Align2::LEFT_CENTER, &(i + 1).to_string(), 13.0, if cur { t.accent } else { t.muted_foreground }, 30.0);
                    }
                    let ar = Rect::from_min_size(pos2(cells[1].min.x, r.center().y - 18.0), vec2(36.0, 36.0));
                    cover(&pp, ar, tr.art, &t);
                    kit::text_at(&pp, pos2(ar.max.x + 12.0, r.center().y - 8.0), Align2::LEFT_CENTER, &tr.title, 13.5, if cur { t.accent } else { t.foreground }, cells[1].width() - 50.0);
                    kit::text_at(&pp, pos2(ar.max.x + 12.0, r.center().y + 9.0), Align2::LEFT_CENTER, &tr.artist, 12.0, t.muted_foreground, cells[1].width() - 50.0);
                    kit::text_at(&pp, cells[2].left_center(), Align2::LEFT_CENTER, &tr.album, 13.0, t.muted_foreground, cells[2].width());
                    let hr = Rect::from_center_size(cells[3].center(), vec2(28.0, 28.0));
                    let hresp = ui.interact(hr, rid.with("like"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    if tr.liked {
                        filled_heart(&pp, hr.center(), 6.5, t.destructive);
                    } else if hv > 0.3 || hresp.hovered() {
                        LineIcon::Heart.paint(&pp, Rect::from_center_size(hr.center(), vec2(15.0, 15.0)), t.muted_foreground);
                    }
                    if hresp.clicked() {
                        tr.liked = !tr.liked;
                        toast(ui.ctx(), Toast::new(if tr.liked { "Added to Liked Songs" } else { "Removed from Liked Songs" }).description(tr.title.clone()));
                    }
                    kit::text_at(&pp, cells[4].right_center(), Align2::RIGHT_CENTER, &mmss(tr.secs), 12.5, t.muted_foreground, 60.0);
                    if resp.double_clicked() || (resp.clicked() && !hresp.clicked() && hv > 0.5 && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pp| cells[0].contains(pp))) {
                        play = Some(i);
                    }
                }
            });
        });
        if let Some(i) = play {
            self.current = i;
            self.pos = 0.0;
            self.playing = true;
        }
        // queue
        if queue_w > 0.0 {
            let q = parts[1];
            kit::vline(&p, q.min.x, q.min.y, q.max.y, &t);
            let qr = q.shrink(18.0);
            kit::text_at(&p, qr.min + vec2(0.0, 8.0), Align2::LEFT_CENTER, "Now playing", 13.0, t.muted_foreground, 200.0);
            let c = &self.tracks[self.current];
            let big = Rect::from_min_size(qr.min + vec2(0.0, 24.0), vec2(qr.width(), qr.width() * 0.62));
            cover(&p, big, c.art, &t);
            kit::text_at(&p, pos2(qr.min.x, big.max.y + 20.0), Align2::LEFT_CENTER, &c.title, 15.0, t.foreground, qr.width());
            kit::text_at(&p, pos2(qr.min.x, big.max.y + 40.0), Align2::LEFT_CENTER, &format!("{} · {}", c.artist, c.album), 12.5, t.muted_foreground, qr.width());
            let mut y = big.max.y + 72.0;
            kit::text_at(&p, pos2(qr.min.x, y), Align2::LEFT_CENTER, "Up next", 13.0, t.muted_foreground, 200.0);
            y += 16.0;
            for k in 1..=6 {
                let i = (self.current + k) % self.tracks.len();
                let r = Rect::from_min_size(pos2(qr.min.x - 6.0, y), vec2(qr.width() + 12.0, 46.0));
                if r.max.y > qr.max.y {
                    break;
                }
                y += 48.0;
                let (resp, _) = kit::row(ui, r, id.with(("q", k)), t.radius * 0.6);
                let tr = &self.tracks[i];
                let pp = ui.painter().clone();
                cover(&pp, Rect::from_min_size(r.min + vec2(6.0, 5.0), vec2(36.0, 36.0)), tr.art, &t);
                kit::text_at(&pp, pos2(r.min.x + 52.0, r.center().y - 8.0), Align2::LEFT_CENTER, &tr.title, 13.0, t.foreground, r.width() - 100.0);
                kit::text_at(&pp, pos2(r.min.x + 52.0, r.center().y + 9.0), Align2::LEFT_CENTER, &tr.artist, 11.5, t.muted_foreground, r.width() - 100.0);
                kit::text_at(&pp, pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, &mmss(tr.secs), 11.5, t.muted_foreground, 50.0);
                if resp.clicked() {
                    self.current = i;
                    self.pos = 0.0;
                    self.playing = true;
                }
            }
        }
    }

    fn now_playing(&mut self, ui: &mut Ui, bar: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("np");
        let p = ui.painter().clone();
        p.rect_filled(bar, 0.0, kit::chrome_bg(&t));
        kit::hline(&p, bar.min.x, bar.max.x, bar.min.y, &t);
        let cy = bar.center().y;
        let c = self.tracks[self.current].clone();
        let ar = Rect::from_min_size(pos2(bar.min.x + 18.0, cy - 24.0), vec2(48.0, 48.0));
        cover(&p, ar, c.art, &t);
        let info_w = (bar.width() * 0.26).min(260.0);
        kit::text_at(&p, pos2(ar.max.x + 12.0, cy - 9.0), Align2::LEFT_CENTER, &c.title, 13.5, t.foreground, info_w - 70.0);
        kit::text_at(&p, pos2(ar.max.x + 12.0, cy + 10.0), Align2::LEFT_CENTER, &c.artist, 12.0, t.muted_foreground, info_w - 70.0);
        // transport
        let mid = bar.center().x;
        let buttons: [(AppIcon, &str, f32); 5] = [(AppIcon::Shuffle, "Shuffle", -86.0), (AppIcon::SkipBack, "Previous  ←", -44.0), (if self.playing { AppIcon::Pause } else { AppIcon::Play }, "Play / pause  Space", 0.0), (AppIcon::SkipForward, "Next  →", 44.0), (AppIcon::Repeat, "Repeat", 86.0)];
        for (k, (icon, label, dx)) in buttons.iter().enumerate() {
            let center = pos2(mid + dx, cy - 10.0);
            if k == 2 {
                let r = Rect::from_center_size(center, vec2(36.0, 36.0));
                let resp = ui.interact(r, id.with("pp"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                let hv = kit::hover(ui, id.with("pp"), resp.hovered());
                p.circle_filled(center, 17.0 + hv, t.foreground);
                icon.paint(&p, Rect::from_center_size(center + vec2(if self.playing { 0.0 } else { 1.0 }, 0.0), vec2(14.0, 14.0)), t.background);
                kit::tip(&resp, label, None);
                if resp.clicked() {
                    self.playing = !self.playing;
                }
            } else {
                let active = (k == 0 && self.shuffle) || (k == 4 && self.repeat);
                let r = Rect::from_center_size(center, vec2(30.0, 30.0));
                let b = kit::icon_button(ui, r, id.with(("b", k)), *icon, false);
                if active {
                    p.circle_filled(center + vec2(0.0, 13.0), 2.0, t.accent);
                }
                kit::tip(&b, label, None);
                if b.clicked() {
                    match k {
                        0 => self.shuffle = !self.shuffle,
                        1 => self.skip(-1),
                        3 => self.skip(1),
                        _ => self.repeat = !self.repeat,
                    }
                }
            }
        }
        // progress
        let pw = (bar.width() * 0.4).clamp(200.0, 520.0);
        let track = Rect::from_center_size(pos2(mid, cy + 20.0), vec2(pw, 4.0));
        let hit = track.expand2(vec2(0.0, 8.0));
        let resp = ui.interact(hit, id.with("seek"), Sense::click_and_drag()).on_hover_cursor(CursorIcon::PointingHand);
        if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            self.pos = ((pp.x - track.min.x) / track.width()).clamp(0.0, 1.0) * c.secs;
        }
        let hv = kit::hover(ui, id.with("seek"), resp.hovered() || resp.dragged());
        let f = (self.pos / c.secs).clamp(0.0, 1.0);
        p.rect_filled(track, 2.0, with_alpha(t.foreground, 0.12));
        p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * f, 4.0)), 2.0, if hv > 0.1 { t.accent } else { t.foreground });
        if hv > 0.01 {
            p.circle_filled(pos2(track.min.x + track.width() * f, track.center().y), 6.0 * hv, t.foreground);
        }
        kit::text_at(&p, pos2(track.min.x - 10.0, track.center().y), Align2::RIGHT_CENTER, &mmss(self.pos), 11.5, t.muted_foreground, 50.0);
        kit::text_at(&p, pos2(track.max.x + 10.0, track.center().y), Align2::LEFT_CENTER, &mmss(c.secs), 11.5, t.muted_foreground, 50.0);
        // volume
        let vt = Rect::from_min_size(pos2(bar.max.x - 130.0, cy - 2.0), vec2(100.0, 4.0));
        AppIcon::Volume.paint(&p, Rect::from_center_size(pos2(vt.min.x - 18.0, cy), vec2(16.0, 16.0)), t.muted_foreground);
        let vresp = ui.interact(vt.expand2(vec2(0.0, 8.0)), id.with("vol"), Sense::click_and_drag()).on_hover_cursor(CursorIcon::PointingHand);
        if let Some(pp) = vresp.interact_pointer_pos().filter(|_| vresp.dragged() || vresp.clicked()) {
            self.volume = ((pp.x - vt.min.x) / vt.width()).clamp(0.0, 1.0);
        }
        p.rect_filled(vt, 2.0, with_alpha(t.foreground, 0.12));
        p.rect_filled(Rect::from_min_size(vt.min, vec2(vt.width() * self.volume, 4.0)), 2.0, t.foreground);
        let _ = mix;
    }
}

/// A solid heart centered at `c` (half-width `r`).
fn filled_heart(p: &egui::Painter, c: egui::Pos2, r: f32, col: Color32) {
    let pts: Vec<egui::Pos2> = (0..48)
        .map(|i| {
            let a = i as f32 / 48.0 * std::f32::consts::TAU;
            let x = 16.0 * a.sin().powi(3);
            let y = 13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos();
            c + vec2(x, -y - 2.0) * (r / 16.0)
        })
        .collect();
    // the curve is not convex at the top notch: fill as two convex halves split on the axis
    let (l, rr): (Vec<_>, Vec<_>) = pts.iter().partition(|q| q.x <= c.x);
    for mut half in [l, rr] {
        half.push(c + vec2(0.0, -0.3 * r));
        let centroid = half.iter().fold(egui::Vec2::ZERO, |a, q| a + q.to_vec2()) / half.len() as f32;
        half.sort_by(|a, b| (a.y - centroid.y).atan2(a.x - centroid.x).total_cmp(&(b.y - centroid.y).atan2(b.x - centroid.x)));
        p.add(egui::Shape::convex_polygon(half, col, Stroke::NONE));
    }
}

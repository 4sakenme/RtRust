//! Pure-Rust Edit-Mode 3D Camera Controller (`Fly` / `Orbit`, Virtual Fly Joystick,
//! Altitude ▲/▼, 🎯 Focus, Speed slider, and non-conflicting 1-finger touch look/orbit).

use std::cell::RefCell;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use glam::Vec3;

use crate::app::Editor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditCamMode {
    Fly,
    Orbit,
}

pub struct EditCamState {
    pub mode: EditCamMode,
    pub speed_mul: f32,
    pub free_look: bool,
    pub joy_vec: Vec2,
    pub joy_touching: bool,
    pub up_held: bool,
    pub down_held: bool,
    pub smooth_target: Option<(Vec3, Vec3)>, // (cam.pos, cam.pivot)
}

impl Default for EditCamState {
    fn default() -> Self {
        Self {
            mode: EditCamMode::Fly,
            speed_mul: 1.0,
            free_look: false,
            joy_vec: Vec2::ZERO,
            joy_touching: false,
            up_held: false,
            down_held: false,
            smooth_target: None,
        }
    }
}

thread_local! {
    static STATE: RefCell<EditCamState> = RefCell::new(EditCamState::default());
}

pub fn is_free_look() -> bool {
    STATE.with(|s| s.borrow().free_look)
}

pub fn ui_and_step(ed: &mut Editor, ui: &mut egui::Ui, rect: Rect, resp: &egui::Response, dt: f32) {
    if ed.game.is_playing() || ed.clean_view || rect.width() < 240.0 || rect.height() < 180.0 {
        return;
    }

    let ctx = ui.ctx().clone();
    let prefs = crate::studio_settings::camera_prefs();

    // 1. Virtual Fly Joystick (bottom-left) + Altitude / Focus / Mode (bottom-right)
    let joy_center = Pos2::new(rect.min.x + 64.0, rect.max.y - 64.0);
    let joy_radius = 42.0;
    let joy_hit_rect = Rect::from_center_size(joy_center, Vec2::splat(joy_radius * 2.3));
    let joy_resp = ui.interact(joy_hit_rect, egui::Id::new("edit_cam_fly_joy"), Sense::click_and_drag());

    let mut joy_vec = Vec2::ZERO;
    let mut joy_touching = false;
    if joy_resp.dragged() || joy_resp.is_pointer_button_down_on() {
        if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
            let delta = p - joy_center;
            let len = delta.length();
            let clamped = if len > joy_radius {
                delta * (joy_radius / len)
            } else {
                delta
            };
            joy_vec = clamped / joy_radius;
            joy_touching = true;
        }
    }

    let painter = ui.painter_at(rect);
    painter.circle_filled(joy_center, joy_radius, Color32::from_rgba_unmultiplied(20, 22, 28, 120));
    painter.circle_stroke(
        joy_center,
        joy_radius,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(220, 225, 235, 140)),
    );
    let knob = joy_center + joy_vec * joy_radius;
    painter.circle_filled(
        knob,
        17.0,
        Color32::from_rgba_unmultiplied(120, 185, 255, if joy_touching { 215 } else { 155 }),
    );

    // Bottom-right circular controls (Mobile Studio v2.0 style: ^, v, Speed x1/x2/x4, Focus, Mode)
    let mut focus_clicked = false;
    let (mut mode, mut speed_mul, mut free_look, smooth_target) = STATE.with(|s| {
        let b = s.borrow();
        (b.mode, b.speed_mul, b.free_look, b.smooth_target)
    });

    let circle_btn = |ui: &mut egui::Ui, id: &str, center: Pos2, radius: f32, active: bool| -> egui::Response {
        let r = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
        let resp = ui.interact(r, egui::Id::new(id), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        let fill = if active {
            Color32::from_rgba_unmultiplied(46, 125, 225, 210)
        } else if resp.is_pointer_button_down_on() {
            Color32::from_rgba_unmultiplied(55, 62, 74, 215)
        } else {
            Color32::from_rgba_unmultiplied(24, 27, 34, 150)
        };
        let stroke = Color32::from_rgba_unmultiplied(230, 235, 245, if active { 230 } else { 170 });
        p.circle_filled(center, radius, fill);
        p.circle_stroke(center, radius, Stroke::new(1.5, stroke));
        resp
    };

    let up_c = Pos2::new(rect.max.x - 36.0, rect.max.y - 96.0);
    let dn_c = Pos2::new(rect.max.x - 36.0, rect.max.y - 38.0);
    let spd_c = Pos2::new(rect.max.x - 94.0, rect.max.y - 38.0);
    let foc_c = Pos2::new(rect.max.x - 94.0, rect.max.y - 96.0);
    let mode_c = Pos2::new(rect.max.x - 146.0, rect.max.y - 96.0);

    let up_r = circle_btn(ui, "edit_cam_up_c", up_c, 24.0, false);
    let up_held = up_r.is_pointer_button_down_on();
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(up_c, Vec2::splat(22.0)), "chevron-up", Color32::WHITE);

    let dn_r = circle_btn(ui, "edit_cam_dn_c", dn_c, 24.0, false);
    let down_held = dn_r.is_pointer_button_down_on();
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(dn_c, Vec2::splat(22.0)), "chevron-down", Color32::WHITE);

    let spd_r = circle_btn(ui, "edit_cam_spd_c", spd_c, 20.0, false).on_hover_text("Camera speed (x1 / x2 / x4)");
    if spd_r.clicked() {
        speed_mul = if speed_mul < 1.5 { 2.0 } else if speed_mul < 3.0 { 4.0 } else { 1.0 };
    }
    let spd_txt = if speed_mul >= 3.5 { "x4" } else if speed_mul >= 1.5 { "x2" } else { "x1" };
    painter.text(spd_c, egui::Align2::CENTER_CENTER, spd_txt, egui::FontId::proportional(13.0), Color32::WHITE);

    let foc_r = circle_btn(ui, "edit_cam_foc_c", foc_c, 20.0, false).on_hover_text("Focus selected (F)");
    if foc_r.clicked() {
        focus_clicked = true;
    }
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(foc_c, Vec2::splat(18.0)), "focus-centered", Color32::WHITE);

    let mode_lbl = match mode {
        EditCamMode::Fly => "Fly",
        EditCamMode::Orbit => "Orb",
    };
    let mode_r = circle_btn(ui, "edit_cam_mode_c", mode_c, 20.0, mode == EditCamMode::Orbit).on_hover_text("Fly / Orbit camera mode");
    if mode_r.clicked() {
        mode = match mode {
            EditCamMode::Fly => EditCamMode::Orbit,
            EditCamMode::Orbit => EditCamMode::Fly,
        };
    }
    painter.text(mode_c, egui::Align2::CENTER_CENTER, mode_lbl, egui::FontId::proportional(11.5), Color32::WHITE);
    let _ = &mut free_look;

    // 2. Apply Virtual Joystick + ▲/▼ Fly Movement
    let elev = if up_held { 1.0 } else { 0.0 } - if down_held { 1.0 } else { 0.0 };
    let fly_dir = Vec3::new(joy_vec.x, elev, -joy_vec.y);
    if fly_dir.length_squared() > 0.001 {
        ed.game.cam.fly(fly_dir, dt.max(0.001), false, speed_mul, &prefs);
        ctx.request_repaint();
    }

    // 3. 🎯 Focus (F): compute center of Selection and smoothly fly + set Orbit pivot
    let mut new_smooth = smooth_target;
    if focus_clicked {
        if let Some((center, radius)) = selection_bounds(ed) {
            let dist = (radius * 2.6).clamp(6.0, 60.0);
            let fwd = ed.game.cam.forward();
            let target_pos = center - fwd * dist;
            new_smooth = Some((target_pos, center));
            ed.game.cam.focus = center;
        }
    }
    if let Some((t_pos, t_piv)) = new_smooth {
        let alpha = (dt * 10.0).clamp(0.05, 0.45);
        ed.game.cam.pos = ed.game.cam.pos.lerp(t_pos, alpha);
        ed.game.cam.focus = ed.game.cam.focus.lerp(t_piv, alpha);
        if ed.game.cam.pos.distance(t_pos) < 0.05 {
            ed.game.cam.pos = t_pos;
            new_smooth = None;
        } else {
            ctx.request_repaint();
        }
    }

    // 4. Camera rotation: Right-drag on PC; 1-finger swipe on Mobile (when not dragging part/gizmo)
    let adorn_hot = rustudio_engine::gui::share().adorn_hot;
    let delta = ctx.input(|i| i.pointer.delta());
    let is_mobile = cfg!(any(target_os = "android", target_os = "ios")) || rustudio_core::mobile::is_drawer_mode();
    let can_free_drag = free_look
        || (ed.drag.is_none() && ed.marquee.is_none() && ed.part_drag.is_none() && !adorn_hot);
    let want_rotate = if is_mobile {
        resp.dragged_by(egui::PointerButton::Primary) || resp.dragged_by(egui::PointerButton::Secondary)
    } else {
        resp.dragged_by(egui::PointerButton::Secondary)
    };
    if want_rotate
        && !joy_touching
        && !up_held
        && !down_held
        && can_free_drag
        && delta != Vec2::ZERO
    {
        match mode {
            EditCamMode::Fly => ed.game.cam.rotate(delta.x, delta.y),
            EditCamMode::Orbit => ed.game.cam.orbit(delta.x, delta.y),
        }
        ctx.request_repaint();
    }

    STATE.with(|s| {
        let mut b = s.borrow_mut();
        b.mode = mode;
        b.speed_mul = speed_mul;
        b.free_look = free_look;
        b.joy_vec = joy_vec;
        b.joy_touching = joy_touching;
        b.up_held = up_held;
        b.down_held = down_held;
        b.smooth_target = new_smooth;
    });
}

fn selection_bounds(ed: &Editor) -> Option<(Vec3, f32)> {
    let mut sum = Vec3::ZERO;
    let mut count = 0usize;
    let mut max_r = 2.0f32;
    for &r in &ed.game.doc.selection {
        if let Some(sp) = ed.game.scene.get(r) {
            sum += sp.draw.xf.pos;
            max_r = max_r.max(sp.draw.size.length() * 0.5);
            count += 1;
        }
    }
    if count > 0 {
        Some((sum / count as f32, max_r))
    } else {
        None
    }
}

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
    pub smooth_joy: Vec2,
    pub joy_origin: Option<Pos2>,
    pub joy_touching: bool,
    pub joy_touch_id: Option<u64>,
    pub look_touch_id: Option<u64>,
    pub last_look_pos: Pos2,
    pub pinch_touch_id: Option<u64>,
    pub last_pinch_pos: Pos2,
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
            smooth_joy: Vec2::ZERO,
            joy_origin: None,
            joy_touching: false,
            joy_touch_id: None,
            look_touch_id: None,
            last_look_pos: Pos2::ZERO,
            pinch_touch_id: None,
            last_pinch_pos: Pos2::ZERO,
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
    // Scale proportionally to screen height so controls fit comfortably on both iPhone landscape (375pt tall) and iPad (768pt+ tall).
    let (_safe_top, safe_bottom, safe_left, _safe_right) = crate::platform::get_safe_area_insets();
    let dock_lift = ctx.data(|d| d.get_temp::<f32>(egui::Id::new("pill_dock_cam_lift")).unwrap_or(0.0));
    let h_scale = (rect.height() / 420.0).clamp(0.76, 1.05);
    let joy_radius = (44.0 * h_scale).round();
    let default_joy_center = Pos2::new(
        rect.min.x + safe_left + 18.0 + joy_radius,
        rect.max.y - safe_bottom - 16.0 - joy_radius - dock_lift * 0.5,
    );
    let joy_zone = Rect::from_min_max(
        Pos2::new(rect.min.x, (rect.max.y - rect.height() * 0.55).max(rect.min.y)),
        Pos2::new((rect.min.x + rect.width() * 0.34).min(default_joy_center.x + joy_radius * 2.0), rect.max.y),
    );
    let joy_hit_rect = Rect::from_center_size(default_joy_center, Vec2::splat(joy_radius * 2.3));
    let joy_resp = ui.interact(joy_hit_rect, egui::Id::new("edit_cam_fly_joy"), Sense::click_and_drag());

    let (
        mut joy_touch_id,
        mut joy_origin,
        mut look_touch_id,
        mut last_look_pos,
        mut pinch_touch_id,
        mut last_pinch_pos,
        prev_joy_vec,
        mut smooth_joy,
    ) = STATE.with(|s| {
        let b = s.borrow();
        (
            b.joy_touch_id,
            b.joy_origin,
            b.look_touch_id,
            b.last_look_pos,
            b.pinch_touch_id,
            b.last_pinch_pos,
            b.joy_vec,
            b.smooth_joy,
        )
    });
    let events = ctx.input(|i| i.events.clone());
    let any_active = ctx.input(|i| i.any_touches() || i.pointer.any_down())
        || events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Touch {
                    phase: egui::TouchPhase::Start | egui::TouchPhase::Move,
                    ..
                }
            )
        });
    if !any_active {
        joy_touch_id = None;
        joy_origin = None;
        look_touch_id = None;
        pinch_touch_id = None;
    }

    let mut joy_vec = if joy_touch_id.is_some() { prev_joy_vec } else { Vec2::ZERO };
    let mut touch_look_delta = Vec2::ZERO;
    let mut pinch_zoom_delta = 0.0_f32;
    let mut pinch_pan_delta = Vec2::ZERO;
    let right_ctrl_zone = Rect::from_min_max(
        Pos2::new(rect.max.x - 175.0 * h_scale, rect.max.y - safe_bottom - dock_lift - 140.0 * h_scale),
        Pos2::new(rect.max.x, rect.max.y),
    );
    let explorer_rect: Option<Rect> = ctx.data(|d| {
        d.get_temp::<Option<Rect>>(egui::Id::new("portrait_explorer_window_rect"))
            .flatten()
    });
    for ev in &events {
        if let egui::Event::Touch { id, phase, pos, .. } = ev {
            let tid = id.0;
            match phase {
                egui::TouchPhase::Start => {
                    let over_overlay = explorer_rect.is_some_and(|er| er.contains(*pos))
                        || ctx
                            .layer_id_at(*pos)
                            .is_some_and(|lid| lid.order >= egui::Order::Middle);
                    if !rect.contains(*pos) || right_ctrl_zone.contains(*pos) || over_overlay {
                        continue;
                    }
                    if joy_touch_id.is_none() && (joy_hit_rect.contains(*pos) || joy_zone.contains(*pos)) {
                        joy_touch_id = Some(tid);
                        let clamped_origin = Pos2::new(
                            pos.x.clamp(rect.min.x + safe_left + joy_radius + 8.0, joy_zone.max.x),
                            pos.y.clamp(joy_zone.min.y + joy_radius, rect.max.y - safe_bottom - joy_radius - 8.0),
                        );
                        joy_origin = Some(clamped_origin);
                        joy_vec = Vec2::ZERO;
                    } else if look_touch_id.is_none() {
                        look_touch_id = Some(tid);
                        last_look_pos = *pos;
                    } else if pinch_touch_id.is_none() && look_touch_id != Some(tid) {
                        pinch_touch_id = Some(tid);
                        last_pinch_pos = *pos;
                    }
                }
                egui::TouchPhase::Move => {
                    if joy_touch_id == Some(tid) {
                        let origin = joy_origin.unwrap_or(default_joy_center);
                        let d = *pos - origin;
                        let len = d.length();
                        let clamped = if len > joy_radius { d * (joy_radius / len) } else { d };
                        joy_vec = clamped / joy_radius;
                    } else if look_touch_id == Some(tid) {
                        if pinch_touch_id.is_some() {
                            let prev_dist = (last_look_pos - last_pinch_pos).length();
                            let prev_mid = last_look_pos.lerp(last_pinch_pos, 0.5);
                            last_look_pos = *pos;
                            let new_dist = (last_look_pos - last_pinch_pos).length();
                            let new_mid = last_look_pos.lerp(last_pinch_pos, 0.5);
                            pinch_zoom_delta += (new_dist - prev_dist) * 0.06;
                            pinch_pan_delta += new_mid - prev_mid;
                        } else {
                            let d = *pos - last_look_pos;
                            last_look_pos = *pos;
                            if d.length() <= 120.0 {
                                touch_look_delta += d;
                            }
                        }
                    } else if pinch_touch_id == Some(tid) {
                        if look_touch_id.is_some() {
                            let prev_dist = (last_look_pos - last_pinch_pos).length();
                            let prev_mid = last_look_pos.lerp(last_pinch_pos, 0.5);
                            last_pinch_pos = *pos;
                            let new_dist = (last_look_pos - last_pinch_pos).length();
                            let new_mid = last_look_pos.lerp(last_pinch_pos, 0.5);
                            pinch_zoom_delta += (new_dist - prev_dist) * 0.06;
                            pinch_pan_delta += new_mid - prev_mid;
                        } else {
                            last_pinch_pos = *pos;
                        }
                    }
                }
                egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                    if joy_touch_id == Some(tid) {
                        joy_touch_id = None;
                        joy_origin = None;
                        joy_vec = Vec2::ZERO;
                    }
                    if look_touch_id == Some(tid) {
                        look_touch_id = pinch_touch_id.take();
                        last_look_pos = last_pinch_pos;
                    } else if pinch_touch_id == Some(tid) {
                        pinch_touch_id = None;
                    }
                }
            }
        }
    }

    let joy_center = joy_origin.unwrap_or(default_joy_center);
    let mut joy_touching = joy_touch_id.is_some();
    if !joy_touching && look_touch_id.is_none() && (joy_resp.dragged() || joy_resp.is_pointer_button_down_on()) {
        if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
            let delta = p - default_joy_center;
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
    painter.circle_filled(joy_center, joy_radius, Color32::from_rgba_unmultiplied(18, 20, 26, if joy_touching { 145 } else { 110 }));
    painter.circle_stroke(
        joy_center,
        joy_radius,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(235, 240, 250, if joy_touching { 205 } else { 145 })),
    );
    let knob = joy_center + joy_vec * joy_radius;
    painter.circle_filled(
        knob,
        17.0,
        if joy_touching {
            Color32::from_rgba_unmultiplied(255, 90, 38, 220)
        } else {
            Color32::from_rgba_unmultiplied(220, 225, 235, 165)
        },
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
            Color32::from_rgba_unmultiplied(255, 90, 38, 215)
        } else if resp.is_pointer_button_down_on() {
            Color32::from_rgba_unmultiplied(55, 62, 74, 215)
        } else {
            Color32::from_rgba_unmultiplied(18, 20, 26, 140)
        };
        let stroke = Color32::from_rgba_unmultiplied(240, 244, 250, if active { 235 } else { 185 });
        p.circle_filled(center, radius, fill);
        p.circle_stroke(center, radius, Stroke::new(1.6, stroke));
        resp
    };

    let r_big = (24.0 * h_scale).round();
    let r_sm = (20.0 * h_scale).round();
    let step_x = (56.0 * h_scale).round();
    let step_y = (56.0 * h_scale).round();
    let base_x = rect.max.x - 14.0 - r_big;
    let base_y = rect.max.y - safe_bottom - 14.0 - r_big - dock_lift;

    let dn_c = Pos2::new(base_x, base_y);
    let up_c = Pos2::new(base_x, base_y - step_y);
    let spd_c = Pos2::new(base_x - step_x, base_y);
    let foc_c = Pos2::new(base_x - step_x, base_y - step_y);
    let mode_c = Pos2::new(base_x - step_x * 1.9, base_y - step_y);

    let up_r = circle_btn(ui, "edit_cam_up_c", up_c, r_big, false);
    let up_held = up_r.is_pointer_button_down_on();
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(up_c, Vec2::splat(20.0 * h_scale)), "chevron-up", Color32::WHITE);

    let dn_r = circle_btn(ui, "edit_cam_dn_c", dn_c, r_big, false);
    let down_held = dn_r.is_pointer_button_down_on();
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(dn_c, Vec2::splat(20.0 * h_scale)), "chevron-down", Color32::WHITE);

    let spd_r = circle_btn(ui, "edit_cam_spd_c", spd_c, r_sm, false).on_hover_text("Camera speed (x1 / x2 / x4)");
    if spd_r.clicked() {
        speed_mul = if speed_mul < 1.5 { 2.0 } else if speed_mul < 3.0 { 4.0 } else { 1.0 };
    }
    let spd_txt = if speed_mul >= 3.5 { "x4" } else if speed_mul >= 1.5 { "x2" } else { "x1" };
    painter.text(spd_c, egui::Align2::CENTER_CENTER, spd_txt, egui::FontId::proportional(12.5 * h_scale), Color32::WHITE);

    let foc_r = circle_btn(ui, "edit_cam_foc_c", foc_c, r_sm, false).on_hover_text("Focus selected (F)");
    if foc_r.clicked() {
        focus_clicked = true;
    }
    rustudio_engine::ti::paint(&painter, Rect::from_center_size(foc_c, Vec2::splat(17.0 * h_scale)), "focus-centered", Color32::WHITE);

    let mode_lbl = match mode {
        EditCamMode::Fly => "Fly",
        EditCamMode::Orbit => "Orb",
    };
    let mode_r = circle_btn(ui, "edit_cam_mode_c", mode_c, r_sm, mode == EditCamMode::Orbit).on_hover_text("Fly / Orbit camera mode");
    if mode_r.clicked() {
        mode = match mode {
            EditCamMode::Fly => EditCamMode::Orbit,
            EditCamMode::Orbit => EditCamMode::Fly,
        };
    }
    painter.text(mode_c, egui::Align2::CENTER_CENTER, mode_lbl, egui::FontId::proportional(11.5), Color32::WHITE);
    let _ = &mut free_look;

    // 2. Apply Smoothed Virtual Joystick + ▲/▼ Fly Movement & 2-Finger Pinch Zoom/Pan
    let smooth_k = (dt * 18.0).clamp(0.18, 0.75);
    smooth_joy = smooth_joy + (joy_vec - smooth_joy) * smooth_k;
    if !joy_touching && smooth_joy.length_sq() < 0.0004 {
        smooth_joy = Vec2::ZERO;
    }
    let elev = if up_held { 1.0 } else { 0.0 } - if down_held { 1.0 } else { 0.0 };
    let fly_dir = Vec3::new(smooth_joy.x, elev, -smooth_joy.y);
    if fly_dir.length_squared() > 0.0005 {
        ed.game.cam.fly(fly_dir, dt.max(0.001), false, speed_mul, &prefs);
        ctx.request_repaint();
    }
    if pinch_zoom_delta.abs() > 0.001 {
        let fwd = ed.game.cam.forward();
        ed.game.cam.pos += fwd * (pinch_zoom_delta * 2.5 * speed_mul);
        ctx.request_repaint();
    }
    if pinch_pan_delta.length_sq() > 0.01 && pinch_pan_delta.length() <= 120.0 {
        ed.game.cam.pan(pinch_pan_delta.x, pinch_pan_delta.y, &prefs);
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

    // 4. Camera rotation: Right-drag on PC; independent TouchId or 1-finger swipe on Mobile
    let adorn_hot = rustudio_engine::gui::share().adorn_hot;
    let is_mobile = cfg!(any(target_os = "android", target_os = "ios")) || rustudio_core::mobile::is_drawer_mode();
    let can_free_drag = free_look
        || (ed.drag.is_none() && ed.marquee.is_none() && ed.part_drag.is_none() && !adorn_hot);
    let (want_rotate, delta) = if look_touch_id.is_some() {
        (true, touch_look_delta)
    } else if is_mobile {
        (
            !joy_touching && (resp.dragged_by(egui::PointerButton::Primary) || resp.dragged_by(egui::PointerButton::Secondary)),
            ctx.input(|i| i.pointer.delta()),
        )
    } else {
        (resp.dragged_by(egui::PointerButton::Secondary), ctx.input(|i| i.pointer.delta()))
    };
    if want_rotate
        && !up_held
        && !down_held
        && can_free_drag
        && delta != Vec2::ZERO
        && delta.length() <= 120.0
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
        b.smooth_joy = smooth_joy;
        b.joy_origin = joy_origin;
        b.joy_touching = joy_touching;
        b.joy_touch_id = joy_touch_id;
        b.look_touch_id = look_touch_id;
        b.last_look_pos = last_look_pos;
        b.pinch_touch_id = pinch_touch_id;
        b.last_pinch_pos = last_pinch_pos;
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

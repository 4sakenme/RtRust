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

    // Bottom-right controls (▲, ▼, 🎯 Focus, Mode, Look toggle, Speed)
    let mut focus_clicked = false;
    let mut up_held = false;
    let mut down_held = false;
    let (mut mode, mut speed_mul, mut free_look, smooth_target) = STATE.with(|s| {
        let b = s.borrow();
        (b.mode, b.speed_mul, b.free_look, b.smooth_target)
    });

    egui::Area::new(egui::Id::new("edit_cam_controls_area"))
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new((rect.max.x - 255.0).max(rect.min.x + 8.0), (rect.max.y - 76.0).max(rect.min.y + 8.0)))
        .show(&ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(22, 24, 30, 195))
                .stroke(Stroke::new(1.0, Color32::from_rgb(65, 70, 82)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(6, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        let up_r = ui.add(egui::Button::new("▲").min_size(Vec2::new(34.0, 28.0)));
                        up_held = up_r.is_pointer_button_down_on();
                        let dn_r = ui.add(egui::Button::new("▼").min_size(Vec2::new(34.0, 28.0)));
                        down_held = dn_r.is_pointer_button_down_on();
                        if ui
                            .add(egui::Button::new("🎯 Focus").min_size(Vec2::new(66.0, 28.0)))
                            .on_hover_text("Bay tới đối tượng đang chọn (F)")
                            .clicked()
                        {
                            focus_clicked = true;
                        }
                    });
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        let mode_lbl = match mode {
                            EditCamMode::Fly => "🕊 Fly",
                            EditCamMode::Orbit => "🔄 Orbit",
                        };
                        if ui.small_button(mode_lbl).clicked() {
                            mode = match mode {
                                EditCamMode::Fly => EditCamMode::Orbit,
                                EditCamMode::Orbit => EditCamMode::Fly,
                            };
                        }
                        let look_btn = egui::Button::new(if free_look { "👁 ON" } else { "👁 Cam" })
                            .fill(if free_look { Color32::from_rgb(55, 120, 200) } else { Color32::TRANSPARENT });
                        if ui.add(look_btn).on_hover_text("Kéo bất kỳ đâu trên màn hình để xoay cam").clicked() {
                            free_look = !free_look;
                        }
                        ui.add(
                            egui::Slider::new(&mut speed_mul, 0.2..=5.0)
                                .show_value(false)
                                .custom_formatter(|v, _| format!("{v:.1}x")),
                        );
                        ui.label(egui::RichText::new(format!("{speed_mul:.1}x")).small());
                    });
                });
        });

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

    // 4. Direct 1-finger touch look/orbit anywhere (free-drag without circular bounds)
    let adorn_hot = rustudio_engine::gui::share().adorn_hot;
    let delta = ctx.input(|i| i.pointer.delta());
    let can_free_drag = free_look || (ed.drag.is_none() && ed.marquee.is_none() && !adorn_hot && ed.hover.is_none());
    if resp.dragged_by(egui::PointerButton::Primary)
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

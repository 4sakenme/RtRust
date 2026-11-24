//! Stitch-inspired RtRust Studio Loading Screen (clean solid Obsidian background,
//! animated Rust-Orange orbit + Electric Cyan cube centerpiece, and frosted-glass status pill).

use egui::{Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use std::f32::consts::TAU;

const BG_OBSIDIAN: Color32 = Color32::from_rgb(11, 13, 18); // #0B0D12
const RUST_ORANGE: Color32 = Color32::from_rgb(255, 90, 38); // #FF5A26
const CYAN_ACCENT: Color32 = Color32::from_rgb(38, 198, 218); // #26C6DA
const TEXT_PRIMARY: Color32 = Color32::from_rgb(238, 241, 248);
const TEXT_DIM: Color32 = Color32::from_rgb(142, 150, 170);

pub struct LoadingScreenState {
    boot_started_at: Option<f64>,
    boot_done: bool,
    fade_alpha: f32,
}

impl Default for LoadingScreenState {
    fn default() -> Self {
        Self {
            boot_started_at: None,
            boot_done: false,
            fade_alpha: 1.0,
        }
    }
}

impl LoadingScreenState {
    /// Draws the loading screen overlay during initial boot or while a place is loading.
    /// Returns `true` if the loading screen is still visible (including fade-out).
    pub fn show(&mut self, ctx: &egui::Context, place_loading: bool, pending_assets: usize) -> bool {
        let now = ctx.input(|i| i.time);
        let start = *self.boot_started_at.get_or_insert(now);
        let boot_elapsed = (now - start).max(0.0) as f32;

        // Initial boot splash runs for ~1.1s unless a place is still loading.
        if !self.boot_done && boot_elapsed >= 1.15 {
            self.boot_done = true;
        }

        let active = !self.boot_done || place_loading;
        let target_alpha = if active { 1.0 } else { 0.0 };
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        let speed = if active { 14.0 } else { 5.5 };
        self.fade_alpha += (target_alpha - self.fade_alpha) * (1.0 - (-speed * dt).exp());

        if !active && self.fade_alpha < 0.01 {
            self.fade_alpha = 0.0;
            return false;
        }

        ctx.request_repaint();

        let alpha = self.fade_alpha.clamp(0.0, 1.0);
        let a = |c: Color32| -> Color32 { c.gamma_multiply(alpha) };
        let screen = ctx.content_rect();
        let (safe_top, safe_bottom, safe_left, safe_right) = crate::platform::get_safe_area_insets();

        egui::Area::new(egui::Id::new("rtrust_loading_screen"))
            .order(egui::Order::Tooltip)
            .fixed_pos(screen.min)
            .interactable(active)
            .show(ctx, |ui| {
                ui.set_min_size(screen.size());
                let painter = ui.painter();

                // 1. Clean solid Obsidian black background (no wallpaper/grid)
                painter.rect_filled(screen, CornerRadius::ZERO, a(BG_OBSIDIAN));

                let center = screen.center();

                // 2. Subtle warm radial ambient glow behind centerpiece
                for (r, mul) in [(180.0, 0.045), (120.0, 0.07), (72.0, 0.11)] {
                    painter.circle_filled(
                        center - Vec2::new(0.0, 18.0),
                        r,
                        RUST_ORANGE.gamma_multiply(mul * alpha),
                    );
                }

                // 3. Top corner metadata (safe-area padded)
                let top_y = screen.top() + 14.0 + safe_top;
                let left_x = screen.left() + 18.0 + safe_left;
                let right_x = screen.right() - 18.0 - safe_right;

                painter.text(
                    Pos2::new(left_x, top_y),
                    egui::Align2::LEFT_TOP,
                    concat!("RtRust Engine v", env!("CARGO_PKG_VERSION")),
                    FontId::monospace(12.0),
                    a(TEXT_DIM),
                );

                let backend_label = if cfg!(any(target_os = "ios", target_os = "macos")) {
                    "Metal · 64-bit"
                } else {
                    "Vulkan · 64-bit"
                };
                let badge_rect = Rect::from_min_size(
                    Pos2::new(right_x - 110.0, top_y - 3.0),
                    Vec2::new(110.0, 22.0),
                );
                painter.rect(
                    badge_rect,
                    CornerRadius::same(11),
                    a(Color32::from_rgba_unmultiplied(25, 28, 36, 200)),
                    Stroke::new(1.0, a(Color32::from_rgba_unmultiplied(255, 255, 255, 28))),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    backend_label,
                    FontId::monospace(11.0),
                    a(TEXT_DIM),
                );

                // 4. Animated Hero Centerpiece: Official RtRust.svg App Icon (Rotating Gear + Tilted Studio Square + 3D Gizmo)
                let logo_center = center - Vec2::new(0.0, 26.0);
                let t = now as f32;
                let icon_size = 112.0;
                let s = icon_size / 512.0;
                let icon_origin = logo_center - Vec2::splat(icon_size * 0.5);
                let pt = |x: f32, y: f32| -> Pos2 { icon_origin + Vec2::new(x * s, y * s) };

                // Outer rounded dark obsidian app icon card
                let card_rect = Rect::from_min_size(pt(24.0, 24.0), Vec2::splat(464.0 * s));
                painter.rect(
                    card_rect,
                    CornerRadius::same((108.0 * s).round() as u8),
                    a(Color32::from_rgb(22, 25, 33)),
                    Stroke::new(1.8, a(Color32::from_rgb(42, 47, 61))),
                    egui::StrokeKind::Inside,
                );

                // Rust gear at bottom-right (295, 295) rotating continuously
                let gear_c = pt(295.0, 295.0);
                let gear_r = 110.0 * s;
                let gear_rot = t * 1.1;
                for seg in 0..10 {
                    let a0 = gear_rot + (seg as f32) * (TAU / 10.0);
                    let a1 = a0 + (TAU / 10.0) * 0.62;
                    let steps = 6;
                    let mut pts = Vec::with_capacity(steps + 1);
                    for i in 0..=steps {
                        let ang = a0 + (a1 - a0) * (i as f32 / steps as f32);
                        pts.push(gear_c + Vec2::new(ang.cos(), ang.sin()) * gear_r);
                    }
                    painter.add(egui::Shape::line(pts, Stroke::new(26.0 * s, a(Color32::from_rgb(255, 107, 43)))));
                }
                painter.circle(
                    gear_c,
                    78.0 * s,
                    a(Color32::from_rgb(19, 22, 29)),
                    Stroke::new(8.0 * s, a(Color32::from_rgb(200, 50, 0))),
                );

                // Tilted Studio square at (215, 215) with gentle breathing pulse
                let sq_c = pt(215.0, 215.0);
                let pulse = 1.0 + 0.035 * (t * 2.4).sin();
                let tilt = -14.0_f32.to_radians() + 0.03 * (t * 1.6).cos();
                let (ts, tc) = tilt.sin_cos();
                let rot_poly = |half: f32| -> Vec<Pos2> {
                    [
                        Vec2::new(-half, -half),
                        Vec2::new(half, -half),
                        Vec2::new(half, half),
                        Vec2::new(-half, half),
                    ]
                    .into_iter()
                    .map(|v| sq_c + Vec2::new(v.x * tc - v.y * ts, v.x * ts + v.y * tc))
                    .collect()
                };
                painter.add(egui::Shape::convex_polygon(
                    rot_poly(115.0 * s * pulse),
                    a(Color32::from_rgb(19, 22, 29)),
                    Stroke::new(30.0 * s, a(Color32::from_rgb(0, 210, 255))),
                ));
                painter.add(egui::Shape::convex_polygon(
                    rot_poly(32.0 * s * pulse),
                    a(Color32::WHITE),
                    Stroke::NONE,
                ));

                // 3D Coordinate Gizmo at (295, 295)
                painter.line_segment([gear_c, gear_c + Vec2::new(0.0, -52.0 * s)], Stroke::new(10.0 * s, a(Color32::from_rgb(34, 197, 94))));
                painter.line_segment([gear_c, gear_c + Vec2::new(45.0 * s, 26.0 * s)], Stroke::new(10.0 * s, a(Color32::from_rgb(239, 68, 68))));
                painter.line_segment([gear_c, gear_c + Vec2::new(-45.0 * s, 26.0 * s)], Stroke::new(10.0 * s, a(Color32::from_rgb(59, 130, 246))));
                painter.circle_filled(gear_c, 9.0 * s, a(Color32::WHITE));

                // Title & subtitle
                painter.text(
                    logo_center + Vec2::new(0.0, 74.0),
                    egui::Align2::CENTER_CENTER,
                    "RtRust Studio",
                    FontId::proportional(24.0),
                    a(TEXT_PRIMARY),
                );
                let subtitle = if place_loading {
                    "Streaming Place Geometry & Assets..."
                } else {
                    "Initializing 3D Viewport & Luau Runtime"
                };
                painter.text(
                    logo_center + Vec2::new(0.0, 99.0),
                    egui::Align2::CENTER_CENTER,
                    subtitle,
                    FontId::proportional(13.0),
                    a(TEXT_DIM),
                );

                // 5. Minimalist Floating Frosted-Glass Status Pill at bottom
                let (stage_text, progress) = if place_loading {
                    let pct = if pending_assets > 0 {
                        (1.0 / (1.0 + (pending_assets as f32 * 0.08))).clamp(0.22, 0.94)
                    } else {
                        0.85
                    };
                    ("Loading Core Assets & Shaders...", pct)
                } else if boot_elapsed < 0.35 {
                    ("Compiling Metal & WGPU Shaders...", 0.28 + (boot_elapsed / 0.35) * 0.22)
                } else if boot_elapsed < 0.72 {
                    ("Mounting 931 Reflection Classes...", 0.50 + ((boot_elapsed - 0.35) / 0.37) * 0.24)
                } else {
                    ("Starting Luau VM & 3D Viewport...", (0.74 + ((boot_elapsed - 0.72) / 0.43) * 0.26).min(1.0))
                };

                let pill_w = (screen.width() - safe_left - safe_right - 40.0).clamp(240.0, 420.0);
                let pill_h = 48.0;
                let pill_rect = Rect::from_center_size(
                    Pos2::new(center.x, screen.bottom() - safe_bottom - 42.0),
                    Vec2::new(pill_w, pill_h),
                );
                painter.rect(
                    pill_rect,
                    CornerRadius::same(14),
                    a(Color32::from_rgba_unmultiplied(22, 25, 34, 232)),
                    Stroke::new(1.0, a(Color32::from_rgba_unmultiplied(255, 255, 255, 30))),
                    egui::StrokeKind::Inside,
                );

                painter.text(
                    Pos2::new(pill_rect.left() + 14.0, pill_rect.top() + 16.0),
                    egui::Align2::LEFT_CENTER,
                    stage_text,
                    FontId::proportional(12.5),
                    a(TEXT_PRIMARY),
                );
                painter.text(
                    Pos2::new(pill_rect.right() - 14.0, pill_rect.top() + 16.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}%", (progress * 100.0).round() as u32),
                    FontId::monospace(12.0),
                    a(CYAN_ACCENT),
                );

                // Progress track & animated fill bar inside pill
                let bar_rect = Rect::from_min_max(
                    Pos2::new(pill_rect.left() + 14.0, pill_rect.bottom() - 12.0),
                    Pos2::new(pill_rect.right() - 14.0, pill_rect.bottom() - 7.0),
                );
                painter.rect_filled(
                    bar_rect,
                    CornerRadius::same(3),
                    a(Color32::from_rgb(34, 38, 50)),
                );
                let fill_w = (bar_rect.width() * progress).max(6.0);
                let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
                painter.rect_filled(fill_rect, CornerRadius::same(3), a(RUST_ORANGE));

                let _ = (TAU, RichText::new(""));
            });

        true
    }
}

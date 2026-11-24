//! Studio's top row (run type, Play / Pause / Stop, the ribbon tabs, the
//! account) and the ribbon tabs' buttons, as in an offline Studio
//! (checks/ui/studio/ribbon-*.png).

use egui::{Color32, Rect, Sense, Vec2};

use crate::app::{Editor, Tool};
use crate::studio_ui::{run_type_key, tab_rect_key, RibbonTab, RunType};
use rustudio_engine::theme;

/// The dragger settings drop-down's popup id (the `dragger_menu` action opens it).
pub(crate) const DRAGGER_MENU_ID: &str = "dragger_menu";

fn divider(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 46.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, theme::DIVIDER);
    ui.add_space(4.0);
}

/// Studio's ribbon button: a 24-px icon over an 11-px label.
pub fn ribbon_button(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, enabled: bool) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.into(), font.clone(), theme::TEXT).size().x);
    let w = (tw + 10.0).max(48.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 50.0), sense);
    let p = ui.painter();
    let icon_r = Rect::from_center_size(r.center() - Vec2::new(0.0, 8.0), Vec2::splat(24.0));
    if active {
        p.rect_filled(icon_r.expand(5.0), 4.0, theme::ACTIVE_TAB.gamma_multiply(1.5));
    } else if enabled && resp.hovered() {
        p.rect_filled(r, 4.0, theme::HOVER_BG);
    }
    let col = if !enabled {
        theme::TEXT_DIM.gamma_multiply(0.8)
    } else if active {
        Color32::WHITE
    } else {
        theme::TEXT
    };
    let icon_color = ribbon_icon_color(icon);
    let icon_color = if enabled { icon_color } else { icon_color.gamma_multiply(0.55) };
    rustudio_engine::ti::paint(p, icon_r, icon, icon_color);
    p.text(egui::pos2(r.center().x, r.max.y - 7.0), egui::Align2::CENTER_CENTER, label, font, col);
    resp
}

/// Palette audited against ../checks/ui/studio/ribbon-{Home,Model,Avatar,UI,Script,Plugins}.png.
/// Neutral tools retain Studio's cool white; disabled icons retain their hue at reduced brightness.
fn ribbon_icon_color(icon: &str) -> Color32 {
    let rgb = match icon {
        "cube" | "circle-dot" | "link" | "link-plus" | "sparkles" => [183, 135, 242],
        "mountain" => [89, 210, 190],
        "user-plus" | "user-cog" | "accessible" | "hanger" | "shirt" | "checklist" |
        "movie" | "chart-line" => [255, 160, 35],
        "app-window" | "square" | "typography" | "forms" | "border-corner-rounded" |
        "layout-list" | "aspect-ratio" => [229, 126, 174],
        "file-code" | "indent-increase" | "search" | "list-numbers" | "terminal-2" |
        "terminal" | "point" | "stack-2" | "chart-bar" | "activity" => [190, 218, 86],
        _ => [225, 229, 241],
    };
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// A top-row icon button (Play, Pause, Stop, Notifications…).
fn top_icon(ui: &mut egui::Ui, icon: &str, color: Color32, enabled: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(28.0, 24.0), sense);
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 3.0, theme::HOVER_BG);
    }
    let col = if enabled { color } else { color.gamma_multiply(0.35) };
    rustudio_engine::ti::paint(ui.painter(), r.shrink2(Vec2::new(6.0, 4.0)), icon, col);
    resp
}

fn tool_key(t: Tool) -> &'static str {
    match t {
        Tool::None => "tool.none",
        Tool::Transform => "tool.transform",
        Tool::Select => "tool.select",
        Tool::Move => "tool.move",
        Tool::Scale => "tool.scale",
        Tool::Rotate => "tool.rotate",
    }
}

impl Editor {
    /// Under the menus: run type, Play / Pause / Stop, the ribbon tabs in
    /// the middle, the account at the right (Studio's layout).
    pub fn studio_top_row(&mut self, ui: &mut egui::Ui) {
        let mode = self.game.play.as_ref().map(|p| p.mode);
        let full = ui.max_rect();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
            let mut picked = None;
            let resp = egui::ComboBox::from_id_salt("run_type")
                .width(80.0)
                .selected_text(self.ui.run_type.label())
                .show_ui(ui, |ui| {
                    for t in RunType::ALL {
                        let b = egui::Button::selectable(self.ui.run_type == t, t.label());
                        let r = ui.add_enabled(t.available(), b);
                        self.ui_rects.insert(run_type_key(t), r.rect);
                        if r.clicked() {
                            picked = Some(t);
                        }
                    }
                })
                .response;
            self.ui_rects.insert("test.type", resp.rect);
            if let Some(t) = picked {
                self.ui.run_type = t;
            }
            let key = match self.ui.run_type {
                RunType::Test => " (F5)",
                RunType::Run => " (F8)",
                _ => "",
            };
            let tip = format!("{}{key}", self.ui.run_type.label());
            let resp = top_icon(ui, "player-play-filled", theme::ACCENT, mode.is_none()).on_hover_text(tip);
            self.ui_rects.insert("test.play", resp.rect);
            if mode.is_none() && resp.clicked() {
                self.start_run(self.ui.run_type);
            }
            let resp = top_icon(ui, "player-pause-filled", theme::TEXT, false).on_hover_text("Pause: not in rustudio yet");
            self.ui_rects.insert("test.pause", resp.rect);
            let resp = top_icon(ui, "player-stop-filled", Color32::from_rgb(200, 60, 60), mode.is_some())
                .on_hover_text("Stop (Shift+F5)");
            self.ui_rects.insert("test.stop", resp.rect);
            if mode.is_some() && resp.clicked() {
                self.stop_play();
            }
            top_icon(ui, "device-mobile", theme::TEXT, false).on_hover_text("Device Simulator: not in rustudio yet");

            // The tabs, centred like Studio's.
            let tab_w = 80.0;
            let x0 = full.center().x - tab_w * 7.0 / 2.0;
            let y = ui.min_rect().min.y;
            for (i, t) in RibbonTab::ALL.into_iter().enumerate() {
                let r = Rect::from_min_size(egui::pos2(x0 + i as f32 * tab_w, y), egui::vec2(tab_w - 4.0, 24.0));
                let resp = ui.interact(r, egui::Id::new(("ribbon_tab", i)), Sense::click());
                let active = self.ui.tab == t;
                let bg = if active {
                    theme::ACTIVE_TAB
                } else if resp.hovered() {
                    theme::HOVER_BG
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(r, 4.0, bg);
                let col = if active { theme::TEXT } else { theme::TEXT.gamma_multiply(0.75) };
                let font = egui::FontId::proportional(13.0);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, t.label(), font, col);
                self.ui_rects.insert(tab_rect_key(t), r);
                if resp.clicked() {
                    self.ui.tab = t;
                }
            }
            let plus = Rect::from_min_size(egui::pos2(x0 + 6.0 * tab_w + 26.0, y), egui::vec2(24.0, 24.0));
            let font = egui::FontId::proportional(15.0);
            ui.painter().text(plus.center(), egui::Align2::CENTER_CENTER, "+", font, theme::TEXT_DIM);
            ui.interact(plus, egui::Id::new("ribbon_tab_add"), Sense::hover())
                .on_hover_text("Custom tabs: not in rustudio yet");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                // The account: a round button and its menu (Add Account, Log Out).
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                let signed = self.game.account.signed_in();
                let fill = if signed { theme::ACCENT.gamma_multiply(0.8) } else { theme::PANEL_ALT };
                ui.painter().circle_filled(r.center(), 12.0, fill);
                rustudio_engine::ti::paint(ui.painter(), r.shrink(5.0), "user", theme::TEXT);
                self.ui_rects.insert("account", r);
                let resp = resp.on_hover_text(self.game.account.status_short());
                if resp.clicked() {
                    self.game.account.open = !self.game.account.open;
                    self.game.account.adding = false;
                }
                top_icon(ui, "bell", theme::TEXT, false).on_hover_text("Notifications: not in rustudio yet");
                let tc = match (&self.game.doc.cloud, &self.cloud.status) {
                    (Some(o), Some((u, s))) if *u == o.universe_id => s.describe(),
                    _ => "Open a Team Create place to see its live collaborators".into(),
                };
                if top_icon(ui, "users-group", theme::TEXT, false).on_hover_text(tc).clicked() {self.cloud.live_collaborators_open=true;}
                top_icon(ui, "share", theme::TEXT, false).on_hover_text("Share: not in rustudio yet");
                if ui.button("Collaborate").clicked() {self.cloud.live_collaborators_open=true;}
                // (right to left: the MCP button sits left of Collaborate)
                self.mcp_top_button(ui);
                // (and File Sync left of MCP)
                self.file_sync_top_button(ui);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("</> Focus Code")
                                .size(11.5)
                                .strong()
                                .color(crate::wallpaper::LOGO_CYAN),
                        )
                        .fill(Color32::from_rgb(28, 34, 46))
                        .stroke(egui::Stroke::new(1.0, crate::wallpaper::LOGO_CYAN.gamma_multiply(0.45)))
                        .corner_radius(6.0),
                    )
                    .on_hover_text("Enter pure Focus Code IDE (hides 3D)")
                    .clicked()
                {
                    self.ui.focus_code_mode = true;
                    self.ui.viewport_hidden = true;
                }
            });
        });
    }

    /// Touch-friendly mobile quick-toggle bar: Explorer, Properties, Toolbox,
    /// Script Editor, Output, Play/Stop, and Mobile Eco Guard preset.
    pub fn mobile_quick_bar(&mut self, ui: &mut egui::Ui) {
        let drawer = rustudio_core::mobile::is_drawer_mode();
        let playing = self.game.play.is_some();
        let eco = rustudio_core::mobile::is_eco();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);

            let exp_on = if drawer {
                self.ui.mobile_explorer_open
            } else {
                self.ui.panels_open.contains("Explorer")
            };
            if ui.selectable_label(exp_on, "🌳 Explorer").clicked() {
                if drawer {
                    self.ui.mobile_explorer_open = !self.ui.mobile_explorer_open;
                } else {
                    self.ui.panel_requests.push((crate::app::Tab::Explorer, None));
                }
            }

            let prop_on = if drawer {
                self.ui.mobile_properties_open
            } else {
                self.ui.panels_open.contains("Properties")
            };
            if ui.selectable_label(prop_on, "⚙ Properties").clicked() {
                if drawer {
                    self.ui.mobile_properties_open = !self.ui.mobile_properties_open;
                } else {
                    self.ui.panel_requests.push((crate::app::Tab::Properties, None));
                }
            }

            let tb_on = self.ui.panels_open.contains("Toolbox");
            if ui.selectable_label(tb_on, "🧰 Toolbox").clicked() {
                self.ui.panel_requests.push((crate::app::Tab::Toolbox, None));
            }

            if ui.button("📜 Script Editor").clicked() {
                let selected_script = self.game.doc.selection.iter().copied().find(|&r| {
                    self.game.doc.dom.get_by_ref(r).is_some_and(|i| {
                        matches!(i.class.as_str(), "Script" | "LocalScript" | "ModuleScript")
                    })
                });
                let any_script = selected_script.or_else(|| {
                    self.game
                        .doc
                        .dom
                        .descendants_of(self.game.doc.dom.root_ref())
                        .find(|i| {
                            matches!(i.class.as_str(), "Script" | "LocalScript" | "ModuleScript")
                        })
                        .map(|i| i.referent())
                });
                if let Some(r) = any_script {
                    if !self.open_script.contains(&r) {
                        self.open_script.push(r);
                    }
                } else {
                    self.ui_action(ui.ctx(), "script");
                }
            }

            let out_on = self.ui.panels_open.contains("Output");
            if ui.selectable_label(out_on, "📟 Output").clicked() {
                self.ui.panel_requests.push((crate::app::Tab::Output, None));
            }

            if playing {
                if ui
                    .button(egui::RichText::new("⏹ Stop").color(Color32::from_rgb(255, 100, 100)).strong())
                    .clicked()
                {
                    self.stop_play();
                }
            } else if ui
                .button(egui::RichText::new("▶ Play").color(Color32::from_rgb(100, 230, 120)).strong())
                .clicked()
            {
                self.start_run(self.ui.run_type);
            }

            let cfg = rustudio_core::hardware_tuner::config();
            let perf_id = egui::Id::new("mobile_perf_popup_open");
            let perf_open = ui.ctx().data(|d| d.get_temp::<bool>(perf_id).unwrap_or(false));
            let eco_label = if cfg.render_scale < 0.8 {
                format!("🔋 {} ({:.0}%)", cfg.preset.label(), cfg.render_scale * 100.0)
            } else {
                format!("⚡ {} ({} FPS)", cfg.preset.label(), if cfg.fps_limit == 0 { "Max".to_owned() } else { cfg.fps_limit.to_string() })
            };
            if ui
                .selectable_label(perf_open, eco_label)
                .on_hover_text("Mở cấu hình Phần cứng, RAM/Cache & FPS")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(perf_id, !perf_open));
            }

            if ui
                .selectable_label(drawer, "📱 Mobile Drawers")
                .on_hover_text("Hiển thị Explorer & Properties dạng ngăn kéo nổi 2 bên")
                .clicked()
            {
                rustudio_core::mobile::set_drawer_mode(!drawer);
            }
            if ui
                .selectable_label(crate::mcp_assistant::is_open(), "🤖 AI (MCP)")
                .on_hover_text("Mở Studio AI Assistant & MCP Engine")
                .clicked()
            {
                crate::mcp_assistant::toggle_open();
            }
        });
        Self::hardware_perf_popup_ui(ui.ctx());
        crate::mcp_assistant::ui(self, ui.ctx());
        let has_open_plugin = self.has_open_plugin();
        crate::touch_input_bridge::floating_shortcut_pill_ui(ui.ctx(), has_open_plugin);
    }

    fn ribbon_tools(&mut self, ui: &mut egui::Ui, act: &mut Option<String>) {
        for (icon, label, a, tool) in [
            ("pointer", "Select", "tool_select", Tool::Select),
            ("arrows-move", "Move", "tool_move", Tool::Move),
            ("resize", "Scale", "tool_scale", Tool::Scale),
            ("rotate-clockwise-2", "Rotate", "tool_rotate", Tool::Rotate),
        ] {
            let on = self.tool == tool;
            let r = self.rb(ui, icon, label, a, on, act);
            self.ui_rects.insert(tool_key(tool), r);
            self.hud.toolbar_buttons += 1;
        }
        self.rb(ui, "transform", "Transform", "transform", self.tool == Tool::Transform, act);
        divider(ui);
        self.movement_mode_button(ui);
        self.snap_widget(ui);
        divider(ui);
    }

    /// The selected ribbon tab's buttons, grouped with dividers like Studio's.
    pub fn studio_ribbon(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut act: Option<String> = None;
        if self.ui.tab != RibbonTab::Plugins {
            self.plugins_tab_hidden();
        }
        let a = &mut act;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
            match self.ui.tab {
                RibbonTab::Home => {
                    self.ribbon_tools(ui, a);
                    self.part_button(ui, a);
                    self.rb(ui, "mountain", "Terrain", "terrain_editor", false, a);
                    let r = self.rb(ui, "user-plus", "Character", "character", self.chars.open, a);
                    self.ui_rects.insert("home.character", r);
                    self.rb(ui, "app-window", "GUI", "gui", false, a);
                    self.rb(ui, "file-code", "Script", "script", false, a);
                    self.rb(ui, "square-plus", "Object", "insert_object", false, a);
                    self.rb(ui, "file-import", "Import", "import_model", false, a);
                    divider(ui);
                    self.material_quick_button(ui);
                    self.color_quick_button(ui);
                    self.rb(ui, "box-multiple", "Group", "group", false, a);
                    let lock = self.action_state("lock").1 == Some(true);
                    self.rb(ui, "lock", "Lock", "lock", lock, a);
                    let anchored = self.action_state("anchor").1 == Some(true);
                    let r = self.rb(ui, "anchor", "Anchor", "anchor", anchored, a);
                    self.ui_rects.insert("home.anchor", r);
                    divider(ui);
                    let e = self.ui.panels_open.contains("Explorer");
                    self.rb(ui, "list-tree", "Explorer", "panel_explorer", e, a);
                    let p = self.ui.panels_open.contains("Properties");
                    self.rb(ui, "list-details", "Properties", "panel_properties", p, a);
                    self.rb(ui, "tools", "Toolbox", "toolbox", false, a);
                    self.rb(ui, "folders", "Assets", "asset_manager", false, a);
                }
                RibbonTab::Model => {
                    self.ribbon_tools(ui, a);
                    self.rb(ui, "focus-centered", "Pivot", "pivot", false, a);
                    self.rb(ui, "restore", "Reset", "pivot_reset", false, a);
                    divider(ui);
                    self.part_button(ui, a);
                    let fx = ["ParticleEmitter", "Beam", "Trail", "Fire", "Smoke", "Sparkles", "Highlight"];
                    self.class_menu_button(ui, "sparkles", "Effect", "effect", &fx, a);
                    divider(ui);
                    self.rb(ui, "circle-dot", "Attachment", "attachment", false, a);
                    self.rb(ui, "link", "Constraint", "constraint", false, a);
                    self.rb(ui, "link-plus", "Weld", "weld", false, a);
                    divider(ui);
                    self.material_quick_button(ui);
                    self.color_quick_button(ui);
                    self.rb(ui, "photo", "Texture", "texture_tool", false, a);
                    self.rb(ui, "box-multiple", "Group", "group", false, a);
                    let lock = self.action_state("lock").1 == Some(true);
                    self.rb(ui, "lock", "Lock", "lock", lock, a);
                    let anchored = self.action_state("anchor").1 == Some(true);
                    self.rb(ui, "anchor", "Anchor", "anchor", anchored, a);
                    divider(ui);
                    self.rb(ui, "layout-align-center", "Align", "align", false, a);
                }
                RibbonTab::Avatar => {
                    self.ribbon_tools(ui, a);
                    let open = self.ui.panels_open.contains("Avatar Settings");
                    let r = self.rb(ui, "user-cog", "Settings", "avatar_settings", open, a);
                    self.ui_rects.insert("avatar.settings", r);
                    divider(ui);
                    let r = self.rb(ui, "user-plus", "Character", "character", self.chars.open, a);
                    self.ui_rects.insert("avatar.character", r);
                    self.rb(ui, "accessible", "Setup", "avatar_setup", false, a);
                    self.rb(ui, "hanger", "Accessory", "accessory_editor", false, a);
                    self.rb(ui, "shirt", "Adaptive…", "adaptive", false, a);
                    self.rb(ui, "checklist", "UGC Validati…", "ugc_validation", false, a);
                    divider(ui);
                    self.rb(ui, "movie", "Clip Editor", "clip_editor", false, a);
                    self.rb(ui, "chart-line", "Graph Editor", "graph_editor", false, a);
                }
                RibbonTab::Ui => {
                    self.rb(ui, "app-window", "GUI", "gui", false, a);
                    self.rb_class(ui, "square", "Frame", "ui_frame", "Frame", a);
                    self.rb_class(ui, "typography", "Label", "ui_label", "TextLabel", a);
                    self.rb_class(ui, "forms", "Input", "ui_input", "TextBox", a);
                    let ap = ["UICorner", "UIStroke", "UIGradient", "UIPadding", "UIScale"];
                    self.class_menu_button(ui, "border-corner-rounded", "Appearance", "ui_appearance", &ap, a);
                    let la = ["UIListLayout", "UIGridLayout", "UIPageLayout", "UITableLayout", "UIFlexItem"];
                    self.class_menu_button(ui, "layout-list", "Layout", "ui_layout", &la, a);
                    let co = ["UIAspectRatioConstraint", "UISizeConstraint", "UITextSizeConstraint"];
                    self.class_menu_button(ui, "aspect-ratio", "Constraint", "ui_constraint", &co, a);
                    divider(ui);
                    self.rb(ui, "brush", "Style Editor", "style_editor", false, a);
                }
                RibbonTab::Script => {
                    self.rb(ui, "chevron-left", "Back", "script_back", false, a);
                    self.rb(ui, "chevron-right", "Fwd", "script_fwd", false, a);
                    divider(ui);
                    self.rb(ui, "file-code", "Script", "script", false, a);
                    divider(ui);
                    self.rb(ui, "indent-increase", "Format", "format_document", false, a);
                    self.rb(ui, "search", "Find", "find_in_place", false, a);
                    self.rb(ui, "list-numbers", "Go to Line", "goto_line", false, a);
                    divider(ui);
                    self.rb(ui, "terminal-2", "Command", "command_bar", false, a);
                    let o = self.ui.panels_open.contains("Output");
                    self.rb(ui, "terminal", "Output", "panel_output", o, a);
                    divider(ui);
                    self.rb(ui, "point", "Breakpoints", "breakpoints", false, a);
                    self.rb(ui, "stack-2", "Call Stack", "call_stack", false, a);
                    self.rb(ui, "eye", "Watch", "watch", false, a);
                    divider(ui);
                    self.rb(ui, "chart-bar", "Analysis", "script_analysis", false, a);
                    self.rb(ui, "activity", "Script Activity", "script_activity", false, a);
                }
                // Only plugin toolbars, as Studio's (Manage Plugins and
                // Plugins Folder are in the Plugins menu): src/plugin_host/ui_ribbon.rs.
                RibbonTab::Plugins => self.plugins_ribbon(ui),
            }
        });
        self.ribbon_color_popup(&ctx);
        if let Some(a) = act {
            self.ui_action(&ctx, &a);
        }
    }

    /// A ribbon button for an action; disabled ones say so. Returns its rect.
    fn rb(&mut self, ui: &mut egui::Ui, icon: &str, label: &str, action: &str, active: bool, act: &mut Option<String>) -> Rect {
        let (enabled, _) = self.action_state(action);
        let resp = ribbon_button(ui, icon, label, active, enabled);
        let rect = resp.rect;
        if enabled {
            if resp.clicked() {
                *act = Some(action.to_string());
            }
        } else {
            resp.on_hover_text(format!("{label}: not in rustudio yet"));
        }
        rect
    }

    fn rb_class(&mut self, ui: &mut egui::Ui, icon: &str, label: &str, action: &str, class: &str, act: &mut Option<String>) {
        let (enabled, _) = self.action_state(action);
        let resp = ribbon_button(ui, icon, label, false, enabled);
        if enabled {
            if resp.clicked() {
                *act = Some(format!("class:{class}"));
            }
        } else {
            resp.on_hover_text(format!("Select a GUI object to add a {class} to it"));
        }
    }

    /// A ribbon button whose drop-down inserts one of `classes` into the selection.
    fn class_menu_button(&mut self, ui: &mut egui::Ui, icon: &str, label: &str, action: &str, classes: &[&str], act: &mut Option<String>) {
        let (enabled, _) = self.action_state(action);
        let resp = ribbon_button(ui, icon, label, false, enabled);
        if !enabled {
            resp.on_hover_text(format!("{label}: select where it goes first"));
            return;
        }
        egui::Popup::menu(&resp).show(|ui| {
            for c in classes {
                ui.horizontal(|ui| {
                    crate::icons::class_icon(ui, c, 16.0);
                    if ui.selectable_label(false, *c).clicked() {
                        *act = Some(format!("class:{c}"));
                        ui.close();
                    }
                });
            }
        });
    }

    /// Part: opens a touch-friendly shape picker popup (Block, Sphere, Wedge, Corner Wedge, Cylinder).
    fn part_button(&mut self, ui: &mut egui::Ui, act: &mut Option<String>) {
        let enabled = self.action_state("part").0;
        let resp = ribbon_button(ui, "cube", "Part", false, enabled);
        self.ui_rects.insert("insert.part", resp.rect);
        self.hud.toolbar_buttons += 1;
        let arrow = Rect::from_min_size(resp.rect.right_top() + Vec2::new(-13.0, 18.0), Vec2::splat(12.0));
        rustudio_engine::ti::paint(ui.painter(), arrow, "chevron-down", theme::TEXT_DIM);
        let ar = ui.interact(arrow, egui::Id::new("part_arrow"), Sense::click());
        let pop_id = egui::Id::new("part_shape_popup_open");
        if enabled && (resp.clicked() || ar.clicked()) {
            ui.ctx().data_mut(|d| {
                let cur = d.get_temp::<bool>(pop_id).unwrap_or(false);
                d.insert_temp(pop_id, !cur);
            });
        }
        let open = ui.ctx().data(|d| d.get_temp::<bool>(pop_id).unwrap_or(false));
        if open {
            let mut close = false;
            let screen = ui.ctx().content_rect();
            let mobile = rustudio_core::mobile::is_drawer_mode() || screen.width() < 700.0;
            let area = if mobile {
                egui::Area::new(egui::Id::new("part_shape_popup"))
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            } else {
                egui::Area::new(egui::Id::new("part_shape_popup"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(resp.rect.left_bottom() + Vec2::new(0.0, 4.0))
            };
            let area_resp = area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme::PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, theme::DIVIDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        let w = if mobile { (screen.width() - 64.0).clamp(220.0, 320.0) } else { 210.0 };
                        ui.set_width(w);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Part").strong().size(16.0).color(Color32::WHITE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").clicked() {
                                    close = true;
                                }
                            });
                        });
                        ui.add_space(6.0);
                        for (label, a) in [
                            ("Block", "part_block"),
                            ("Sphere", "part_sphere"),
                            ("Wedge", "part_wedge"),
                            ("Corner Wedge", "part_corner"),
                            ("Cylinder", "part_cylinder"),
                        ] {
                            let btn = egui::Button::new(egui::RichText::new(label).size(15.0).color(theme::TEXT))
                                .fill(Color32::TRANSPARENT)
                                .min_size(Vec2::new(w, 38.0));
                            if ui.add(btn).clicked() {
                                *act = Some(a.into());
                                close = true;
                            }
                        }
                    });
            });
            let outside = ui.ctx().input(|i| i.pointer.any_pressed())
                && !ui.ctx().pointer_interact_pos().is_some_and(|p| area_resp.response.rect.contains(p) || resp.rect.contains(p));
            if close || outside || ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                ui.ctx().data_mut(|d| d.insert_temp(pop_id, false));
            }
        }
    }

    /// The dragger settings drop-down, labelled with the movement mode
    /// (Studio's "Geometric ▾"): Dragger modes Geometric | Physical
    /// (DraggerService.DraggerMovementMode), Collisions (plugin.CollisionEnabled),
    /// Join Surfaces (plugin:GetJoinMode()), then Snap: Use Bounding Boxes
    /// (DraggerService.UseBoundingBoxes) and Snap to Parts (PartSnapEnabled).
    fn movement_mode_button(&mut self, ui: &mut egui::Ui) {
        let mode = if self.physical_drag { "Physical" } else { "Geometric" };
        let resp = ribbon_button(ui, if self.physical_drag { "atom-2" } else { "geometry" }, mode, false, true);
        self.ui_rects.insert("model.geometric", resp.rect);
        let arrow = Rect::from_min_size(resp.rect.right_top() + Vec2::new(-13.0, 18.0), Vec2::splat(12.0));
        rustudio_engine::ti::paint(ui.painter(), arrow, "chevron-down", theme::TEXT_DIM);
        let resp = resp.on_hover_text("Dragger settings: movement mode, collisions, joins and snapping");
        egui::Popup::menu(&resp)
            .id(egui::Id::new(DRAGGER_MENU_ID))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| self.dragger_menu(ui));
    }

    fn dragger_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_width(186.0);
        ui.spacing_mut().item_spacing.y = 6.0;
        let head = |ui: &mut egui::Ui, t: &str| {
            ui.label(egui::RichText::new(t).size(12.5).color(theme::TEXT));
        };
        ui.add_space(2.0);
        head(ui, "Dragger modes");
        // Segmented Geometric | Physical toggle.
        let (r, _) = ui.allocate_exact_size(Vec2::new(186.0, 26.0), Sense::hover());
        ui.painter().rect_filled(r, 4.0, Color32::from_rgb(30, 31, 35));
        ui.painter().rect_stroke(r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(70, 72, 78)), egui::StrokeKind::Inside);
        for (i, (label, physical)) in [("Geometric", false), ("Physical", true)].into_iter().enumerate() {
            let half = Rect::from_min_size(r.min + Vec2::new(i as f32 * r.width() / 2.0, 0.0), Vec2::new(r.width() / 2.0, r.height()));
            let resp = ui.interact(half, egui::Id::new(("dragger_mode", i)), Sense::click());
            self.ui_rects.insert(if physical { "dragger.physical" } else { "dragger.geometric" }, half);
            let on = self.physical_drag == physical;
            if on {
                ui.painter().rect_filled(half.shrink(2.0), 3.0, Color32::from_rgb(64, 66, 72));
            } else if resp.hovered() {
                ui.painter().rect_filled(half.shrink(2.0), 3.0, theme::HOVER_BG);
            }
            ui.painter().text(half.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0),
                if on { Color32::WHITE } else { theme::TEXT });
            if resp.clicked() {
                self.physical_drag = physical;
            }
        }
        ui.add_space(2.0);
        let r = ui.checkbox(&mut self.collisions, "Collisions")
            .on_hover_text("Physical drags and handle moves / scales stop where parts would overlap");
        self.ui_rects.insert("dragger.collisions", r.rect);
        let r = ui.checkbox(&mut self.join_surfaces, "Join Surfaces")
            .on_hover_text("Dropped parts are welded to the parts they touch");
        self.ui_rects.insert("dragger.join", r.rect);
        ui.add_space(2.0);
        head(ui, "Snap");
        let r = ui.checkbox(&mut self.use_bounding_boxes, "Use Bounding Boxes")
            .on_hover_text("Drop onto the bounding box of the part under the mouse instead of its shape");
        self.ui_rects.insert("dragger.bbox", r.rect);
        let r = ui.checkbox(&mut self.part_snap, "Snap to Parts")
            .on_hover_text("Snap the dragged selection's edges and centre to the target's (Shift+C)");
        self.ui_rects.insert("dragger.part_snap", r.rect);
        ui.add_space(2.0);
    }

    /// Studio's snap controls: a check box and an increment for moves, then
    /// for rotations. Studio turns a snap on when its increment is changed to
    /// more than zero.
    fn snap_widget(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.game.snap_move, "").on_hover_text("Snap moves to the grid");
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                rustudio_engine::ti::paint(ui.painter(), r, "arrows-move", theme::TEXT_DIM);
                let dv = egui::DragValue::new(&mut self.game.move_inc).range(0.01..=512.0).speed(0.1).suffix(" studs");
                if ui.add(dv).changed() && self.game.move_inc > 0.0 {
                    self.game.snap_move = true;
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.game.snap_rotate, "").on_hover_text("Snap rotation");
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                rustudio_engine::ti::paint(ui.painter(), r, "rotate", theme::TEXT_DIM);
                let dv = egui::DragValue::new(&mut self.game.rotate_inc).range(0.1..=180.0).speed(0.5).suffix("°");
                if ui.add(dv).changed() && self.game.rotate_inc > 0.0 {
                    self.game.snap_rotate = true;
                }
            });
        });
    }

    /// Portrait phone layout (`screen_rect.width() < screen_rect.height()`):
    /// - 44px single-row top bar with full `rustudio` `☰ Menu`
    /// - Horizontal Scrollable Multi-Tab Bar (`3D View` + open Script tabs with `×` + `+`)
    /// - Right Slide-Over Drawer (~75% width, full height) for Explorer + Properties with draggable splitter
    /// - 5-button Bottom Bar (`3D`, `Code`, `Explorer`, `Output`, `+` Quick Tools & Plugins grid)
    /// - Touch Luau Snippet Bar above Bottom Bar when in Code tab
    pub fn portrait_layout_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        use crate::studio_ui::PortraitTab;

        if std::mem::take(&mut self.switch_to_viewport) {
            self.ui.portrait_tab = PortraitTab::Viewport;
            self.ui.portrait_tool_panel = None;
            self.ui.portrait_explorer_drawer = false;
            self.ui.portrait_output_drawer = false;
            self.ui.portrait_quick_grid_open = false;
        }

        // Sync any scripts opened from Explorer into the Multi-Tab Bar
        for r in std::mem::take(&mut self.open_script) {
            if !self.ui.portrait_open_scripts.contains(&r) {
                self.ui.portrait_open_scripts.push(r);
            }
            self.ui.portrait_active_script = Some(r);
            self.ui.portrait_tab = PortraitTab::Code;
            self.ui.portrait_tool_panel = None;
        }
        self.ui
            .portrait_open_scripts
            .retain(|r| self.game.doc.dom.get_by_ref(*r).is_some());

        for (tab, want) in std::mem::take(&mut self.ui.panel_requests) {
            match tab {
                crate::app::Tab::Explorer | crate::app::Tab::Properties => {
                    self.ui.portrait_explorer_drawer = want.unwrap_or(!self.ui.portrait_explorer_drawer);
                }
                crate::app::Tab::Output => {
                    self.ui.portrait_output_drawer = want.unwrap_or(!self.ui.portrait_output_drawer);
                }
                crate::app::Tab::Toolbox | crate::app::Tab::TerrainEditor => {
                    let show = want.unwrap_or(self.ui.portrait_tool_panel != Some(tab));
                    self.ui.portrait_tool_panel = if show { Some(tab) } else { None };
                }
                _ => {}
            }
        }

        // 1. Modern Roblox Studio Single Top Bar (File | Home Model Avatar UI ... | Undo Redo Play Perf Account)
        let (safe_top, safe_bottom, safe_left, safe_right) = crate::platform::get_safe_area_insets();
        let top_pad = if safe_top > 0.0 { (safe_top + 6.0).round().clamp(0.0, 120.0) as i8 } else { 3 };
        let side_left = (6.0 + safe_left).round().clamp(0.0, 120.0) as i8;
        let side_right = (6.0 + safe_right).round().clamp(0.0, 120.0) as i8;
        let bot_pad = (6.0 + safe_bottom).round().clamp(0.0, 120.0) as i8;

        let ribbon_open_id = egui::Id::new("portrait_ribbon_expanded");
        let mut ribbon_open = ctx.data(|d| d.get_temp::<bool>(ribbon_open_id).unwrap_or(true));
        egui::Panel::top("portrait_top_bar")
            .exact_size(34.0 + if safe_top > 0.0 { safe_top + 3.0 } else { 0.0 })
            .frame(
                egui::Frame::new()
                    .fill(theme::MENU_BAR)
                    .inner_margin(egui::Margin {
                        left: side_left,
                        right: side_right,
                        top: top_pad,
                        bottom: 3,
                    }),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                    let file_btn = egui::Button::new(egui::RichText::new("File").color(Color32::WHITE).strong())
                        .fill(theme::SELECT)
                        .corner_radius(4.0);
                    let file_resp = ui.add(file_btn);
                    egui::Popup::menu(&file_resp).show(|ui| {
                        ui.label(egui::RichText::new("File").strong().small().color(theme::TEXT_DIM));
                        if ui.button("New Place").clicked() { self.ui_action(ctx, "new"); ui.close(); }
                        if ui.button("Open .rbxl...").clicked() { self.ui_action(ctx, "open"); ui.close(); }
                        if ui.button("Open .project.json...").clicked() { self.ui_action(ctx, "open_folder"); ui.close(); }
                        if ui.button("Open from Roblox Cloud...").clicked() { self.ui_action(ctx, "open_roblox"); ui.close(); }
                        if ui.button("Save").clicked() { self.ui_action(ctx, "save"); ui.close(); }
                        if ui.button("Save to Roblox As...").clicked() { self.ui_action(ctx, "save_roblox_as"); ui.close(); }
                        if ui.button("Publish to Roblox").clicked() { self.ui_action(ctx, "save_roblox"); ui.close(); }
                        if ui
                            .button(egui::RichText::new("Close Place (Home)").color(Color32::from_rgb(255, 115, 115)))
                            .clicked()
                        {
                            self.ui_action(ctx, "close_place");
                            ui.close();
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Tools & Settings").strong().small().color(theme::TEXT_DIM));
                        if ui.button("🎨 Themes & Wallpapers...").clicked() { crate::wallpaper::toggle_modal(ctx); ui.close(); }
                        if ui.button("Toolbox").clicked() { self.ui_action(ctx, "toolbox"); ui.close(); }
                        if ui.button("Manage Plugins...").clicked() { self.ui_action(ctx, "manage_plugins"); ui.close(); }
                        if ui.button("Studio Settings...").clicked() { self.ui_action(ctx, "studio_settings"); ui.close(); }
                        if ui.button("Fonts & Typography...").clicked() { self.ui_action(ctx, "studio_settings:Fonts & Typography"); ui.close(); }
                        if ui.button("AI Assistant (MCP)").clicked() { crate::mcp_assistant::toggle_open(); ui.close(); }
                    });

                    // Compact Category Pill Dropdown Box (replaces cramped horizontal ribbon tab scroll)
                    let cat_pop_id = egui::Id::new("portrait_cat_dropdown_open");
                    let mut cat_open = ctx.data(|d| d.get_temp::<bool>(cat_pop_id).unwrap_or(false));
                    let cat_icon = match self.ui.tab {
                        RibbonTab::Home => "pointer",
                        RibbonTab::Model => "cube",
                        RibbonTab::Avatar => "user-cog",
                        RibbonTab::Ui => "app-window",
                        RibbonTab::Script => "file-code",
                        RibbonTab::Plugins => "sparkles",
                    };
                    let cat_accent = match self.ui.tab {
                        RibbonTab::Home => crate::wallpaper::PASTEL_LAVENDER,
                        RibbonTab::Model => crate::wallpaper::PASTEL_MINT,
                        RibbonTab::Avatar => crate::wallpaper::PASTEL_GOLD,
                        RibbonTab::Ui => crate::wallpaper::PASTEL_ROSE,
                        RibbonTab::Script => crate::wallpaper::PASTEL_SKY,
                        RibbonTab::Plugins => crate::wallpaper::PASTEL_PEACH,
                    };
                    let pill_label = format!("{} ▾", self.ui.tab.label());
                    let font = egui::FontId::proportional(12.5);
                    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(pill_label.clone(), font.clone(), Color32::WHITE).size().x);
                    let pill_sz = egui::vec2((text_w + 34.0).max(86.0), 26.0);
                    let (pill_rect, pill_resp) = ui.allocate_exact_size(pill_sz, Sense::click());
                    let pill_bg = if cat_open {
                        Color32::from_rgba_unmultiplied(40, 44, 58, 240)
                    } else if pill_resp.hovered() {
                        Color32::from_rgba_unmultiplied(32, 36, 48, 230)
                    } else {
                        Color32::from_rgba_unmultiplied(24, 27, 36, 220)
                    };
                    ui.painter().rect_filled(pill_rect, egui::CornerRadius::same(13), pill_bg);
                    ui.painter().rect_stroke(
                        pill_rect,
                        egui::CornerRadius::same(13),
                        egui::Stroke::new(1.0, cat_accent.gamma_multiply(0.65)),
                        egui::StrokeKind::Inside,
                    );
                    let ic_rect = Rect::from_center_size(
                        egui::pos2(pill_rect.left() + 14.0, pill_rect.center().y),
                        Vec2::splat(14.0),
                    );
                    rustudio_engine::ti::paint(ui.painter(), ic_rect, cat_icon, cat_accent);
                    ui.painter().text(
                        egui::pos2(pill_rect.left() + 26.0, pill_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &pill_label,
                        font,
                        Color32::WHITE,
                    );
                    if pill_resp.clicked() {
                        cat_open = !cat_open;
                        ctx.data_mut(|d| d.insert_temp(cat_pop_id, cat_open));
                    }
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("portrait_cat_pill_rect"), pill_rect));

                    let arrow = if ribbon_open { "▴" } else { "▾" };
                    if ui.small_button(arrow).on_hover_text("Thu gọn/mở thanh công cụ Ribbon").clicked() {
                        ribbon_open = !ribbon_open;
                        ctx.data_mut(|d| d.insert_temp(ribbon_open_id, ribbon_open));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let signed = self.game.account.signed_in();
                        let acc_label = if signed { "✓" } else { "Sign In" };
                        if ui.selectable_label(signed, acc_label).on_hover_text("Roblox Account").clicked() {
                            self.game.account.open = !self.game.account.open;
                            self.game.account.adding = false;
                        }

                        if ui.small_button("🎨").on_hover_text("Themes & Wallpapers").clicked() {
                            crate::wallpaper::toggle_modal(ctx);
                        }

                        let cfg = rustudio_core::hardware_tuner::config();
                        let perf_id = egui::Id::new("mobile_perf_popup_open");
                        let perf_open = ctx.data(|d| d.get_temp::<bool>(perf_id).unwrap_or(false));
                        let eco_btn = if cfg.render_scale < 0.7 { "Eco" } else { "Perf" };
                        if ui.selectable_label(perf_open, eco_btn).on_hover_text("Performance").clicked() {
                            ctx.data_mut(|d| d.insert_temp(perf_id, !perf_open));
                        }

                        let playing = self.game.play.is_some();
                        if playing {
                            if ui
                                .add(egui::Button::new(egui::RichText::new("■ Stop").color(Color32::WHITE).strong()).fill(Color32::from_rgb(185, 50, 50)))
                                .clicked()
                            {
                                self.stop_play();
                            }
                        } else if ui
                            .add(egui::Button::new(egui::RichText::new("▶ Play").color(Color32::WHITE).strong()).fill(Color32::from_rgb(38, 138, 72)))
                            .clicked()
                        {
                            self.start_run(self.ui.run_type);
                        }

                        if ui.small_button("↷").on_hover_text("Redo").clicked() {
                            self.ui_action(ctx, "redo");
                        }
                        if ui.small_button("↶").on_hover_text("Undo").clicked() {
                            self.ui_action(ctx, "undo");
                        }
                    });
                });
            });

        // Floating Rounded Category Popover Menu with 180ms Ease-Out-Back Spring Pop-In,
        // Staggered 12ms Item Reveal, and 1px Glass-Edge Rim
        {
            let cat_pop_id = egui::Id::new("portrait_cat_dropdown_open");
            let mut cat_open = ctx.data(|d| d.get_temp::<bool>(cat_pop_id).unwrap_or(false));
            let anim = ctx.animate_bool_with_time(egui::Id::new("portrait_cat_pop_anim"), cat_open, 0.18);
            if cat_open || anim > 0.01 {
                let spring = crate::wallpaper::ease_out_back(anim);
                let scale = 0.92 + 0.08 * spring;
                let slide_y = (1.0 - anim) * -8.0;
                let anchor_rect = ctx
                    .data(|d| d.get_temp::<Rect>(egui::Id::new("portrait_cat_pill_rect")))
                    .unwrap_or_else(|| Rect::from_min_size(egui::pos2(56.0, 44.0), Vec2::new(90.0, 26.0)));
                let pop_pos = egui::pos2(anchor_rect.left(), anchor_rect.bottom() + 6.0 + slide_y);
                let shown = egui::Area::new(egui::Id::new("portrait_cat_dropdown_area"))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .fixed_pos(pop_pos)
                    .show(ctx, |ui| {
                        let frame = egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(20, 22, 30, (242.0 * anim.clamp(0.0, 1.0)) as u8))
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::same(8))
                            .shadow(egui::epaint::Shadow {
                                offset: [0, 10],
                                blur: 24,
                                spread: 0,
                                color: Color32::from_black_alpha((135.0 * anim.clamp(0.0, 1.0)) as u8),
                            });
                        frame.show(ui, |ui| {
                            ui.set_width((184.0 * scale).max(150.0));
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                            ui.label(
                                egui::RichText::new("RIBBON CATEGORY")
                                    .size(10.5)
                                    .strong()
                                    .color(theme::TEXT_DIM),
                            );
                            for (idx, (tab, icon, accent, sub)) in [
                                (RibbonTab::Home, "pointer", crate::wallpaper::PASTEL_LAVENDER, "Select, Move, Scale"),
                                (RibbonTab::Model, "cube", crate::wallpaper::PASTEL_MINT, "Parts, Rig, Snap"),
                                (RibbonTab::Avatar, "user-cog", crate::wallpaper::PASTEL_GOLD, "Rig Builder & Animations"),
                                (RibbonTab::Ui, "app-window", crate::wallpaper::PASTEL_ROSE, "ScreenGui, Frame, Text"),
                                (RibbonTab::Script, "file-code", crate::wallpaper::PASTEL_SKY, "Luau Scripts & Debug"),
                                (RibbonTab::Plugins, "sparkles", crate::wallpaper::PASTEL_PEACH, "Installed Studio Plugins"),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                let item_alpha = crate::wallpaper::staggered_item_alpha(anim, idx);
                                let item_slide_x = (1.0 - item_alpha) * -6.0;
                                let selected = self.ui.tab == tab;
                                let (base_r, row_resp) = ui.allocate_exact_size(Vec2::new(168.0 * scale, 34.0 * scale), Sense::click());
                                let press_squish = if row_resp.is_pointer_button_down_on() { 0.96 } else { 1.0 };
                                let row_r = Rect::from_center_size(
                                    base_r.center() + Vec2::new(item_slide_x, 0.0),
                                    base_r.size() * press_squish,
                                );
                                let fill = if selected {
                                    accent.gamma_multiply(0.22 * item_alpha)
                                } else if row_resp.hovered() {
                                    Color32::from_rgba_unmultiplied(45, 50, 64, (180.0 * item_alpha) as u8)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(row_r, egui::CornerRadius::same(8), fill);
                                if selected {
                                    ui.painter().rect_stroke(
                                        row_r,
                                        egui::CornerRadius::same(8),
                                        egui::Stroke::new(1.0, accent.gamma_multiply(0.75 * item_alpha)),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                let ic_r = Rect::from_center_size(
                                    egui::pos2(row_r.left() + 16.0, row_r.center().y),
                                    Vec2::splat(16.0),
                                );
                                rustudio_engine::ti::paint(ui.painter(), ic_r, icon, accent.gamma_multiply(item_alpha));
                                ui.painter().text(
                                    egui::pos2(row_r.left() + 32.0, row_r.top() + 11.0),
                                    egui::Align2::LEFT_CENTER,
                                    tab.label(),
                                    egui::FontId::proportional(13.0),
                                    Color32::WHITE.gamma_multiply(item_alpha),
                                );
                                ui.painter().text(
                                    egui::pos2(row_r.left() + 32.0, row_r.bottom() - 8.0),
                                    egui::Align2::LEFT_CENTER,
                                    sub,
                                    egui::FontId::proportional(10.0),
                                    theme::TEXT_DIM.gamma_multiply(item_alpha),
                                );
                                if row_resp.clicked() {
                                    self.ui.tab = tab;
                                    ribbon_open = true;
                                    ctx.data_mut(|d| {
                                        d.insert_temp(ribbon_open_id, true);
                                        d.insert_temp(cat_pop_id, false);
                                    });
                                    cat_open = false;
                                }
                            }
                            ui.separator();
                            if ui
                                .button(
                                    egui::RichText::new("⛰ Open Terrain Editor")
                                        .size(12.0)
                                        .color(crate::wallpaper::PASTEL_MINT),
                                )
                                .clicked()
                            {
                                self.ui.portrait_tool_panel = Some(crate::app::Tab::TerrainEditor);
                                ctx.data_mut(|d| d.insert_temp(cat_pop_id, false));
                                cat_open = false;
                            }
                        })
                    });
                let pop_rect = shown.inner.response.rect;
                ctx.layer_painter(shown.response.layer_id).rect_stroke(
                    pop_rect,
                    egui::CornerRadius::same(14),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
                let clicked_out = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| {
                            !pop_rect.expand(8.0).contains(p) && !anchor_rect.expand(4.0).contains(p)
                        })
                });
                if clicked_out {
                    ctx.data_mut(|d| d.insert_temp(cat_pop_id, false));
                }
            }
        }

        if ribbon_open {
            egui::Panel::top("portrait_ribbon_tools")
                .exact_size(58.0)
                .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(6, 4)))
                .show(ui, |ui| {
                    egui::ScrollArea::horizontal().id_salt("portrait_ribbon_tools_scroll").show(ui, |ui| {
                        self.studio_ribbon(ui);
                    });
                });
        }

        // 2. Horizontal Scrollable Multi-Tab Bar (Roblox Studio PC style)
        egui::Panel::top("portrait_multi_tabs")
            .exact_size(32.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL_DARK)
                    .inner_margin(egui::Margin::symmetric(6, 3)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::horizontal().id_salt("portrait_tabs_scroll").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                        let is_3d = self.ui.portrait_tab == PortraitTab::Viewport
                            && self.ui.portrait_tool_panel.is_none();
                        if ui.selectable_label(is_3d, "Workspace").clicked() {
                            self.ui.portrait_tab = PortraitTab::Viewport;
                            self.ui.portrait_tool_panel = None;
                        }

                        let mut close_idx: Option<usize> = None;
                        let open_list = self.ui.portrait_open_scripts.clone();
                        for (idx, r) in open_list.into_iter().enumerate() {
                            let name = self
                                .game
                                .doc
                                .dom
                                .get_by_ref(r)
                                .map(|i| i.name.as_str())
                                .unwrap_or("Script");
                            let active = self.ui.portrait_tab == PortraitTab::Code
                                && self.ui.portrait_active_script == Some(r)
                                && self.ui.portrait_tool_panel.is_none();
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                if ui.selectable_label(active, name).clicked() {
                                    self.ui.portrait_active_script = Some(r);
                                    self.ui.portrait_tab = PortraitTab::Code;
                                    self.ui.portrait_tool_panel = None;
                                }
                                if ui.small_button("×").clicked() {
                                    close_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = close_idx {
                            let removed = self.ui.portrait_open_scripts.remove(idx);
                            if self.ui.portrait_active_script == Some(removed) {
                                self.ui.portrait_active_script = self.ui.portrait_open_scripts.last().copied();
                                if self.ui.portrait_active_script.is_none() {
                                    self.ui.portrait_tab = PortraitTab::Viewport;
                                }
                            }
                        }

                        if ui.small_button(" + ").on_hover_text("New Script").clicked() {
                            self.ui_action(ctx, "script");
                        }

                        if !self.plugin_bar.model.groups.is_empty() {
                            ui.menu_button("Plugins", |ui| {
                                self.plugins_menu_toolbars(ui);
                            });
                        }
                        if ui.selectable_label(crate::mcp_assistant::is_open(), "AI").clicked() {
                            crate::mcp_assistant::toggle_open();
                        }
                    });
                });
            });

        // 3. Flat Full-Width Bottom Tab Bar (`Workspace | Code | Explorer | Output | + Tools`) BEFORE CentralPanel
        let kb_h = crate::platform::keyboard_height_points(ctx);
        if kb_h <= 40.0 {
            let bar_h = 48.0 + safe_bottom;
            egui::Panel::bottom("portrait_flat_bottom_bar")
                .exact_size(bar_h)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 20, 27))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(36, 40, 52)))
                        .inner_margin(egui::Margin {
                            left: 6,
                            right: 6,
                            top: 4,
                            bottom: (4.0 + safe_bottom).round().clamp(4.0, 64.0) as i8,
                        }),
                )
                .show(ui, |ui| {
                    let item_w = ((ui.available_width() - 24.0) / 5.0).max(52.0);
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let is_ws = self.ui.portrait_tab == PortraitTab::Viewport
                            && !self.ui.portrait_explorer_drawer
                            && !self.ui.portrait_output_drawer
                            && self.ui.portrait_tool_panel.is_none();
                        let is_code = self.ui.portrait_tab == PortraitTab::Code;
                        let is_exp = self.ui.portrait_explorer_drawer;
                        let is_out = self.ui.portrait_output_drawer;

                        for (idx, icon_lbl, label, active) in [
                            (0u8, "🌐", "Workspace", is_ws),
                            (1, "</>", "Code", is_code),
                            (2, "☰", "Explorer", is_exp),
                            (3, "📟", "Output", is_out),
                        ] {
                            let bg = if active {
                                if idx == 0 {
                                    crate::wallpaper::RUST_ORANGE
                                } else {
                                    crate::wallpaper::STUDIO_BLUE
                                }
                            } else {
                                Color32::TRANSPARENT
                            };
                            let fg = if active { Color32::WHITE } else { theme::TEXT_DIM };
                            if ui
                                .add_sized(
                                    [item_w, 38.0],
                                    egui::Button::new(
                                        egui::RichText::new(format!("{icon_lbl}\n{label}"))
                                            .size(10.5)
                                            .color(fg)
                                            .strong(),
                                    )
                                    .fill(bg)
                                    .corner_radius(8.0),
                                )
                                .clicked()
                            {
                                match idx {
                                    0 => {
                                        self.ui.portrait_tab = PortraitTab::Viewport;
                                        self.ui.portrait_tool_panel = None;
                                        self.ui.portrait_explorer_drawer = false;
                                        self.ui.portrait_output_drawer = false;
                                    }
                                    1 => {
                                        self.ui.portrait_tab = PortraitTab::Code;
                                        self.ui.portrait_tool_panel = None;
                                    }
                                    2 => {
                                        self.ui.portrait_explorer_drawer = !self.ui.portrait_explorer_drawer;
                                    }
                                    3 => {
                                        self.ui.portrait_output_drawer = !self.ui.portrait_output_drawer;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Square #FF5A26 `+ Tools` button on far right
                        if ui
                            .add_sized(
                                [item_w, 38.0],
                                egui::Button::new(
                                    egui::RichText::new("+\nTools")
                                        .size(10.5)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(crate::wallpaper::RUST_ORANGE)
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.ui.portrait_quick_grid_open = !self.ui.portrait_quick_grid_open;
                        }
                    });
                });
        }

        // 4. Touch Luau Snippet Bar (above Bottom Pill Dock, or lifted above soft keyboard when typing in Code tab)
        if self.ui.portrait_tab == PortraitTab::Code && self.ui.portrait_tool_panel.is_none() {
            let kb_spacer = kb_h.max(0.0);
            if kb_spacer > 1.0 {
                egui::Panel::bottom("portrait_kb_spacer")
                    .exact_size(kb_spacer)
                    .frame(egui::Frame::NONE)
                    .show(ui, |_ui| {});
            }
            egui::Panel::bottom("portrait_luau_snippets")
                .frame(
                    egui::Frame::new()
                        .fill(theme::PANEL)
                        .inner_margin(egui::Margin::symmetric(6, 4)),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::horizontal().id_salt("luau_snippet_scroll").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
                            for (label, dir) in [("◀", "left"), ("▶", "right"), ("▲", "up"), ("▼", "down")] {
                                if ui.button(egui::RichText::new(label).monospace().size(14.0).strong()).clicked() {
                                    ctx.data_mut(|d| {
                                        d.insert_temp(egui::Id::new("mobile_cursor_move"), dir.to_string());
                                    });
                                }
                            }
                            let pad_resp = ui
                                .add(egui::Button::new("⇆ Drag Cursor").sense(egui::Sense::drag()))
                                .on_hover_text("Nhấn giữ và kéo ngang/dọc để di chuyển con trỏ");
                            if pad_resp.dragged() {
                                let d = pad_resp.drag_delta();
                                let acc_id = egui::Id::new("mobile_cursor_drag_acc");
                                let mut acc = ctx.data(|data| data.get_temp::<egui::Vec2>(acc_id).unwrap_or(egui::Vec2::ZERO));
                                acc += d;
                                let dir = if acc.x >= 12.0 {
                                    acc.x = 0.0;
                                    Some("right")
                                } else if acc.x <= -12.0 {
                                    acc.x = 0.0;
                                    Some("left")
                                } else if acc.y >= 18.0 {
                                    acc.y = 0.0;
                                    Some("down")
                                } else if acc.y <= -18.0 {
                                    acc.y = 0.0;
                                    Some("up")
                                } else {
                                    None
                                };
                                ctx.data_mut(|data| {
                                    data.insert_temp(acc_id, acc);
                                    if let Some(direction) = dir {
                                        data.insert_temp(egui::Id::new("mobile_cursor_move"), direction.to_string());
                                    }
                                });
                            }
                            if ui.small_button("Select Word").clicked() {
                                ctx.data_mut(|d| d.insert_temp(egui::Id::new("mobile_select_word"), true));
                            }
                            if ui.small_button("Dup Line").clicked() {
                                ctx.data_mut(|d| d.insert_temp(egui::Id::new("mobile_dup_line"), true));
                            }
                            ui.separator();
                            for (label, text) in [
                                ("Tab", "  "),
                                ("local", "local "),
                                ("function", "function "),
                                ("end", "end"),
                                ("if", "if "),
                                ("then", "then"),
                                ("else", "else"),
                                ("for", "for "),
                                ("return", "return "),
                                ("task.wait()", "task.wait()"),
                                ("print()", "print()"),
                                ("()", "()"),
                                ("{}", "{}"),
                                ("[]", "[]"),
                                (":", ":"),
                                (".", "."),
                                ("=", " = "),
                                ("~=", " ~= "),
                                ("..", " .. "),
                                ("--", "-- "),
                            ] {
                                if ui.button(egui::RichText::new(label).monospace().size(13.5)).clicked() {
                                    ctx.data_mut(|d| {
                                        d.insert_temp(egui::Id::new("mobile_luau_snippet"), text.to_string());
                                    });
                                }
                            }
                        });
                    });
                });
        }

        // 5b. Mobile DockWidget Manager (Moon Animator / Eclipse Animator / RigEdit / AutoScale)
        let active_plugin_slots = self.open_plugin_slots();
        let has_open_plugin = !active_plugin_slots.is_empty();
        if has_open_plugin {
            let dock_min_id = egui::Id::new("mobile_plugin_dock_minimized");
            let dock_sel_id = egui::Id::new("mobile_plugin_dock_active_idx");
            let dock_h_id = egui::Id::new("mobile_plugin_dock_height");
            let mut minimized = ctx.data(|d| d.get_temp::<bool>(dock_min_id).unwrap_or(false));
            let mut active_slot = ctx.data(|d| d.get_temp::<usize>(dock_sel_id).unwrap_or(active_plugin_slots[0].0));
            if !active_plugin_slots.iter().any(|(i, _)| *i == active_slot) {
                active_slot = active_plugin_slots[0].0;
            }
            let default_h = (screen_rect.height() * 0.38).clamp(180.0, 360.0);
            let dock_h = ctx.data(|d| d.get_temp::<f32>(dock_h_id).unwrap_or(default_h));

            let panel_h = if minimized { 32.0 } else { dock_h };
            egui::Panel::bottom("mobile_plugin_dock_panel")
                .exact_size(panel_h)
                .frame(
                    egui::Frame::new()
                        .fill(theme::PANEL_DARK)
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(65, 68, 78)))
                        .inner_margin(egui::Margin::symmetric(6, 4)),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (idx, title) in &active_plugin_slots {
                            if ui.selectable_label(active_slot == *idx, format!("🔌 {title}")).clicked() {
                                active_slot = *idx;
                                minimized = false;
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(if minimized { "▲" } else { "▼" })
                                .on_hover_text("Mở rộng / Thu gọn Plugin Dock")
                                .clicked()
                            {
                                minimized = !minimized;
                            }
                        });
                    });
                    if !minimized {
                        ui.separator();
                        self.plugin_widget_ui(ui, active_slot as u32);
                    }
                });
            ctx.data_mut(|d| {
                d.insert_temp(dock_min_id, minimized);
                d.insert_temp(dock_sel_id, active_slot);
            });
        }

        // 5. Output Panel (Supports Bottom Dock, Left/Right Snap, or Floating Window mode with luminous border trail)
        let out_mode_id = egui::Id::new("portrait_output_dock_mode");
        // 0 = Bottom Dock, 1 = Floating Window, 2 = Left Dock, 3 = Right Dock
        let mut out_mode = ctx.data(|d| d.get_temp::<u8>(out_mode_id).unwrap_or(0));
        if self.ui.portrait_output_drawer && out_mode == 0 {
            let drawer_h = (screen_rect.height() * 0.36).clamp(150.0, 360.0);
            egui::Panel::bottom("portrait_output_drawer")
                .exact_size(drawer_h)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(22, 24, 32, 242))
                        .stroke(egui::Stroke::new(1.0, crate::wallpaper::PASTEL_SKY.gamma_multiply(0.35)))
                        .inner_margin(egui::Margin::symmetric(8, 5)),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📟 Output")
                                .strong()
                                .size(12.5)
                                .color(crate::wallpaper::PASTEL_SKY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").clicked() {
                                self.ui.portrait_output_drawer = false;
                            }
                            if ui
                                .small_button("❐ Float")
                                .on_hover_text("Chuyển sang cửa sổ nổi (Floating Window)")
                                .clicked()
                            {
                                out_mode = 1;
                                ctx.data_mut(|d| d.insert_temp(out_mode_id, out_mode));
                            }
                        });
                    });
                    ui.separator();
                    self.output_ui(ui);
                });
        }

        // 6. Central Full-Screen Tab (Workspace 3D Viewport, Luau Code Editor, or Tool Panel)
        let mut content_rect = screen_rect;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG))
            .show(ui, |ui| {
                content_rect = ui.max_rect();
                if self.ui.portrait_tab == PortraitTab::Code || self.ui.portrait_tool_panel.is_some() {
                    ui.painter().rect_filled(content_rect, 0.0, crate::wallpaper::OBSIDIAN_CANVAS);
                }
                if let Some(tool_tab) = self.ui.portrait_tool_panel {
                    ui.horizontal(|ui| {
                        if ui.button("← Back").clicked() {
                            self.ui.portrait_tool_panel = None;
                        }
                        ui.label(egui::RichText::new(tool_tab.title()).strong());
                    });
                    ui.separator();
                    match tool_tab {
                        crate::app::Tab::Toolbox => self.toolbox_ui(ui),
                        crate::app::Tab::TerrainEditor => self.terrain_editor_ui(ui),
                        _ => {}
                    }
                    return;
                }

                match self.ui.portrait_tab {
                    PortraitTab::Viewport => {
                        self.viewport_ui(ui);
                    }
                    PortraitTab::Code => {
                        if self.ui.portrait_open_scripts.is_empty() {
                            if let Some(first_ref) = self
                                .game
                                .doc
                                .dom
                                .descendants_of(self.game.doc.dom.root_ref())
                                .find(|i| matches!(i.class.as_str(), "Script" | "LocalScript" | "ModuleScript"))
                                .map(|i| i.referent())
                            {
                                self.ui.portrait_open_scripts.push(first_ref);
                                self.ui.portrait_active_script = Some(first_ref);
                            }
                        }
                        if self.ui.portrait_active_script.is_none() {
                            self.ui.portrait_active_script = self.ui.portrait_open_scripts.first().copied();
                        }

                        if let Some(r) = self.ui.portrait_active_script {
                            self.script_tab_ui(ui, r);
                        } else {
                            ui.centered_and_justified(|ui| {
                                if ui.button("📜 + Tạo Script Luau mới").clicked() {
                                    self.ui_action(ctx, "script");
                                }
                            });
                        }
                    }
                }
            });

        // 5b. Floating / Snap-Docked Output Window when out_mode != 0
        if self.ui.portrait_output_drawer && out_mode != 0 {
            let win_w = (screen_rect.width() * 0.84).clamp(240.0, 440.0);
            let win_h = (content_rect.height() * 0.42).clamp(170.0, 340.0);
            let mut win = egui::Window::new("📟 Output")
                .id(egui::Id::new("portrait_floating_output_win"))
                .title_bar(false)
                .resizable(true)
                .default_size(Vec2::new(win_w, win_h))
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(Color32::from_rgba_unmultiplied(20, 22, 30, 238))
                        .stroke(egui::Stroke::new(1.0, crate::wallpaper::PASTEL_SKY.gamma_multiply(0.5)))
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(8)),
                );
            if out_mode == 2 {
                win = win.current_pos(egui::pos2(screen_rect.left() + 6.0, content_rect.top() + 8.0));
            } else if out_mode == 3 {
                win = win.current_pos(egui::pos2((screen_rect.right() - win_w - 6.0).max(6.0), content_rect.top() + 8.0));
            }
            if let Some(shown) = win.show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("📟 Output (Floating / Snap)")
                            .strong()
                            .size(12.5)
                            .color(crate::wallpaper::PASTEL_SKY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            self.ui.portrait_output_drawer = false;
                        }
                        if ui.selectable_label(out_mode == 0, "⬇ Bot").on_hover_text("Snap Bottom").clicked() {
                            out_mode = 0;
                            ctx.data_mut(|d| d.insert_temp(out_mode_id, out_mode));
                        }
                        if ui.selectable_label(out_mode == 3, "➡ R").on_hover_text("Snap Right").clicked() {
                            out_mode = 3;
                            ctx.data_mut(|d| d.insert_temp(out_mode_id, out_mode));
                        }
                        if ui.selectable_label(out_mode == 2, "⬅ L").on_hover_text("Snap Left").clicked() {
                            out_mode = 2;
                            ctx.data_mut(|d| d.insert_temp(out_mode_id, out_mode));
                        }
                        if ui.selectable_label(out_mode == 1, "❐ Float").on_hover_text("Free Float").clicked() {
                            out_mode = 1;
                            ctx.data_mut(|d| d.insert_temp(out_mode_id, out_mode));
                        }
                    });
                });
                ui.separator();
                self.output_ui(ui);
            }) {
                ctx.layer_painter(shown.response.layer_id).rect_stroke(
                    shown.response.rect,
                    egui::CornerRadius::same(14),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
            }
        }

        // 7. Quick Tools & Plugins Grid Popup (from bottom `+` button) with Luminous Border Trail
        if self.ui.portrait_quick_grid_open {
            let mut open = self.ui.portrait_quick_grid_open;
            let shown = egui::Window::new("⚡ Quick Tools & Plugins")
                .id(egui::Id::new("portrait_quick_grid"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -(68.0 + safe_bottom)))
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(Color32::from_rgba_unmultiplied(20, 22, 30, 242))
                        .corner_radius(egui::CornerRadius::same(16))
                        .inner_margin(egui::Margin::same(12)),
                )
                .show(ctx, |ui| {
                    let btn_sz = egui::vec2(108.0, 42.0);
                    egui::Grid::new("portrait_tools_grid").spacing([8.0, 8.0]).show(ui, |ui| {
                        if ui.add_sized(btn_sz, egui::Button::new("🔌 Plugins")).clicked() {
                            self.ui_action(ctx, "manage_plugins");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("🧑 Character")).clicked() {
                            self.ui_action(ctx, "character");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("🧰 Toolbox")).clicked() {
                            self.ui.portrait_tool_panel = Some(crate::app::Tab::Toolbox);
                            self.ui.portrait_quick_grid_open = false;
                        }
                        ui.end_row();
                        if ui.add_sized(btn_sz, egui::Button::new("⛰ Terrain")).clicked() {
                            self.ui.portrait_tool_panel = Some(crate::app::Tab::TerrainEditor);
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("➕ Insert Obj")).clicked() {
                            self.ui_action(ctx, "insert_object");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("📜 New Script")).clicked() {
                            self.ui_action(ctx, "script");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        ui.end_row();
                        if ui.add_sized(btn_sz, egui::Button::new("🎨 Wallpapers")).clicked() {
                            crate::wallpaper::toggle_modal(ctx);
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("⚙ Settings")).clicked() {
                            self.ui_action(ctx, "studio_settings");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("🤖 AI (MCP)")).clicked() {
                            crate::mcp_assistant::toggle_open();
                            self.ui.portrait_quick_grid_open = false;
                        }
                        ui.end_row();
                    });
                });
            if let Some(r) = shown {
                ctx.layer_painter(r.response.layer_id).rect_stroke(
                    r.response.rect,
                    egui::CornerRadius::same(16),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
            }
            if !open {
                self.ui.portrait_quick_grid_open = false;
            }
        }

        // 8. Non-blocking Floating / Dockable Explorer & Properties Tool Panel with Snap Zones (Left, Right, Bottom, Float)
        if self.ui.portrait_explorer_drawer {
            let width_id = egui::Id::new("portrait_explorer_width");
            let max_w = (screen_rect.width() - 20.0).max(200.0);
            let default_w = (screen_rect.width() * 0.46).clamp(195.0, 340.0);
            let mut drawer_w = ctx.data(|d| d.get_temp::<f32>(width_id).unwrap_or(default_w)).clamp(165.0, max_w);
            let popup_h = (content_rect.height() - 12.0).clamp(240.0, 680.0);
            // 0 = Float, 1 = Snap Left, 2 = Snap Right (default), 3 = Snap Bottom
            let snap_id = egui::Id::new("portrait_explorer_snap_zone");
            let mut snap_zone = ctx.data(|d| d.get_temp::<u8>(snap_id).unwrap_or(2));

            let (win_size, pinned_pos) = match snap_zone {
                1 => (
                    egui::vec2(drawer_w, popup_h),
                    egui::pos2(screen_rect.left() + 4.0, content_rect.top() + 4.0),
                ),
                3 => {
                    let bh = (content_rect.height() * 0.46).clamp(200.0, 360.0);
                    (
                        egui::vec2((screen_rect.width() - 12.0).max(220.0), bh),
                        egui::pos2(screen_rect.left() + 6.0, (content_rect.bottom() - bh - 4.0).max(content_rect.top() + 4.0)),
                    )
                }
                _ => (
                    egui::vec2(drawer_w, popup_h),
                    egui::pos2(screen_rect.right() - drawer_w - 4.0, content_rect.top() + 4.0),
                ),
            };

            let mut win = egui::Window::new("Explorer & Properties")
                .id(egui::Id::new("portrait_floating_explorer"))
                .title_bar(false)
                .resizable(false)
                .movable(snap_zone == 0)
                .fixed_size(win_size)
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(Color32::from_rgba_unmultiplied(22, 24, 32, 238))
                        .stroke(egui::Stroke::new(1.0, crate::wallpaper::PASTEL_MINT.gamma_multiply(0.45)))
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::symmetric(6, 6)),
                );
            if snap_zone != 0 {
                win = win.current_pos(pinned_pos);
            } else {
                win = win.default_pos(pinned_pos);
            }
            if let Some(shown) = win.show(ctx, |ui| {
                ui.set_max_width(win_size.x - 12.0);
                ui.horizontal(|ui| {
                    // Left dark vertical resize bar with 5 grip dots
                    let handle_w = 12.0;
                    let (handle_rect, handle_resp) = ui.allocate_exact_size(
                        egui::vec2(handle_w, (win_size.y - 14.0).max(140.0)),
                        egui::Sense::drag(),
                    );
                    if handle_resp.hovered() || handle_resp.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if handle_resp.dragged() {
                        let sign = if snap_zone == 1 { 1.0 } else { -1.0 };
                        drawer_w = (drawer_w + sign * handle_resp.drag_delta().x).clamp(165.0, max_w);
                        ctx.data_mut(|d| d.insert_temp(width_id, drawer_w));
                    }
                    let bar_col = if handle_resp.dragged() {
                        crate::wallpaper::PASTEL_MINT.gamma_multiply(0.85)
                    } else if handle_resp.hovered() {
                        Color32::from_rgb(28, 31, 38)
                    } else {
                        Color32::from_rgb(15, 16, 19)
                    };
                    ui.painter().rect_filled(handle_rect, 3.0, bar_col);
                    let dot_col = if handle_resp.dragged() || handle_resp.hovered() {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(145, 150, 162)
                    };
                    for i in -2..=2 {
                        ui.painter().circle_filled(
                            egui::pos2(handle_rect.center().x, handle_rect.center().y + i as f32 * 5.0),
                            1.3,
                            dot_col,
                        );
                    }

                    ui.vertical(|ui| {
                        let inner_w = (win_size.x - handle_w - 22.0).max(120.0);
                        ui.set_width(inner_w);
                        ui.set_max_width(inner_w);

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("🌳 Explorer")
                                    .strong()
                                    .size(12.5)
                                    .color(crate::wallpaper::PASTEL_MINT),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").on_hover_text("Đóng").clicked() {
                                    self.ui.portrait_explorer_drawer = false;
                                }
                                if ui.selectable_label(snap_zone == 3, "⬇").on_hover_text("Dock Bottom").clicked() {
                                    snap_zone = 3;
                                    ctx.data_mut(|d| d.insert_temp(snap_id, snap_zone));
                                }
                                if ui.selectable_label(snap_zone == 2, "➡").on_hover_text("Dock Right").clicked() {
                                    snap_zone = 2;
                                    ctx.data_mut(|d| d.insert_temp(snap_id, snap_zone));
                                }
                                if ui.selectable_label(snap_zone == 1, "⬅").on_hover_text("Dock Left").clicked() {
                                    snap_zone = 1;
                                    ctx.data_mut(|d| d.insert_temp(snap_id, snap_zone));
                                }
                                if ui.selectable_label(snap_zone == 0, "❐").on_hover_text("Floating Window").clicked() {
                                    snap_zone = 0;
                                    ctx.data_mut(|d| d.insert_temp(snap_id, snap_zone));
                                }
                            });
                        });
                        ui.separator();

                        let total_h = (ui.available_height() - 18.0).max(150.0);
                        let split = self.ui.portrait_drawer_split.clamp(0.2, 0.8);
                        let top_h = (total_h * split).max(75.0);
                        let bot_h = (total_h - top_h).max(75.0);

                        ui.allocate_ui_with_layout(
                            egui::vec2(inner_w, top_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_max_width(inner_w);
                                self.explorer_ui(ui);
                            },
                        );

                        let (split_rect, split_resp) = ui.allocate_exact_size(
                            egui::vec2(inner_w, 12.0),
                            egui::Sense::drag(),
                        );
                        if split_resp.dragged() {
                            let delta = split_resp.drag_delta().y / total_h;
                            self.ui.portrait_drawer_split = (self.ui.portrait_drawer_split + delta).clamp(0.2, 0.8);
                        }
                        ui.painter().rect_filled(
                            egui::Rect::from_center_size(split_rect.center(), egui::vec2(44.0, 3.0)),
                            2.0,
                            crate::wallpaper::PASTEL_MINT.gamma_multiply(0.6),
                        );

                        ui.allocate_ui_with_layout(
                            egui::vec2(inner_w, bot_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_max_width(inner_w);
                                ui.label(
                                    egui::RichText::new("⚙ Properties")
                                        .strong()
                                        .size(12.0)
                                        .color(crate::wallpaper::PASTEL_LAVENDER),
                                );
                                self.properties_ui(ui);
                            },
                        );
                    });
                });
            }) {
                ctx.layer_painter(shown.response.layer_id).rect_stroke(
                    shown.response.rect,
                    egui::CornerRadius::same(14),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
            }
        } else {
            egui::Area::new(egui::Id::new("portrait_drawer_handle"))
                .anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-2.0, 0.0))
                .show(ctx, |ui| {
                    if ui
                        .add_sized([26.0, 46.0], egui::Button::new("◀").corner_radius(10.0))
                        .on_hover_text("Mở Explorer & Properties")
                        .clicked()
                    {
                        self.ui.portrait_explorer_drawer = true;
                    }
                });
        }

        Self::hardware_perf_popup_ui(ctx);
        crate::mcp_assistant::ui(self, ctx);
        crate::touch_input_bridge::floating_shortcut_pill_ui(ctx, has_open_plugin);
    }

    /// Landscape Mobile & Tablet Studio layout (`width > height` on iOS/Android):
    /// 1. Single-Row Compact Top Ribbon Bar (`height: 38pt`) with `File`, `Play`, `Select`,
    ///    `Move`, `Scale`, `Rotate`, `Part`, Category Pill Dropdown `[ <Icon> UI ▾ ]`,
    ///    and `Settings` / `Sign In`, padded by `safe_left` and `safe_right`.
    /// 2. Split-Pane Landscape Workspace:
    ///    - **Left Main 3D Viewport (~74% width)**: full-height 3D scene with Dynamic Thumbstick
    ///      (bottom-left), 4-button Camera Control Cluster (bottom-right of viewport pane),
    ///      and Collapsible Dark-Translucent Bottom Pill Dock (`Workspace | Code | Explorer | Output | +`).
    ///    - **Right Collapsible Stacked Inspector (~26% width, min 210pt, max 280pt)**:
    ///      Top half `Explorer` (with quick `+` add button) + Bottom half `Properties`
    ///      (preserving `is_renaming_property` focus lock), with an animated Slide-Out / Collapse Handle.
    pub fn landscape_mobile_layout_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        screen_rect: egui::Rect,
    ) {
        use crate::studio_ui::PortraitTab;

        if std::mem::take(&mut self.switch_to_viewport) {
            self.ui.portrait_tab = PortraitTab::Viewport;
            self.ui.portrait_tool_panel = None;
            self.ui.portrait_output_drawer = false;
            self.ui.portrait_quick_grid_open = false;
        }

        // Sync any scripts opened from Explorer
        for r in std::mem::take(&mut self.open_script) {
            if !self.ui.portrait_open_scripts.contains(&r) {
                self.ui.portrait_open_scripts.push(r);
            }
            self.ui.portrait_active_script = Some(r);
            self.ui.portrait_tab = PortraitTab::Code;
            self.ui.portrait_tool_panel = None;
        }
        self.ui
            .portrait_open_scripts
            .retain(|r| self.game.doc.dom.get_by_ref(*r).is_some());

        let inspector_open_id = egui::Id::new("landscape_inspector_open");
        // Expanded by default on iPads (width >= 900pt) and also expanded on phones unless toggled
        let default_insp_open = true;
        let mut inspector_open = ctx
            .data(|d| d.get_temp::<bool>(inspector_open_id))
            .unwrap_or(default_insp_open);

        for (tab, want) in std::mem::take(&mut self.ui.panel_requests) {
            match tab {
                crate::app::Tab::Explorer | crate::app::Tab::Properties => {
                    inspector_open = want.unwrap_or(!inspector_open);
                    ctx.data_mut(|d| d.insert_temp(inspector_open_id, inspector_open));
                }
                crate::app::Tab::Output => {
                    self.ui.portrait_output_drawer = want.unwrap_or(!self.ui.portrait_output_drawer);
                }
                crate::app::Tab::Toolbox | crate::app::Tab::TerrainEditor => {
                    let show = want.unwrap_or(self.ui.portrait_tool_panel != Some(tab));
                    self.ui.portrait_tool_panel = if show { Some(tab) } else { None };
                }
                _ => {}
            }
        }
        self.ui.portrait_explorer_drawer = inspector_open;

        let (safe_top, safe_bottom, safe_left, safe_right) = crate::platform::get_safe_area_insets();
        let side_left = (8.0 + safe_left).round().clamp(6.0, 120.0) as i8;
        let side_right = (8.0 + safe_right).round().clamp(6.0, 120.0) as i8;
        let top_pad = (4.0 + safe_top * 0.35).round().clamp(3.0, 48.0) as i8;

        // 1. Single-Row Compact Top Ribbon Bar (`height: 38pt`)
        let mut act: Option<String> = None;
        egui::Panel::top("landscape_single_top_bar")
            .exact_size(38.0 + top_pad as f32)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(18, 20, 27, 246))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 42, 54)))
                    .inner_margin(egui::Margin {
                        left: side_left,
                        right: side_right,
                        top: top_pad,
                        bottom: 3,
                    }),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                    // File menu pill (Stitch Rust-Orange #FF5A26)
                    let file_btn = egui::Button::new(
                        egui::RichText::new("File")
                            .size(12.5)
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(crate::wallpaper::RUST_ORANGE)
                    .corner_radius(6.0)
                    .min_size(Vec2::new(42.0, 26.0));
                    let file_resp = ui.add(file_btn);
                    egui::Popup::menu(&file_resp).show(|ui| {
                        ui.label(egui::RichText::new("File").strong().small().color(theme::TEXT_DIM));
                        if ui.button("New Place").clicked() { self.ui_action(ctx, "new"); ui.close(); }
                        if ui.button("Open .rbxl...").clicked() { self.ui_action(ctx, "open"); ui.close(); }
                        if ui.button("Open .project.json...").clicked() { self.ui_action(ctx, "open_folder"); ui.close(); }
                        if ui.button("Open from Roblox Cloud...").clicked() { self.ui_action(ctx, "open_roblox"); ui.close(); }
                        if ui.button("Save").clicked() { self.ui_action(ctx, "save"); ui.close(); }
                        if ui.button("Save to Roblox As...").clicked() { self.ui_action(ctx, "save_roblox_as"); ui.close(); }
                        if ui.button("Publish to Roblox").clicked() { self.ui_action(ctx, "save_roblox"); ui.close(); }
                        if ui
                            .button(egui::RichText::new("Close Place (Home)").color(Color32::from_rgb(255, 115, 115)))
                            .clicked()
                        {
                            self.ui_action(ctx, "close_place");
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("🎨 Themes & Wallpapers...").clicked() { crate::wallpaper::toggle_modal(ctx); ui.close(); }
                        if ui.button("Studio Settings...").clicked() { self.ui_action(ctx, "studio_settings"); ui.close(); }
                        if ui.button("Manage Plugins...").clicked() { self.ui_action(ctx, "manage_plugins"); ui.close(); }
                        if ui.button("AI Assistant (MCP)").clicked() { crate::mcp_assistant::toggle_open(); ui.close(); }
                    });

                    // Always-visible `3D Viewport` / `Workspace` pill button so user can return to pure 3D view anytime
                    let is_pure_3d = self.ui.portrait_tab == PortraitTab::Viewport
                        && self.ui.portrait_tool_panel.is_none();
                    let ws_btn = egui::Button::new(
                        egui::RichText::new("🌐 3D Viewport")
                            .size(12.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(if is_pure_3d {
                        Color32::from_rgb(33, 112, 228)
                    } else {
                        Color32::from_rgba_unmultiplied(32, 36, 48, 220)
                    })
                    .corner_radius(13.0)
                    .min_size(Vec2::new(88.0, 28.0));
                    if ui.add(ws_btn).on_hover_text("Return to pure 3D Viewport").clicked() {
                        self.ui.portrait_tab = PortraitTab::Viewport;
                        self.ui.portrait_tool_panel = None;
                        self.ui.portrait_quick_grid_open = false;
                    }

                    // Interactive Snap Grid Pill (`Off`, `0.25`, `1.0` studs)
                    let snap_lbl = if self.game.snap_move {
                        format!("Snap: {:.2} ▾", self.game.move_inc)
                    } else {
                        "Snap: Off ▾".to_string()
                    };
                    let snap_resp = ui.add(
                        egui::Button::new(egui::RichText::new(snap_lbl).size(11.5).color(Color32::WHITE))
                            .fill(Color32::from_rgb(30, 34, 46))
                            .corner_radius(12.0)
                            .min_size(Vec2::new(82.0, 26.0)),
                    );
                    egui::Popup::menu(&snap_resp).show(|ui| {
                        if ui.selectable_label(!self.game.snap_move, "Snap: Off").clicked() {
                            self.game.snap_move = false;
                            ui.close();
                        }
                        if ui.selectable_label(self.game.snap_move && (self.game.move_inc - 0.25).abs() < 0.01, "Snap: 0.25 studs").clicked() {
                            self.game.snap_move = true;
                            self.game.move_inc = 0.25;
                            ui.close();
                        }
                        if ui.selectable_label(self.game.snap_move && (self.game.move_inc - 1.0).abs() < 0.01, "Snap: 1.00 studs").clicked() {
                            self.game.snap_move = true;
                            self.game.move_inc = 1.0;
                            ui.close();
                        }
                    });

                    // Focus Code IDE pill (enters pure Code IDE without 3D rendering)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("</> Code IDE")
                                    .size(11.5)
                                    .strong()
                                    .color(crate::wallpaper::LOGO_CYAN),
                            )
                            .fill(Color32::from_rgba_unmultiplied(28, 34, 46, 220))
                            .stroke(egui::Stroke::new(1.0, crate::wallpaper::LOGO_CYAN.gamma_multiply(0.45)))
                            .corner_radius(13.0)
                            .min_size(Vec2::new(78.0, 26.0)),
                        )
                        .on_hover_text("Switch to pure Focus Code IDE (hides 3D)")
                        .clicked()
                    {
                        self.ui.focus_code_mode = true;
                        self.ui.viewport_hidden = true;
                    }

                    // Play / Stop compact button
                    let playing = self.game.play.is_some();
                    if playing {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("■ Stop")
                                        .size(12.5)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(Color32::from_rgb(185, 50, 50))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(56.0, 26.0)),
                            )
                            .clicked()
                        {
                            self.stop_play();
                        }
                    } else if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("▶ Play")
                                    .size(12.5)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(38, 142, 74))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(56.0, 26.0)),
                        )
                        .clicked()
                    {
                        self.start_run(self.ui.run_type);
                    }

                    // Core Transform & Insert tools inline: Select, Move, Scale, Rotate, Part
                    for (icon, label, tool) in [
                        ("pointer", "Select", Tool::Select),
                        ("arrows-move", "Move", Tool::Move),
                        ("resize", "Scale", Tool::Scale),
                        ("rotate-clockwise-2", "Rotate", Tool::Rotate),
                    ] {
                        let active = self.tool == tool;
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(52.0, 28.0), Sense::click());
                        let fill = if active {
                            Color32::from_rgb(33, 112, 228)
                        } else if resp.hovered() {
                            theme::HOVER_BG
                        } else {
                            Color32::from_rgba_unmultiplied(28, 31, 40, 200)
                        };
                        ui.painter().rect_filled(r, egui::CornerRadius::same(6), fill);
                        let ic_r = Rect::from_center_size(egui::pos2(r.left() + 11.0, r.center().y), Vec2::splat(13.0));
                        rustudio_engine::ti::paint(
                            ui.painter(),
                            ic_r,
                            icon,
                            if active { Color32::WHITE } else { crate::wallpaper::PASTEL_LAVENDER },
                        );
                        ui.painter().text(
                            egui::pos2(r.left() + 20.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            Color32::WHITE,
                        );
                        self.ui_rects.insert(tool_key(tool), r);
                        if resp.clicked() {
                            self.tool = tool;
                        }
                    }

                    // Quick `+ Part` button
                    let (part_r, part_resp) = ui.allocate_exact_size(Vec2::new(48.0, 28.0), Sense::click());
                    ui.painter().rect_filled(
                        part_r,
                        egui::CornerRadius::same(6),
                        if part_resp.hovered() {
                            theme::HOVER_BG
                        } else {
                            Color32::from_rgba_unmultiplied(28, 31, 40, 200)
                        },
                    );
                    let p_ic = Rect::from_center_size(egui::pos2(part_r.left() + 11.0, part_r.center().y), Vec2::splat(13.0));
                    rustudio_engine::ti::paint(ui.painter(), p_ic, "cube", crate::wallpaper::PASTEL_MINT);
                    ui.painter().text(
                        egui::pos2(part_r.left() + 20.0, part_r.center().y),
                        egui::Align2::LEFT_CENTER,
                        "Part",
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    if part_resp.on_hover_text("Insert Part").clicked() {
                        act = Some("part_block".to_string());
                    }

                    // Pill Category Dropdown Box (`[ <Icon> UI ▾ ]` / `[ <Icon> Script ▾ ]`)
                    let cat_pop_id = egui::Id::new("portrait_cat_dropdown_open");
                    let mut cat_open = ctx.data(|d| d.get_temp::<bool>(cat_pop_id).unwrap_or(false));
                    let cat_icon = match self.ui.tab {
                        RibbonTab::Home => "pointer",
                        RibbonTab::Model => "cube",
                        RibbonTab::Avatar => "user-cog",
                        RibbonTab::Ui => "app-window",
                        RibbonTab::Script => "file-code",
                        RibbonTab::Plugins => "sparkles",
                    };
                    let cat_accent = match self.ui.tab {
                        RibbonTab::Home => crate::wallpaper::PASTEL_LAVENDER,
                        RibbonTab::Model => crate::wallpaper::PASTEL_MINT,
                        RibbonTab::Avatar => crate::wallpaper::PASTEL_GOLD,
                        RibbonTab::Ui => crate::wallpaper::PASTEL_ROSE,
                        RibbonTab::Script => crate::wallpaper::PASTEL_SKY,
                        RibbonTab::Plugins => crate::wallpaper::PASTEL_PEACH,
                    };
                    let pill_label = format!("{} ▾", self.ui.tab.label());
                    let font = egui::FontId::proportional(12.0);
                    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(pill_label.clone(), font.clone(), Color32::WHITE).size().x);
                    let pill_sz = egui::vec2((text_w + 38.0).max(84.0), 26.0);
                    let (pill_rect, pill_resp) = ui.allocate_exact_size(pill_sz, Sense::click());
                    ui.painter().rect_filled(
                        pill_rect,
                        egui::CornerRadius::same(13),
                        if cat_open {
                            Color32::from_rgba_unmultiplied(40, 44, 58, 240)
                        } else {
                            Color32::from_rgba_unmultiplied(26, 29, 38, 225)
                        },
                    );
                    ui.painter().rect_stroke(
                        pill_rect,
                        egui::CornerRadius::same(13),
                        egui::Stroke::new(1.0, cat_accent.gamma_multiply(0.7)),
                        egui::StrokeKind::Inside,
                    );
                    rustudio_engine::ti::paint(
                        ui.painter(),
                        Rect::from_center_size(egui::pos2(pill_rect.left() + 13.0, pill_rect.center().y), Vec2::splat(13.0)),
                        cat_icon,
                        cat_accent,
                    );
                    ui.painter().text(
                        egui::pos2(pill_rect.left() + 24.0, pill_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &pill_label,
                        font,
                        Color32::WHITE,
                    );
                    if pill_resp.clicked() {
                        cat_open = !cat_open;
                        ctx.data_mut(|d| d.insert_temp(cat_pop_id, cat_open));
                    }
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("portrait_cat_pill_rect"), pill_rect));

                    // Right side controls: Sign In, Settings/Wallpaper, Perf, Undo, Redo, Inspector Toggle
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let signed = self.game.account.signed_in();
                        let acc_label = if signed {
                            self.game.account.status_short().trim_start_matches("Roblox: ").to_string()
                        } else {
                            "Sign In".to_string()
                        };
                        let acc_resp = ui.add(
                            egui::Button::new(
                                egui::RichText::new(acc_label)
                                    .size(11.5)
                                    .strong()
                                    .color(if signed { Color32::WHITE } else { Color32::BLACK }),
                            )
                            .fill(if signed {
                                Color32::from_rgb(38, 60, 50)
                            } else {
                                crate::wallpaper::PASTEL_LAVENDER
                            })
                            .corner_radius(12.0),
                        );
                        self.ui_rects.insert("account", acc_resp.rect);
                        if acc_resp.clicked() {
                            self.game.account.open = !self.game.account.open;
                            self.game.account.adding = false;
                        }

                        if ui.small_button("⚙").on_hover_text("Studio Settings").clicked() {
                            self.ui_action(ctx, "studio_settings");
                        }
                        if ui.small_button("🎨").on_hover_text("Themes & Wallpapers").clicked() {
                            crate::wallpaper::toggle_modal(ctx);
                        }
                        if ui.small_button("↷").on_hover_text("Redo").clicked() {
                            self.ui_action(ctx, "redo");
                        }
                        if ui.small_button("↶").on_hover_text("Undo").clicked() {
                            self.ui_action(ctx, "undo");
                        }
                    });
                });
            });

        if let Some(a) = act {
            self.ui_action(ctx, &a);
        }

        // Category Dropdown Popover with Animated Luminous Border Trail
        {
            let cat_pop_id = egui::Id::new("portrait_cat_dropdown_open");
            let cat_open = ctx.data(|d| d.get_temp::<bool>(cat_pop_id).unwrap_or(false));
            let anim = ctx.animate_bool_with_time(egui::Id::new("landscape_cat_pop_anim"), cat_open, 0.16);
            if cat_open || anim > 0.01 {
                let anchor_rect = ctx
                    .data(|d| d.get_temp::<Rect>(egui::Id::new("portrait_cat_pill_rect")))
                    .unwrap_or_else(|| Rect::from_min_size(egui::pos2(240.0, 40.0), Vec2::new(86.0, 26.0)));
                let pop_pos = egui::pos2(anchor_rect.left(), anchor_rect.bottom() + 6.0 * anim);
                let shown = egui::Area::new(egui::Id::new("landscape_cat_dropdown_area"))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .fixed_pos(pop_pos)
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(20, 22, 30, (242.0 * anim) as u8))
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(220.0);
                                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                ui.label(
                                    egui::RichText::new("CATEGORY & TOOLS")
                                        .size(10.5)
                                        .strong()
                                        .color(theme::TEXT_DIM),
                                );
                                for (tab, icon, accent) in [
                                    (RibbonTab::Home, "pointer", crate::wallpaper::PASTEL_LAVENDER),
                                    (RibbonTab::Model, "cube", crate::wallpaper::PASTEL_MINT),
                                    (RibbonTab::Avatar, "user-cog", crate::wallpaper::PASTEL_GOLD),
                                    (RibbonTab::Ui, "app-window", crate::wallpaper::PASTEL_ROSE),
                                    (RibbonTab::Script, "file-code", crate::wallpaper::PASTEL_SKY),
                                    (RibbonTab::Plugins, "sparkles", crate::wallpaper::PASTEL_PEACH),
                                ] {
                                    let selected = self.ui.tab == tab;
                                    if ui
                                        .add_sized(
                                            [200.0, 28.0],
                                            egui::Button::new(
                                                egui::RichText::new(format!("{} Category", tab.label()))
                                                    .size(12.5)
                                                    .color(if selected { Color32::BLACK } else { Color32::WHITE })
                                                    .strong(),
                                            )
                                            .fill(if selected {
                                                accent
                                            } else {
                                                Color32::from_rgba_unmultiplied(32, 36, 48, 200)
                                            })
                                            .corner_radius(8.0),
                                        )
                                        .clicked()
                                    {
                                        self.ui.tab = tab;
                                    }
                                    let _ = icon;
                                }
                                ui.separator();
                                egui::ScrollArea::horizontal().id_salt("landscape_cat_ribbon_tools").show(ui, |ui| {
                                    self.studio_ribbon(ui);
                                });
                            })
                    });
                let pop_rect = shown.inner.response.rect;
                ctx.layer_painter(shown.response.layer_id).rect_stroke(
                    pop_rect,
                    egui::CornerRadius::same(14),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
                let clicked_out = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| {
                            !pop_rect.expand(8.0).contains(p) && !anchor_rect.expand(4.0).contains(p)
                        })
                });
                if clicked_out {
                    ctx.data_mut(|d| d.insert_temp(cat_pop_id, false));
                }
            }
        }

        // 2. Right Collapsible Stacked Inspector (~26% width, min 210pt, max 280pt)
        //    with animated Slide-Out / Collapse Handle
        let insp_anim = ctx.animate_bool_with_time(
            egui::Id::new("landscape_inspector_slide_anim"),
            inspector_open,
            0.18,
        );
        let target_insp_w = (screen_rect.width() * 0.26).clamp(210.0, 280.0);
        if insp_anim > 0.01 {
            let cur_w = (target_insp_w * insp_anim).max(24.0);
            let right_pad = (8.0 + safe_right).round().clamp(8.0, 120.0) as i8;
            egui::Panel::right("landscape_right_inspector")
                .exact_size(cur_w)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(20, 22, 29, 246))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(42, 46, 58)))
                        .inner_margin(egui::Margin {
                            left: 6,
                            right: right_pad,
                            top: 6,
                            bottom: (6.0 + safe_bottom * 0.5).round() as i8,
                        }),
                )
                .show(ui, |ui| {
                    ui.set_clip_rect(ui.max_rect());
                    let inner_w = (ui.available_width() - 6.0).max(140.0);
                    let total_h = (ui.available_height() - 16.0).max(140.0);
                    let split = self.ui.portrait_drawer_split.clamp(0.25, 0.75);
                    let top_h = (total_h * split).max(70.0);
                    let bot_h = (total_h - top_h - 10.0).max(70.0);

                    // Top Half: Explorer (with quick `+` add button and collapse handle `▶`)
                    ui.allocate_ui_with_layout(
                        egui::vec2(inner_w, top_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_width(inner_w);
                            ui.horizontal(|ui| {
                                if ui
                                    .add_sized(
                                        [28.0, 24.0],
                                        egui::Button::new(egui::RichText::new("✕").strong())
                                            .fill(Color32::from_rgba_unmultiplied(45, 48, 60, 200))
                                            .corner_radius(6.0),
                                    )
                                    .on_hover_text("Close / Slide out Inspector")
                                    .clicked()
                                {
                                    inspector_open = false;
                                    ctx.data_mut(|d| d.insert_temp(inspector_open_id, false));
                                }
                                ui.label(
                                    egui::RichText::new("Explorer")
                                        .strong()
                                        .size(12.5)
                                        .color(crate::wallpaper::PASTEL_MINT),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .small_button("➕")
                                        .on_hover_text("Quick Insert Object into Selection / Workspace")
                                        .clicked()
                                    {
                                        self.ui_action(ctx, "insert_object");
                                    }
                                });
                            });
                            ui.separator();
                            self.explorer_ui(ui);
                        },
                    );

                    // Draggable vertical splitter between Explorer and Properties
                    let (split_rect, split_resp) = ui.allocate_exact_size(
                        egui::vec2(inner_w, 10.0),
                        egui::Sense::drag(),
                    );
                    if split_resp.dragged() {
                        let delta = split_resp.drag_delta().y / total_h;
                        self.ui.portrait_drawer_split = (self.ui.portrait_drawer_split + delta).clamp(0.25, 0.75);
                    }
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(split_rect.center(), egui::vec2(42.0, 2.5)),
                        2.0,
                        crate::wallpaper::PASTEL_LAVENDER.gamma_multiply(0.55),
                    );

                    // Bottom Half: Properties
                    ui.allocate_ui_with_layout(
                        egui::vec2(inner_w, bot_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_width(inner_w);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Properties")
                                        .strong()
                                        .size(12.5)
                                        .color(crate::wallpaper::PASTEL_LAVENDER),
                                );
                            });
                            ui.separator();
                            self.properties_ui(ui);
                        },
                    );
                });
        } else {
            // Collapsed Slide-In Handle on the right edge
            egui::Area::new(egui::Id::new("landscape_inspector_restore_handle"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-2.0, 0.0))
                .show(ctx, |ui| {
                    if ui
                        .add_sized(
                            [26.0, 52.0],
                            egui::Button::new(
                                egui::RichText::new("◀")
                                    .color(crate::wallpaper::PASTEL_MINT)
                                    .strong(),
                            )
                            .fill(Color32::from_rgba_unmultiplied(18, 20, 27, 225))
                            .corner_radius(10.0),
                        )
                        .on_hover_text("Slide in Explorer & Properties")
                        .clicked()
                    {
                        inspector_open = true;
                        ctx.data_mut(|d| d.insert_temp(inspector_open_id, true));
                    }
                });
        }

        // Optional Resizable Output Drawer at bottom of left workspace pane when toggled
        if self.ui.portrait_output_drawer {
            let max_out_h = (screen_rect.height() * 0.65).max(160.0);
            self.ui.output_height = self.ui.output_height.clamp(90.0, max_out_h);
            egui::Panel::bottom("landscape_output_drawer")
                .exact_size(self.ui.output_height)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(20, 22, 30, 244))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                        .inner_margin(egui::Margin::symmetric(8, 4)),
                )
                .show(ui, |ui| {
                    let (grip_rect, grip_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 10.0),
                        egui::Sense::drag(),
                    );
                    if grip_resp.dragged() {
                        self.ui.output_height = (self.ui.output_height - grip_resp.drag_delta().y)
                            .clamp(90.0, max_out_h);
                    }
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(grip_rect.center(), egui::vec2(48.0, 3.0)),
                        2.0,
                        if grip_resp.hovered() || grip_resp.dragged() {
                            crate::wallpaper::LOGO_CYAN
                        } else {
                            Color32::from_rgb(80, 88, 108)
                        },
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📟 Output")
                                .strong()
                                .size(12.0)
                                .color(crate::wallpaper::LOGO_CYAN),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").clicked() {
                                self.ui.portrait_output_drawer = false;
                            }
                        });
                    });
                    ui.separator();
                    self.output_ui(ui);
                });
        }

        // 2b. Flat Full-Width Bottom Status Bar BEFORE CentralPanel
        let kb_h = crate::platform::keyboard_height_points(ctx);
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("pill_dock_cam_lift"), 0.0_f32));
        if kb_h <= 40.0 {
            egui::Panel::bottom("landscape_flat_status_bar")
                .exact_size(24.0 + safe_bottom * 0.5)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(16, 18, 25))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(34, 38, 50)))
                        .inner_margin(egui::Margin {
                            left: side_left,
                            right: side_right,
                            top: 2,
                            bottom: (2.0 + safe_bottom * 0.35).round().clamp(2.0, 32.0) as i8,
                        }),
                )
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new(concat!("Output  rustudio ", env!("CARGO_PKG_VERSION"), " ready · 931 classes"))
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let tag = if cfg!(target_os = "ios") {
                                "iPhone Safe-Area Active"
                            } else {
                                "Android Vulkan"
                            };
                            ui.label(egui::RichText::new(tag).size(10.5).color(theme::TEXT_DIM));
                            if ui.small_button("Output").clicked() {
                                self.ui.portrait_output_drawer = !self.ui.portrait_output_drawer;
                            }
                            if ui.small_button("Code").clicked() {
                                self.ui.portrait_tab = if self.ui.portrait_tab == PortraitTab::Code {
                                    PortraitTab::Viewport
                                } else {
                                    PortraitTab::Code
                                };
                            }
                        });
                    });
                });
        }

        // 3. Left Main 3D Viewport (~74% width, full height below top bar)
        //    ALWAYS rendered as the base layer so the 3D scene never turns into a blank black screen!
        let mut left_pane_rect = screen_rect;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG))
            .show(ui, |ui| {
                left_pane_rect = ui.max_rect();
                self.viewport_ui(ui);
            });

        // Subtle Top-Left Live Streaming Telemetry Badge in 3D Viewport
        if self.ui.portrait_tab == PortraitTab::Viewport && self.ui.portrait_tool_panel.is_none() {
            let (pending_tex, loaded_tex) = self.game.textures.counts();
            let loaded_meshes = self.game.meshes.count();
            let badge_pos = left_pane_rect.min + Vec2::new(10.0 + safe_left * 0.5, 8.0);
            egui::Area::new(egui::Id::new("vp_streaming_telemetry_badge"))
                .order(egui::Order::Foreground)
                .interactable(false)
                .fixed_pos(badge_pos)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(16, 18, 25, 200))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Streaming: {}m / {}t ({} pending)",
                                    loaded_meshes, loaded_tex, pending_tex
                                ))
                                .monospace()
                                .size(10.5)
                                .color(crate::wallpaper::LOGO_CYAN),
                            );
                        });
                });
        }

        // 3b. Dismissible Floating / Overlay Card for Script Editor or Tool Panel (`Toolbox` / `Terrain`)
        //     with a prominent `← Back to 3D` pill button so the user is never trapped away from the 3D scene.
        if self.ui.portrait_tab == PortraitTab::Code || self.ui.portrait_tool_panel.is_some() {
            let card_rect = left_pane_rect.shrink2(Vec2::new(10.0, 8.0));
            let shown = egui::Area::new(egui::Id::new("landscape_overlay_editor_card"))
                .order(egui::Order::Middle)
                .interactable(true)
                .fixed_pos(card_rect.min)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(18, 20, 28, 245))
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(10))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 8],
                            blur: 22,
                            spread: 0,
                            color: Color32::from_black_alpha(140),
                        })
                        .show(ui, |ui| {
                            ui.set_min_size(card_rect.size() - Vec2::splat(20.0));
                            ui.set_max_size(card_rect.size() - Vec2::splat(20.0));
                            ui.painter().rect_filled(ui.max_rect(), 0.0, crate::wallpaper::OBSIDIAN_CANVAS);

                            // Top navigation bar with `← Back to 3D` pill button + open script tabs
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let back_btn = egui::Button::new(
                                    egui::RichText::new("← Back to 3D")
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::BLACK),
                                )
                                .fill(crate::wallpaper::PASTEL_LAVENDER)
                                .corner_radius(14.0)
                                .min_size(Vec2::new(112.0, 32.0));
                                if ui.add(back_btn).clicked() {
                                    self.ui.portrait_tab = PortraitTab::Viewport;
                                    self.ui.portrait_tool_panel = None;
                                }

                                if let Some(tool_tab) = self.ui.portrait_tool_panel {
                                    ui.label(
                                        egui::RichText::new(tool_tab.title())
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                } else {
                                    egui::ScrollArea::horizontal().id_salt("landscape_script_tabs_bar").show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let mut close_idx: Option<usize> = None;
                                            let open_list = self.ui.portrait_open_scripts.clone();
                                            for (idx, r) in open_list.into_iter().enumerate() {
                                                let name = self
                                                    .game
                                                    .doc
                                                    .dom
                                                    .get_by_ref(r)
                                                    .map(|i| i.name.as_str())
                                                    .unwrap_or("Script");
                                                let active = self.ui.portrait_active_script == Some(r);
                                                if ui.selectable_label(active, format!("📜 {name}")).clicked() {
                                                    self.ui.portrait_active_script = Some(r);
                                                }
                                                if ui.small_button("×").clicked() {
                                                    close_idx = Some(idx);
                                                }
                                            }
                                            if let Some(idx) = close_idx {
                                                let removed = self.ui.portrait_open_scripts.remove(idx);
                                                if self.ui.portrait_active_script == Some(removed) {
                                                    self.ui.portrait_active_script = self.ui.portrait_open_scripts.last().copied();
                                                }
                                            }
                                            if ui.small_button(" + New Script ").clicked() {
                                                self.ui_action(ctx, "script");
                                            }
                                        });
                                    });
                                }
                            });
                            ui.separator();

                            if let Some(tool_tab) = self.ui.portrait_tool_panel {
                                match tool_tab {
                                    crate::app::Tab::Toolbox => self.toolbox_ui(ui),
                                    crate::app::Tab::TerrainEditor => self.terrain_editor_ui(ui),
                                    _ => {}
                                }
                                return;
                            }

                            if self.ui.portrait_open_scripts.is_empty() {
                                if let Some(first_ref) = self
                                    .game
                                    .doc
                                    .dom
                                    .descendants_of(self.game.doc.dom.root_ref())
                                    .find(|i| matches!(i.class.as_str(), "Script" | "LocalScript" | "ModuleScript"))
                                    .map(|i| i.referent())
                                {
                                    self.ui.portrait_open_scripts.push(first_ref);
                                    self.ui.portrait_active_script = Some(first_ref);
                                }
                            }
                            if self.ui.portrait_active_script.is_none() {
                                self.ui.portrait_active_script = self.ui.portrait_open_scripts.first().copied();
                            }
                            if let Some(r) = self.ui.portrait_active_script {
                                self.script_tab_ui(ui, r);
                            } else {
                                ui.centered_and_justified(|ui| {
                                    if ui
                                        .add_sized(
                                            [180.0, 40.0],
                                            egui::Button::new("📜 + Tạo Script Luau mới"),
                                        )
                                        .clicked()
                                    {
                                        self.ui_action(ctx, "script");
                                    }
                                });
                            }
                        })
                });
            ctx.layer_painter(shown.response.layer_id).rect_stroke(
                shown.inner.response.rect,
                egui::CornerRadius::same(14),
                egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                egui::StrokeKind::Inside,
            );
        }

        // Quick Tools & Plugins Grid Popup (from bottom `+` button)
        if self.ui.portrait_quick_grid_open {
            let mut open = self.ui.portrait_quick_grid_open;
            let shown = egui::Window::new("⚡ Quick Tools & Plugins")
                .id(egui::Id::new("landscape_quick_grid"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(-target_insp_w * 0.35 * insp_anim, -(66.0 + safe_bottom)))
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(Color32::from_rgba_unmultiplied(20, 22, 30, 242))
                        .corner_radius(egui::CornerRadius::same(16))
                        .inner_margin(egui::Margin::same(10)),
                )
                .show(ctx, |ui| {
                    let btn_sz = egui::vec2(105.0, 38.0);
                    egui::Grid::new("landscape_tools_grid").spacing([8.0, 8.0]).show(ui, |ui| {
                        if ui.add_sized(btn_sz, egui::Button::new("🔌 Plugins")).clicked() {
                            self.ui_action(ctx, "manage_plugins");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("🧑 Character")).clicked() {
                            self.ui_action(ctx, "character");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("🧰 Toolbox")).clicked() {
                            self.ui.portrait_tool_panel = Some(crate::app::Tab::Toolbox);
                            self.ui.portrait_quick_grid_open = false;
                        }
                        ui.end_row();
                        if ui.add_sized(btn_sz, egui::Button::new("⛰ Terrain")).clicked() {
                            self.ui.portrait_tool_panel = Some(crate::app::Tab::TerrainEditor);
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("➕ Insert Obj")).clicked() {
                            self.ui_action(ctx, "insert_object");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        if ui.add_sized(btn_sz, egui::Button::new("📜 New Script")).clicked() {
                            self.ui_action(ctx, "script");
                            self.ui.portrait_quick_grid_open = false;
                        }
                        ui.end_row();
                    });
                });
            if let Some(r) = shown {
                ctx.layer_painter(r.response.layer_id).rect_stroke(
                    r.response.rect,
                    egui::CornerRadius::same(16),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
                    egui::StrokeKind::Inside,
                );
            }
            if !open {
                self.ui.portrait_quick_grid_open = false;
            }
        }

        Self::hardware_perf_popup_ui(ctx);
        crate::mcp_assistant::ui(self, ctx);
        let has_open_plugin = self.has_open_plugin();
        crate::touch_input_bridge::floating_shortcut_pill_ui(ctx, has_open_plugin);
    }

    fn floating_bottom_pill_dock_ui(
        &mut self,
        ctx: &egui::Context,
        screen_rect: Rect,
        safe_bottom: f32,
        dock_collapsed: &mut bool,
        dock_open_t: f32,
    ) {
        use crate::studio_ui::PortraitTab;

        // When collapsed, show a sleek upward arrow pill (`▴ Dock`) at the bottom center
        if *dock_collapsed && dock_open_t < 0.05 {
            let pill_w = 88.0;
            let pill_h = 26.0;
            let pos = egui::pos2(
                screen_rect.center().x - pill_w * 0.5,
                screen_rect.bottom() - safe_bottom.max(6.0) - pill_h - 4.0,
            );
            egui::Area::new(egui::Id::new("portrait_pill_dock_collapsed_restore"))
                .order(egui::Order::Foreground)
                .interactable(true)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(pill_w, pill_h), Sense::click());
                    ui.painter().rect_filled(
                        r,
                        egui::CornerRadius::same(13),
                        Color32::from_rgba_unmultiplied(18, 20, 26, 200),
                    );
                    ui.painter().rect_stroke(
                        r,
                        egui::CornerRadius::same(13),
                        egui::Stroke::new(1.0, crate::wallpaper::PASTEL_LAVENDER.gamma_multiply(0.6)),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        "▴ Dock",
                        egui::FontId::proportional(12.0),
                        crate::wallpaper::PASTEL_LAVENDER,
                    );
                    if resp.clicked() {
                        *dock_collapsed = false;
                    }
                });
            return;
        }

        let max_dock_w = (screen_rect.width() - 16.0).max(220.0);
        let dock_w = max_dock_w.clamp(220.0, 440.0).min(max_dock_w);
        let dock_h = 54.0;
        let slide_offset = (1.0 - dock_open_t) * (dock_h + safe_bottom + 20.0);
        let dock_x = (screen_rect.center().x - dock_w * 0.5)
            .clamp(screen_rect.left() + 8.0, (screen_rect.right() - dock_w - 8.0).max(screen_rect.left() + 8.0));
        let dock_y = screen_rect.bottom() - safe_bottom.max(6.0) - dock_h - 4.0 + slide_offset;

        egui::Area::new(egui::Id::new("portrait_floating_pill_dock_area"))
            .order(egui::Order::Foreground)
            .interactable(true)
            .fixed_pos(egui::pos2(dock_x, dock_y))
            .show(ctx, |ui| {
                let (full_r, _) = ui.allocate_exact_size(Vec2::new(dock_w, dock_h), Sense::hover());

                // Dark-Translucent Pill Dock background: rgba(18, 20, 26, 0.72) (~184 alpha), rounding 24.0, 1px crisp border
                let cr = egui::CornerRadius::same(24);
                ui.painter().rect_filled(
                    full_r,
                    cr,
                    Color32::from_rgba_unmultiplied(18, 20, 26, 188),
                );
                ui.painter().rect_stroke(
                    full_r,
                    cr,
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(184, 181, 255, 78)),
                    egui::StrokeKind::Inside,
                );

                // Right-edge compact collapse arrow toggle (`▾`)
                let collapse_w = 26.0;
                let tabs_area_w = dock_w - collapse_w - 10.0;
                let item_w = (tabs_area_w / 5.0).max(42.0);
                let items: [(&str, &str, Color32); 5] = [
                    ("cube", "Workspace", crate::wallpaper::PASTEL_LAVENDER),
                    ("file-code", "Code", crate::wallpaper::PASTEL_SKY),
                    ("layout-list", "Explorer", crate::wallpaper::PASTEL_MINT),
                    ("terminal-2", "Output", crate::wallpaper::PASTEL_GOLD),
                    ("sparkles", "+", crate::wallpaper::PASTEL_PEACH),
                ];

                // Track Press-to-Glow & Horizontal Scrubbing across the 5 tabs
                let tabs_rect = Rect::from_min_size(
                    egui::pos2(full_r.left() + 6.0, full_r.top() + 4.0),
                    Vec2::new(tabs_area_w, dock_h - 8.0),
                );
                let scrub_id = egui::Id::new("portrait_pill_dock_scrub_state");
                // (press_start_time: Option<f64>, hovered_idx: Option<usize>, scrubbing: bool)
                let (mut press_t0, mut scrub_idx, mut is_scrubbing) = ctx
                    .data(|d| d.get_temp::<(Option<f64>, Option<usize>, bool)>(scrub_id))
                    .unwrap_or((None, None, false));

                let now = ctx.input(|i| i.time);
                let ptr_pos = ctx.input(|i| i.pointer.interact_pos());
                let ptr_down = ctx.input(|i| i.pointer.primary_down());
                let ptr_pressed = ctx.input(|i| i.pointer.any_pressed());
                let ptr_released = ctx.input(|i| i.pointer.any_released());
                let drag_delta = ctx.input(|i| i.pointer.delta());

                if ptr_pressed {
                    if let Some(p) = ptr_pos {
                        if tabs_rect.contains(p) {
                            press_t0 = Some(now);
                            let rel_x = (p.x - tabs_rect.left()).clamp(0.0, tabs_area_w - 0.01);
                            scrub_idx = Some(((rel_x / item_w).floor() as usize).min(4));
                            is_scrubbing = false;
                        }
                    }
                }

                if ptr_down {
                    if let (Some(t0), Some(p)) = (press_t0, ptr_pos) {
                        if (now - t0) > 0.14 || drag_delta.x.abs() > 3.5 {
                            is_scrubbing = true;
                        }
                        if tabs_rect.expand(18.0).contains(p) {
                            let rel_x = (p.x - tabs_rect.left()).clamp(0.0, tabs_area_w - 0.01);
                            scrub_idx = Some(((rel_x / item_w).floor() as usize).min(4));
                        }
                        ctx.request_repaint();
                    }
                }

                let glow_intensity = ctx.animate_bool_with_time(
                    egui::Id::new("portrait_pill_dock_glow"),
                    is_scrubbing || press_t0.is_some(),
                    0.14,
                );

                let mut activated_idx: Option<usize> = None;
                if ptr_released {
                    if press_t0.is_some() {
                        activated_idx = scrub_idx;
                    }
                    press_t0 = None;
                    scrub_idx = None;
                    is_scrubbing = false;
                }

                ctx.data_mut(|d| d.insert_temp(scrub_id, (press_t0, scrub_idx, is_scrubbing)));

                // Determine active tab index (0..=4) and animate a smooth Jelly Pill Glide
                // that subtly stretches horizontally while moving between tabs and settles into place.
                let active_tab_idx: usize = if self.ui.portrait_quick_grid_open {
                    4
                } else if self.ui.portrait_output_drawer {
                    3
                } else if self.ui.portrait_explorer_drawer {
                    2
                } else if self.ui.portrait_tab == PortraitTab::Code && self.ui.portrait_tool_panel.is_none() {
                    1
                } else {
                    0
                };
                let target_idx = scrub_idx.unwrap_or(active_tab_idx);
                let target_x = tabs_rect.left() + (target_idx as f32) * item_w + item_w * 0.5;
                let anim_x = ctx.animate_value_with_time(
                    egui::Id::new("pill_dock_jelly_x"),
                    target_x,
                    0.18,
                );
                let velocity_stretch = ((target_x - anim_x).abs() / item_w.max(1.0)).clamp(0.0, 0.55);
                let pill_accent = items[target_idx.min(4)].2;
                let jelly_w = (item_w - 4.0) * (1.0 + 0.36 * velocity_stretch + 0.05 * glow_intensity);
                let jelly_h = tabs_rect.height() * (1.0 - 0.10 * velocity_stretch + 0.04 * glow_intensity);
                let jelly_rect = Rect::from_center_size(
                    egui::pos2(anim_x, tabs_rect.center().y),
                    Vec2::new(jelly_w, jelly_h),
                );
                let fill_alpha = (54.0 + 44.0 * glow_intensity) as u8;
                ui.painter().rect_filled(
                    jelly_rect,
                    egui::CornerRadius::same(18),
                    Color32::from_rgba_unmultiplied(pill_accent.r(), pill_accent.g(), pill_accent.b(), fill_alpha),
                );
                ui.painter().rect_stroke(
                    jelly_rect,
                    egui::CornerRadius::same(18),
                    egui::Stroke::new(
                        1.0 + 0.5 * glow_intensity,
                        Color32::from_rgba_unmultiplied(255, 255, 255, (52.0 + 48.0 * glow_intensity) as u8),
                    ),
                    egui::StrokeKind::Inside,
                );

                for (idx, (icon, label, accent)) in items.iter().enumerate() {
                    let is_active = idx == active_tab_idx;
                    let is_scrub_target = scrub_idx == Some(idx);
                    let highlight = is_active || is_scrub_target;

                    let base_r = Rect::from_min_size(
                        egui::pos2(tabs_rect.left() + idx as f32 * item_w + 2.0, tabs_rect.top()),
                        Vec2::new(item_w - 4.0, tabs_rect.height()),
                    );
                    // Tactile 0.96x press squish when initially pressed, or 1.05x pulse while scrubbing
                    let scale = if is_scrub_target && !is_scrubbing && press_t0.is_some() {
                        0.96
                    } else if is_scrub_target {
                        1.0 + 0.05 * glow_intensity
                    } else {
                        1.0
                    };
                    let item_r = Rect::from_center_size(base_r.center(), base_r.size() * scale);

                    // Icon on top, label below
                    let ic_r = Rect::from_center_size(
                        egui::pos2(item_r.center().x, item_r.top() + 14.0),
                        Vec2::splat(17.0 * scale),
                    );
                    let ic_col = if highlight { *accent } else { theme::TEXT.gamma_multiply(0.82) };
                    rustudio_engine::ti::paint(ui.painter(), ic_r, icon, ic_col);

                    let lbl_col = if highlight { Color32::WHITE } else { theme::TEXT_DIM };
                    ui.painter().text(
                        egui::pos2(item_r.center().x, item_r.bottom() - 10.0),
                        egui::Align2::CENTER_CENTER,
                        *label,
                        egui::FontId::proportional(10.5),
                        lbl_col,
                    );
                }

                if let Some(idx) = activated_idx {
                    match idx {
                        0 => {
                            self.ui.portrait_tab = PortraitTab::Viewport;
                            self.ui.portrait_tool_panel = None;
                            self.ui.portrait_explorer_drawer = false;
                            self.ui.portrait_output_drawer = false;
                            self.ui.portrait_quick_grid_open = false;
                        }
                        1 => {
                            self.ui.portrait_tab = PortraitTab::Code;
                            self.ui.portrait_tool_panel = None;
                            self.ui.portrait_quick_grid_open = false;
                        }
                        2 => {
                            self.ui.portrait_explorer_drawer = !self.ui.portrait_explorer_drawer;
                            if self.ui.portrait_explorer_drawer {
                                self.ui.portrait_output_drawer = false;
                                self.ui.portrait_quick_grid_open = false;
                            }
                        }
                        3 => {
                            self.ui.portrait_output_drawer = !self.ui.portrait_output_drawer;
                            if self.ui.portrait_output_drawer {
                                self.ui.portrait_explorer_drawer = false;
                                self.ui.portrait_quick_grid_open = false;
                            }
                        }
                        4 => {
                            self.ui.portrait_quick_grid_open = !self.ui.portrait_quick_grid_open;
                        }
                        _ => {}
                    }
                }

                // Right-edge Collapse Arrow Button (`▾`)
                let col_r = Rect::from_center_size(
                    egui::pos2(full_r.right() - collapse_w * 0.5 - 3.0, full_r.center().y),
                    Vec2::new(22.0, 34.0),
                );
                let col_resp = ui.interact(col_r, egui::Id::new("portrait_pill_dock_collapse_btn"), Sense::click());
                if col_resp.hovered() {
                    ui.painter().rect_filled(
                        col_r,
                        egui::CornerRadius::same(11),
                        Color32::from_rgba_unmultiplied(60, 65, 82, 160),
                    );
                }
                ui.painter().text(
                    col_r.center(),
                    egui::Align2::CENTER_CENTER,
                    "▾",
                    egui::FontId::proportional(13.5),
                    crate::wallpaper::PASTEL_LAVENDER,
                );
                if col_resp.on_hover_text("Ẩn thanh Dock xuống (Collapse Dock)").clicked() {
                    *dock_collapsed = true;
                }
            });
    }

    fn hardware_perf_popup_ui(ctx: &egui::Context) {
        let perf_id = egui::Id::new("mobile_perf_popup_open");
        let mut open = ctx.data(|d| d.get_temp::<bool>(perf_id).unwrap_or(false));
        if !open {
            return;
        }
        let hw = rustudio_core::hardware_tuner::hardware_profile();
        let mut cfg = rustudio_core::hardware_tuner::config();
        let rss_mb = rustudio_core::hardware_tuner::current_rss_mb();
        let dt = ctx.input(|i| i.unstable_dt).max(0.0001);
        let fps = (1.0 / dt).round() as u32;

        egui::Window::new("⚡ Hardware & Performance")
            .id(egui::Id::new("mobile_hardware_perf_popup"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgb(26, 28, 34))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("GPU: {} ({})", hw.gpu_name, hw.gpu_backend))
                                .small()
                                .color(if hw.is_software_gpu {
                                    Color32::from_rgb(255, 165, 95)
                                } else {
                                    Color32::from_rgb(130, 220, 155)
                                }),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "FPS: {}  |  RAM: {:.0} MB / {} MB  |  CPU: {} cores",
                                fps, rss_mb, hw.total_ram_mb, hw.cpu_cores
                            ))
                            .small()
                            .color(theme::TEXT_DIM),
                        );
                    });

                ui.add_space(6.0);
                ui.label(egui::RichText::new("Preset").strong().small());
                ui.horizontal_wrapped(|ui| {
                    for preset in [
                        rustudio_core::hardware_tuner::PerfPreset::AutoDetect,
                        rustudio_core::hardware_tuner::PerfPreset::Ultra,
                        rustudio_core::hardware_tuner::PerfPreset::Balanced,
                        rustudio_core::hardware_tuner::PerfPreset::BatterySaver,
                        rustudio_core::hardware_tuner::PerfPreset::Custom,
                    ] {
                        if ui.selectable_label(cfg.preset == preset, preset.label()).clicked() {
                            rustudio_core::hardware_tuner::apply_preset(preset);
                            cfg = rustudio_core::hardware_tuner::config();
                        }
                    }
                });

                ui.add_space(6.0);
                let mut changed = false;
                let mut cache = cfg.cache_budget_mb;
                if ui
                    .add(egui::Slider::new(&mut cache, 512..=3072).text("RAM & Cache (MB)"))
                    .changed()
                {
                    cfg.cache_budget_mb = cache;
                    changed = true;
                }
                let mut gc = cfg.luau_gc_mb;
                if ui
                    .add(egui::Slider::new(&mut gc, 16..=512).text("Luau GC (MB)"))
                    .changed()
                {
                    cfg.luau_gc_mb = gc;
                    changed = true;
                }
                let mut scale = cfg.render_scale;
                if ui
                    .add(egui::Slider::new(&mut scale, 0.05..=1.25).step_by(0.05).text("Render Scale"))
                    .changed()
                {
                    cfg.render_scale = scale;
                    changed = true;
                }

                ui.horizontal(|ui| {
                    ui.label("FPS Limit:");
                    for (val, label) in [(30, "30"), (60, "60"), (0, "Unlimited")] {
                        if ui.selectable_label(cfg.fps_limit == val, label).clicked() {
                            cfg.fps_limit = val;
                            changed = true;
                        }
                    }
                });

                if ui.checkbox(&mut cfg.shadows_enabled, "Shadows").changed() {
                    changed = true;
                }

                if changed {
                    rustudio_core::hardware_tuner::update_config(|c| *c = cfg.clone());
                }
            });

        ctx.data_mut(|d| d.insert_temp(perf_id, open));
    }

    /// Pure Focus Code IDE mode: skips 3D rendering completely, shows Code Editor,
    /// draggable/dockable Explorer (Left / Right / Bottom + ghost preview + corner resize),
    /// and a full-width vertically resizable Output drawer.
    pub fn focus_code_ide_ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        screen_rect: egui::Rect,
    ) {
        use crate::studio_ui::ExplorerDockSide;

        // Sync any scripts opened from Explorer
        for r in std::mem::take(&mut self.open_script) {
            if !self.ui.portrait_open_scripts.contains(&r) {
                self.ui.portrait_open_scripts.push(r);
            }
            self.ui.portrait_active_script = Some(r);
        }
        self.ui
            .portrait_open_scripts
            .retain(|r| self.game.doc.dom.get_by_ref(*r).is_some());
        if self.ui.portrait_active_script.is_none() {
            self.ui.portrait_active_script = self.ui.portrait_open_scripts.first().copied();
        }

        let (safe_top, safe_bottom, safe_left, safe_right) = crate::platform::get_safe_area_insets();
        let top_pad = (4.0 + safe_top * 0.35).round().clamp(3.0, 48.0) as i8;
        let side_left = (8.0 + safe_left).round().clamp(6.0, 120.0) as i8;
        let side_right = (8.0 + safe_right).round().clamp(6.0, 120.0) as i8;

        // 1. IDE Top Bar
        egui::Panel::top("focus_code_top_bar")
            .exact_size(40.0 + top_pad as f32)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(15, 17, 24))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(36, 40, 52)))
                    .inner_margin(egui::Margin {
                        left: side_left,
                        right: side_right,
                        top: top_pad,
                        bottom: 4,
                    }),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("← 3D Studio")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(crate::wallpaper::RUST_ORANGE)
                            .corner_radius(8.0)
                            .min_size(Vec2::new(88.0, 26.0)),
                        )
                        .clicked()
                    {
                        self.ui.focus_code_mode = false;
                        self.ui.viewport_hidden = false;
                    }

                    ui.label(
                        egui::RichText::new("Focus Code IDE")
                            .strong()
                            .size(12.5)
                            .color(crate::wallpaper::LOGO_CYAN),
                    );

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("+ Script").size(11.5).color(Color32::WHITE))
                                .fill(Color32::from_rgb(34, 38, 50))
                                .corner_radius(6.0),
                        )
                        .clicked()
                    {
                        self.ui_action(ctx, "script");
                    }

                    let exp_open_id = egui::Id::new("focus_code_explorer_open");
                    let mut exp_open = ctx.data(|d| d.get_temp::<bool>(exp_open_id).unwrap_or(true));
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Explorer")
                                    .size(11.5)
                                    .color(if exp_open { Color32::BLACK } else { Color32::WHITE }),
                            )
                            .fill(if exp_open {
                                crate::wallpaper::LOGO_CYAN
                            } else {
                                Color32::from_rgb(34, 38, 50)
                            })
                            .corner_radius(6.0),
                        )
                        .clicked()
                    {
                        exp_open = !exp_open;
                        ctx.data_mut(|d| d.insert_temp(exp_open_id, exp_open));
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Output")
                                    .size(11.5)
                                    .color(if self.ui.portrait_output_drawer {
                                        Color32::BLACK
                                    } else {
                                        Color32::WHITE
                                    }),
                            )
                            .fill(if self.ui.portrait_output_drawer {
                                crate::wallpaper::RUST_ORANGE_HOVER
                            } else {
                                Color32::from_rgb(34, 38, 50)
                            })
                            .corner_radius(6.0),
                        )
                        .clicked()
                    {
                        self.ui.portrait_output_drawer = !self.ui.portrait_output_drawer;
                    }

                    // Script tabs
                    egui::ScrollArea::horizontal()
                        .id_salt("focus_code_tabs_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let open_list = self.ui.portrait_open_scripts.clone();
                                for r in open_list {
                                    let name = self
                                        .game
                                        .doc
                                        .dom
                                        .get_by_ref(r)
                                        .map(|i| i.name.clone())
                                        .unwrap_or_else(|| "Script".into());
                                    let active = self.ui.portrait_active_script == Some(r);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(name)
                                                    .size(11.5)
                                                    .color(if active { Color32::WHITE } else { theme::TEXT_DIM }),
                                            )
                                            .fill(if active {
                                                Color32::from_rgb(42, 48, 64)
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .corner_radius(6.0),
                                        )
                                        .clicked()
                                    {
                                        self.ui.portrait_active_script = Some(r);
                                    }
                                }
                            });
                        });
                });
            });

        // 2. Resizable Output Panel at bottom with full-width vertical drag handle
        if self.ui.portrait_output_drawer {
            let max_out_h = (screen_rect.height() * 0.65).max(160.0);
            self.ui.output_height = self.ui.output_height.clamp(90.0, max_out_h);
            egui::Panel::bottom("focus_code_output_panel")
                .exact_size(self.ui.output_height)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 20, 27))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 42, 54)))
                        .inner_margin(egui::Margin {
                            left: side_left,
                            right: side_right,
                            top: 4,
                            bottom: (6.0 + safe_bottom * 0.5).round() as i8,
                        }),
                )
                .show(ui, |ui| {
                    let (grip_rect, grip_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 10.0),
                        egui::Sense::drag(),
                    );
                    if grip_resp.dragged() {
                        self.ui.output_height = (self.ui.output_height - grip_resp.drag_delta().y)
                            .clamp(90.0, max_out_h);
                    }
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(grip_rect.center(), egui::vec2(56.0, 3.0)),
                        2.0,
                        if grip_resp.hovered() || grip_resp.dragged() {
                            crate::wallpaper::LOGO_CYAN
                        } else {
                            Color32::from_rgb(80, 88, 108)
                        },
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📟 Output")
                                .strong()
                                .size(12.0)
                                .color(crate::wallpaper::LOGO_CYAN),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").clicked() {
                                self.ui.portrait_output_drawer = false;
                            }
                        });
                    });
                    ui.separator();
                    self.output_ui(ui);
                });
        }

        // 3. Draggable & Dockable Explorer (Left / Right / Bottom) with ghost preview & corner resize grip
        let exp_open = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("focus_code_explorer_open")).unwrap_or(true));
        let exp_drag_id = egui::Id::new("focus_code_explorer_dragging");
        let mut dragging_exp = ctx.data(|d| d.get_temp::<bool>(exp_drag_id).unwrap_or(false));

        if exp_open {
            let max_exp = match self.ui.explorer_dock {
                ExplorerDockSide::Left | ExplorerDockSide::Right => (screen_rect.width() * 0.55).max(200.0),
                ExplorerDockSide::Bottom => (screen_rect.height() * 0.55).max(140.0),
            };
            self.ui.explorer_size = self.ui.explorer_size.clamp(160.0, max_exp);
            let dock_side = self.ui.explorer_dock;
            let frame = egui::Frame::new()
                .fill(Color32::from_rgb(19, 21, 28))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(40, 44, 58)))
                .inner_margin(egui::Margin::same(6));

            let mut draw_explorer_inner = |this: &mut Self, ui: &mut egui::Ui| {
                ui.horizontal(|ui| {
                    let (hdr_rect, hdr_resp) = ui.allocate_exact_size(
                        egui::vec2((ui.available_width() - 32.0).max(80.0), 24.0),
                        egui::Sense::drag(),
                    );
                    if hdr_resp.drag_started() {
                        dragging_exp = true;
                        ctx.data_mut(|d| d.insert_temp(exp_drag_id, true));
                    }
                    ui.painter().text(
                        hdr_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        "≡ Explorer (Hold & Drag to Dock)",
                        egui::FontId::proportional(11.5),
                        crate::wallpaper::LOGO_CYAN,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("➕").clicked() {
                            this.ui_action(ctx, "insert_object");
                        }
                    });
                });
                ui.separator();
                let list_h = (ui.available_height() - 18.0).max(60.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), list_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        this.explorer_ui(ui);
                    },
                );
                // Small corner resize grip at bottom of Explorer
                ui.with_layout(
                    if dock_side == ExplorerDockSide::Right {
                        egui::Layout::left_to_right(egui::Align::BOTTOM)
                    } else {
                        egui::Layout::right_to_left(egui::Align::BOTTOM)
                    },
                    |ui| {
                        let (grip_r, grip_resp) =
                            ui.allocate_exact_size(egui::vec2(18.0, 14.0), egui::Sense::drag());
                        if grip_resp.dragged() {
                            let d = grip_resp.drag_delta();
                            let delta = match dock_side {
                                ExplorerDockSide::Left => d.x,
                                ExplorerDockSide::Right => -d.x,
                                ExplorerDockSide::Bottom => -d.y,
                            };
                            this.ui.explorer_size = (this.ui.explorer_size + delta).clamp(160.0, max_exp);
                        }
                        ui.painter().text(
                            grip_r.center(),
                            egui::Align2::CENTER_CENTER,
                            "◢",
                            egui::FontId::proportional(11.0),
                            if grip_resp.hovered() || grip_resp.dragged() {
                                crate::wallpaper::RUST_ORANGE
                            } else {
                                Color32::from_rgb(100, 108, 128)
                            },
                        );
                    },
                );
            };

            match dock_side {
                ExplorerDockSide::Left => {
                    egui::Panel::left("focus_code_explorer_left")
                        .exact_size(self.ui.explorer_size)
                        .resizable(false)
                        .frame(frame)
                        .show(ui, |ui| draw_explorer_inner(self, ui));
                }
                ExplorerDockSide::Right => {
                    egui::Panel::right("focus_code_explorer_right")
                        .exact_size(self.ui.explorer_size)
                        .resizable(false)
                        .frame(frame)
                        .show(ui, |ui| draw_explorer_inner(self, ui));
                }
                ExplorerDockSide::Bottom => {
                    egui::Panel::bottom("focus_code_explorer_bottom")
                        .exact_size(self.ui.explorer_size)
                        .resizable(false)
                        .frame(frame)
                        .show(ui, |ui| draw_explorer_inner(self, ui));
                }
            }
        }

        // 4. Translucent Ghost Preview + 3 Dock Drop Zones (Left / Right / Bottom) while dragging Explorer
        if dragging_exp {
            let ptr = ctx.input(|i| i.pointer.interact_pos());
            let still_down = ctx.input(|i| i.pointer.any_down());
            let zone_left = Rect::from_min_max(
                egui::pos2(screen_rect.left() + 8.0, screen_rect.top() + 52.0),
                egui::pos2(screen_rect.left() + 120.0, screen_rect.bottom() - 24.0),
            );
            let zone_right = Rect::from_min_max(
                egui::pos2(screen_rect.right() - 120.0, screen_rect.top() + 52.0),
                egui::pos2(screen_rect.right() - 8.0, screen_rect.bottom() - 24.0),
            );
            let zone_bottom = Rect::from_min_max(
                egui::pos2(screen_rect.left() + 128.0, screen_rect.bottom() - 120.0),
                egui::pos2(screen_rect.right() - 128.0, screen_rect.bottom() - 12.0),
            );
            let hovered_zone = ptr.and_then(|p| {
                if zone_left.contains(p) {
                    Some(ExplorerDockSide::Left)
                } else if zone_right.contains(p) {
                    Some(ExplorerDockSide::Right)
                } else if zone_bottom.contains(p) {
                    Some(ExplorerDockSide::Bottom)
                } else {
                    None
                }
            });

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("explorer_dock_overlay"),
            ));
            for (side, rect, label) in [
                (ExplorerDockSide::Left, zone_left, "Dock Left"),
                (ExplorerDockSide::Right, zone_right, "Dock Right"),
                (ExplorerDockSide::Bottom, zone_bottom, "Dock Bottom"),
            ] {
                let active = hovered_zone == Some(side);
                painter.rect(
                    rect,
                    egui::CornerRadius::same(12),
                    if active {
                        crate::wallpaper::LOGO_CYAN.gamma_multiply(0.24)
                    } else {
                        Color32::from_rgba_unmultiplied(30, 34, 46, 130)
                    },
                    egui::Stroke::new(
                        if active { 2.0 } else { 1.0 },
                        if active {
                            crate::wallpaper::LOGO_CYAN
                        } else {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 70)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }

            // Translucent Ghost Preview following pointer
            if let Some(p) = ptr {
                let ghost = Rect::from_center_size(p, Vec2::new(170.0, 210.0));
                painter.rect(
                    ghost,
                    egui::CornerRadius::same(12),
                    Color32::from_rgba_unmultiplied(25, 28, 38, 155),
                    egui::Stroke::new(1.5, crate::wallpaper::RUST_ORANGE.gamma_multiply(0.85)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    ghost.center_top() + Vec2::new(0.0, 18.0),
                    egui::Align2::CENTER_CENTER,
                    "Explorer",
                    egui::FontId::proportional(12.5),
                    Color32::WHITE.gamma_multiply(0.85),
                );
            }

            if !still_down {
                if let Some(side) = hovered_zone {
                    self.ui.explorer_dock = side;
                }
                ctx.data_mut(|d| d.insert_temp(exp_drag_id, false));
            }
        }

        // 5. Central Code Editor (no 3D viewport!) + Luau Quick-Symbol Bar (Stitch iPhone/Android IDE)
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(13, 15, 21)).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| {
                if let Some(r) = self.ui.portrait_active_script {
                    let body_h = (ui.available_height() - 36.0).max(120.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), body_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            self.script_tab_ui(ui, r);
                        },
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::horizontal()
                        .id_salt("luau_quick_symbols_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                for (label, snippet) in [
                                    ("Tab", "    "),
                                    ("local", "local "),
                                    ("function", "function "),
                                    ("end", "end"),
                                    ("()", "()"),
                                    ("{}", "{}"),
                                    ("[]", "[]"),
                                    (":", ":"),
                                    ("=", " = "),
                                    ("--", "-- "),
                                ] {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(label)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(32, 36, 48))
                                            .corner_radius(6.0)
                                            .min_size(Vec2::new(38.0, 26.0)),
                                        )
                                        .clicked()
                                    {
                                        ctx.input_mut(|i| {
                                            i.events.push(egui::Event::Text(snippet.to_string()));
                                        });
                                    }
                                }
                            });
                        });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Focus Code IDE — 3D Viewport Disabled")
                                    .size(16.0)
                                    .strong()
                                    .color(crate::wallpaper::LOGO_CYAN),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Select a Script in Explorer or create a new one to start editing.")
                                    .size(12.5)
                                    .color(theme::TEXT_DIM),
                            );
                            ui.add_space(10.0);
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("+ New Script").strong().color(Color32::WHITE))
                                        .fill(crate::wallpaper::RUST_ORANGE)
                                        .corner_radius(8.0),
                                )
                                .clicked()
                            {
                                self.ui_action(ctx, "script");
                            }
                        });
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    /// Every icon the top row and the ribbon name exists in the bundled set.
    #[test]
    fn icons_exist() {
        let all = include_str!("studio_ribbon.rs");
        // (the code above this test module, not the test's own strings)
        let src = &all[..all.find("#[cfg(test)]").unwrap_or(all.len())];
        let quoted: Vec<&str> = src.split('"').collect();
        let mut missing = Vec::new();
        for w in quoted.windows(2) {
            let calls = ["rb(ui, ", "top_icon(ui, ", "class_menu_button(ui, ", "rb_class(ui, ", "ribbon_button(ui, "];
            if calls.iter().any(|p| w[0].ends_with(p)) && !rustudio_engine::ti::has(w[1]) {
                missing.push(w[1]);
            }
        }
        for i in ["pointer", "arrows-move", "resize", "rotate-clockwise-2", "user", "chevron-down", "rotate", "check"] {
            if !rustudio_engine::ti::has(i) {
                missing.push(i);
            }
        }
        assert!(missing.is_empty(), "icons not in Tabler: {missing:?}");
    }
}

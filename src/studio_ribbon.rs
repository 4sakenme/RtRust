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
                        if ui.button("Toolbox").clicked() { self.ui_action(ctx, "toolbox"); ui.close(); }
                        if ui.button("Manage Plugins...").clicked() { self.ui_action(ctx, "manage_plugins"); ui.close(); }
                        if ui.button("Studio Settings...").clicked() { self.ui_action(ctx, "studio_settings"); ui.close(); }
                        if ui.button("Fonts & Typography...").clicked() { self.ui_action(ctx, "studio_settings:Fonts & Typography"); ui.close(); }
                        if ui.button("AI Assistant (MCP)").clicked() { crate::mcp_assistant::toggle_open(); ui.close(); }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let signed = self.game.account.signed_in();
                        let acc_label = if signed { "✓" } else { "Sign In" };
                        if ui.selectable_label(signed, acc_label).on_hover_text("Roblox Account").clicked() {
                            self.game.account.open = !self.game.account.open;
                            self.game.account.adding = false;
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
                        let arrow = if ribbon_open { "▴" } else { "▾" };
                        if ui.small_button(arrow).on_hover_text("Thu gọn/mở Ribbon").clicked() {
                            ribbon_open = !ribbon_open;
                            ctx.data_mut(|d| d.insert_temp(ribbon_open_id, ribbon_open));
                        }

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            egui::ScrollArea::horizontal().id_salt("portrait_ribbon_tab_scroll").show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                    for tab in crate::studio_ui::RibbonTab::ALL {
                                        let active = self.ui.tab == tab && ribbon_open;
                                        if ui.selectable_label(active, tab.label()).clicked() {
                                            if self.ui.tab == tab {
                                                ribbon_open = !ribbon_open;
                                                ctx.data_mut(|d| d.insert_temp(ribbon_open_id, ribbon_open));
                                            } else {
                                                self.ui.tab = tab;
                                                ribbon_open = true;
                                                ctx.data_mut(|d| d.insert_temp(ribbon_open_id, true));
                                            }
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
            });

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
                        if ui.selectable_label(is_3d, "3D View").clicked() {
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

        // 3. Bottom Navigation Bar (5 large buttons: 3D | Code | Explorer | Output | +)
        egui::Panel::bottom("portrait_bottom_bar")
            .exact_size(48.0 + safe_bottom)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL_DARK)
                    .inner_margin(egui::Margin {
                        left: side_left,
                        right: side_right,
                        top: 6,
                        bottom: bot_pad,
                    }),
            )
            .show(ui, |ui| {
                let w = ((ui.available_width() - 24.0) / 5.0).max(48.0);
                let h = 34.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                    if ui
                        .add_sized(
                            [w, h],
                            egui::Button::selectable(
                                self.ui.portrait_tab == PortraitTab::Viewport
                                    && self.ui.portrait_tool_panel.is_none()
                                    && !self.ui.portrait_explorer_drawer
                                    && !self.ui.portrait_output_drawer,
                                "3D",
                            ),
                        )
                        .clicked()
                    {
                        self.ui.portrait_tab = PortraitTab::Viewport;
                        self.ui.portrait_tool_panel = None;
                        self.ui.portrait_explorer_drawer = false;
                        self.ui.portrait_output_drawer = false;
                        self.ui.portrait_quick_grid_open = false;
                    }
                    if ui
                        .add_sized(
                            [w, h],
                            egui::Button::selectable(
                                self.ui.portrait_tab == PortraitTab::Code && self.ui.portrait_tool_panel.is_none(),
                                "Code",
                            ),
                        )
                        .clicked()
                    {
                        self.ui.portrait_tab = PortraitTab::Code;
                        self.ui.portrait_tool_panel = None;
                        self.ui.portrait_quick_grid_open = false;
                    }
                    if ui
                        .add_sized(
                            [w, h],
                            egui::Button::selectable(self.ui.portrait_explorer_drawer, "Explorer"),
                        )
                        .clicked()
                    {
                        self.ui.portrait_explorer_drawer = !self.ui.portrait_explorer_drawer;
                        if self.ui.portrait_explorer_drawer {
                            self.ui.portrait_output_drawer = false;
                            self.ui.portrait_quick_grid_open = false;
                        }
                    }
                    if ui
                        .add_sized(
                            [w, h],
                            egui::Button::selectable(self.ui.portrait_output_drawer, "Output"),
                        )
                        .clicked()
                    {
                        self.ui.portrait_output_drawer = !self.ui.portrait_output_drawer;
                        if self.ui.portrait_output_drawer {
                            self.ui.portrait_explorer_drawer = false;
                            self.ui.portrait_quick_grid_open = false;
                        }
                    }
                    if ui
                        .add_sized(
                            [w, h],
                            egui::Button::selectable(self.ui.portrait_quick_grid_open, "+"),
                        )
                        .clicked()
                    {
                        self.ui.portrait_quick_grid_open = !self.ui.portrait_quick_grid_open;
                    }
                });
            });

        // 4. Touch Luau Snippet Bar (above Bottom Tab Bar when in Code tab)
        if self.ui.portrait_tab == PortraitTab::Code && self.ui.portrait_tool_panel.is_none() {
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

        // 5. Output Bottom Drawer (only when toggled)
        if self.ui.portrait_output_drawer {
            let drawer_h = (screen_rect.height() * 0.38).clamp(160.0, 380.0);
            egui::Panel::bottom("portrait_output_drawer")
                .exact_size(drawer_h)
                .frame(
                    egui::Frame::new()
                        .fill(theme::PANEL)
                        .inner_margin(egui::Margin::symmetric(6, 4)),
                )
                .show(ui, |ui| {
                    self.output_ui(ui);
                });
        }

        // 6. Central Full-Screen Tab (3D Viewport, Luau Code Editor, or Tool Panel)
        let mut content_rect = screen_rect;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG))
            .show(ui, |ui| {
                content_rect = ui.max_rect();
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

        // 7. Quick Tools & Plugins Grid Popup (from bottom `+` button)
        if self.ui.portrait_quick_grid_open {
            let mut open = self.ui.portrait_quick_grid_open;
            egui::Window::new("⚡ Quick Tools & Plugins")
                .id(egui::Id::new("portrait_quick_grid"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -56.0))
                .show(ctx, |ui| {
                    let btn_sz = egui::vec2(110.0, 44.0);
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
                    });
                });
            if !open {
                self.ui.portrait_quick_grid_open = false;
            }
        }

        // 8. Non-blocking Floating / Dockable Side Drawer with Left Dark Resize Handle
        if self.ui.portrait_explorer_drawer {
            let width_id = egui::Id::new("portrait_explorer_width");
            let max_w = (screen_rect.width() - 28.0).max(200.0);
            let default_w = (screen_rect.width() * 0.42).clamp(185.0, 340.0);
            let mut drawer_w = ctx.data(|d| d.get_temp::<f32>(width_id).unwrap_or(default_w)).clamp(160.0, max_w);
            let popup_h = (content_rect.height() - 12.0).clamp(240.0, 680.0);
            let pin_id = egui::Id::new("portrait_explorer_pinned");
            let pinned = ctx.data(|d| d.get_temp::<bool>(pin_id).unwrap_or(true));
            let pinned_pos = egui::pos2(screen_rect.right() - drawer_w - 4.0, content_rect.top() + 4.0);

            let mut win = egui::Window::new("Explorer & Properties")
                .id(egui::Id::new("portrait_floating_explorer"))
                .title_bar(false)
                .resizable(false)
                .movable(!pinned)
                .fixed_size(egui::vec2(drawer_w, popup_h))
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(Color32::from_rgba_unmultiplied(30, 32, 38, 236))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(70, 74, 84)))
                        .inner_margin(egui::Margin::symmetric(6, 6)),
                );
            if pinned {
                win = win.current_pos(pinned_pos);
            } else {
                win = win.default_pos(pinned_pos);
            }
            win.show(ctx, |ui| {
                ui.set_max_width(drawer_w - 12.0);
                ui.horizontal(|ui| {
                    // Left dark vertical resize bar with 5 grip dots (Mobile Studio v2.0 style)
                    let handle_w = 12.0;
                    let (handle_rect, handle_resp) = ui.allocate_exact_size(
                        egui::vec2(handle_w, (popup_h - 14.0).max(140.0)),
                        egui::Sense::drag(),
                    );
                    if handle_resp.hovered() || handle_resp.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if handle_resp.dragged() {
                        drawer_w = (drawer_w - handle_resp.drag_delta().x).clamp(160.0, max_w);
                        ctx.data_mut(|d| d.insert_temp(width_id, drawer_w));
                    }
                    let bar_col = if handle_resp.dragged() {
                        theme::SELECT.gamma_multiply(0.85)
                    } else if handle_resp.hovered() {
                        Color32::from_rgb(28, 31, 38)
                    } else {
                        Color32::from_rgb(15, 16, 19)
                    };
                    ui.painter().rect_filled(handle_rect, 2.0, bar_col);
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
                        let inner_w = (drawer_w - handle_w - 22.0).max(120.0);
                        ui.set_width(inner_w);
                        ui.set_max_width(inner_w);

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Explorer").strong().size(12.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("▶").on_hover_text("Thu gọn").clicked() {
                                    self.ui.portrait_explorer_drawer = false;
                                }
                                if ui
                                    .selectable_label(pinned, "Pin")
                                    .on_hover_text("Ghim / Bỏ ghim để kéo di chuyển")
                                    .clicked()
                                {
                                    ctx.data_mut(|d| d.insert_temp(pin_id, !pinned));
                                }
                            });
                        });
                        ui.separator();

                        let total_h = (ui.available_height() - 18.0).max(160.0);
                        let split = self.ui.portrait_drawer_split.clamp(0.2, 0.8);
                        let top_h = (total_h * split).max(80.0);
                        let bot_h = (total_h - top_h).max(80.0);

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
                            theme::TEXT_DIM,
                        );

                        ui.allocate_ui_with_layout(
                            egui::vec2(inner_w, bot_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_max_width(inner_w);
                                ui.label(egui::RichText::new("Properties").strong().size(12.0));
                                self.properties_ui(ui);
                            },
                        );
                    });
                });
            });
        } else {
            egui::Area::new(egui::Id::new("portrait_drawer_handle"))
                .anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-2.0, 0.0))
                .show(ctx, |ui| {
                    if ui
                        .add_sized([28.0, 48.0], egui::Button::new("◀"))
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

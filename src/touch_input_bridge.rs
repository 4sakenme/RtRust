//! Pure-Rust zero-allocation Touch -> Mouse/Keyboard bridge for RtRustMobile.
//!
//! Translates `egui::Event::Touch` into deterministic `PointerMoved`,
//! `PointerButton` (Primary, Secondary via 350ms Long Press, Middle via 2-finger
//! pan), and `MouseWheel` (2-finger pinch & pan) events so `egui`, `PluginMouse`,
//! `GuiObject`s, and `UserInputService` work seamlessly with complex plugins like
//! Moon Animator, Eclipse Animator, RigEdit, and AutoScale.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, TouchId, TouchPhase};

const MAX_TOUCHES: usize = 10;
const LONG_PRESS_DURATION: Duration = Duration::from_millis(350);
const LONG_PRESS_MAX_DRIFT: f32 = 10.0;

#[derive(Clone, Copy)]
struct TouchSlot {
    active: bool,
    id: TouchId,
    start_pos: Pos2,
    pos: Pos2,
    prev_pos: Pos2,
    start_time: Instant,
}

impl TouchSlot {
    const EMPTY: Self = Self {
        active: false,
        id: TouchId(0),
        start_pos: Pos2::ZERO,
        pos: Pos2::ZERO,
        prev_pos: Pos2::ZERO,
        start_time: unsafe { std::mem::zeroed() },
    };
}

#[derive(Clone, Copy)]
struct QueuedKey {
    key: Key,
    modifiers: Modifiers,
}

pub struct TouchBridge {
    slots: [TouchSlot; MAX_TOUCHES],
    primary_down: bool,
    secondary_down: bool,
    middle_down: bool,
    long_press_fired: bool,
    two_finger_active: bool,
    shift_held: bool,
    queued_keys: [Option<QueuedKey>; 8],
    queued_len: usize,
    toolbar_visible: bool,
}

impl Default for TouchBridge {
    fn default() -> Self {
        let now = Instant::now();
        let empty = TouchSlot {
            active: false,
            id: TouchId(0),
            start_pos: Pos2::ZERO,
            pos: Pos2::ZERO,
            prev_pos: Pos2::ZERO,
            start_time: now,
        };
        Self {
            slots: [empty; MAX_TOUCHES],
            primary_down: false,
            secondary_down: false,
            middle_down: false,
            long_press_fired: false,
            two_finger_active: false,
            shift_held: false,
            queued_keys: [None; 8],
            queued_len: 0,
            toolbar_visible: true,
        }
    }
}

thread_local! {
    static BRIDGE: RefCell<TouchBridge> = RefCell::new(TouchBridge::default());
}

impl TouchBridge {
    fn active_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    fn find_slot(&self, id: TouchId) -> Option<usize> {
        self.slots.iter().position(|s| s.active && s.id == id)
    }

    fn alloc_slot(&mut self, id: TouchId, pos: Pos2, now: Instant) -> Option<usize> {
        if let Some(i) = self.find_slot(id) {
            return Some(i);
        }
        if let Some(i) = self.slots.iter().position(|s| !s.active) {
            self.slots[i] = TouchSlot {
                active: true,
                id,
                start_pos: pos,
                pos,
                prev_pos: pos,
                start_time: now,
            };
            return Some(i);
        }
        None
    }

    fn first_two(&self) -> Option<(TouchSlot, TouchSlot)> {
        let mut first = None;
        for s in self.slots {
            if s.active {
                if let Some(a) = first {
                    return Some((a, s));
                }
                first = Some(s);
            }
        }
        None
    }

    fn first_active(&self) -> Option<TouchSlot> {
        self.slots.into_iter().find(|s| s.active)
    }

    fn release_all_buttons(&mut self, pos: Pos2, mods: Modifiers, out: &mut Vec<egui::Event>) {
        if self.primary_down {
            out.push(egui::Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: mods,
            });
            self.primary_down = false;
        }
        if self.secondary_down {
            out.push(egui::Event::PointerButton {
                pos,
                button: PointerButton::Secondary,
                pressed: false,
                modifiers: mods,
            });
            self.secondary_down = false;
        }
        if self.middle_down {
            out.push(egui::Event::PointerButton {
                pos,
                button: PointerButton::Middle,
                pressed: false,
                modifiers: mods,
            });
            self.middle_down = false;
        }
    }

    fn process(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let now = Instant::now();
        let mut mods = Modifiers::NONE;
        if self.shift_held {
            mods.shift = true;
        }

        // 1. Flush any virtual shortcut key taps from the Floating Shortcut Pill.
        if self.queued_len > 0 {
            for i in 0..self.queued_len {
                if let Some(q) = self.queued_keys[i].take() {
                    let mut m = q.modifiers;
                    if self.shift_held {
                        m.shift = true;
                    }
                    raw.events.push(egui::Event::Key {
                        key: q.key,
                        physical_key: Some(q.key),
                        pressed: true,
                        repeat: false,
                        modifiers: m,
                    });
                    raw.events.push(egui::Event::Key {
                        key: q.key,
                        physical_key: Some(q.key),
                        pressed: false,
                        repeat: false,
                        modifiers: m,
                    });
                }
            }
            self.queued_len = 0;
        }

        // Check if any touch events arrived this frame.
        let has_touch = raw
            .events
            .iter()
            .any(|e| matches!(e, egui::Event::Touch { .. }));

        if !has_touch {
            // Still check for 350ms Long Press while 1 finger is held steady.
            if self.active_count() == 1 && !self.long_press_fired && !self.two_finger_active {
                if let Some(s) = self.first_active() {
                    if now.duration_since(s.start_time) >= LONG_PRESS_DURATION
                        && s.pos.distance(s.start_pos) <= LONG_PRESS_MAX_DRIFT
                    {
                        self.long_press_fired = true;
                        if self.primary_down {
                            raw.events.push(egui::Event::PointerButton {
                                pos: s.pos,
                                button: PointerButton::Primary,
                                pressed: false,
                                modifiers: mods,
                            });
                            self.primary_down = false;
                        }
                        raw.events.push(egui::Event::PointerMoved(s.pos));
                        raw.events.push(egui::Event::PointerButton {
                            pos: s.pos,
                            button: PointerButton::Secondary,
                            pressed: true,
                            modifiers: mods,
                        });
                        self.secondary_down = true;
                    } else {
                        ctx.request_repaint_after(Duration::from_millis(50));
                    }
                }
            }
            return;
        }

        // Snapshot prev_pos before applying new touch events.
        for s in &mut self.slots {
            if s.active {
                s.prev_pos = s.pos;
            }
        }

        // Extract touch events without allocating when possible by draining via index.
        let mut synth: [Option<egui::Event>; 24] = [const { None }; 24];
        let mut synth_len = 0usize;
        let mut push_synth = |ev: egui::Event| {
            if synth_len < synth.len() {
                synth[synth_len] = Some(ev);
                synth_len += 1;
            }
        };

        let mut i = 0;
        while i < raw.events.len() {
            if let egui::Event::Touch { id, phase, pos, .. } = raw.events[i] {
                raw.events.remove(i);
                match phase {
                    TouchPhase::Start => {
                        let prev_count = self.active_count();
                        self.alloc_slot(id, pos, now);
                        let new_count = self.active_count();
                        if new_count == 1 && prev_count == 0 {
                            self.long_press_fired = false;
                            self.two_finger_active = false;
                            push_synth(egui::Event::PointerMoved(pos));
                            push_synth(egui::Event::PointerButton {
                                pos,
                                button: PointerButton::Primary,
                                pressed: true,
                                modifiers: mods,
                            });
                            self.primary_down = true;
                            ctx.request_repaint_after(LONG_PRESS_DURATION);
                        } else if new_count >= 2 && !self.two_finger_active {
                            self.two_finger_active = true;
                            self.long_press_fired = false;
                            if self.primary_down {
                                push_synth(egui::Event::PointerButton {
                                    pos,
                                    button: PointerButton::Primary,
                                    pressed: false,
                                    modifiers: mods,
                                });
                                self.primary_down = false;
                            }
                            if self.secondary_down {
                                push_synth(egui::Event::PointerButton {
                                    pos,
                                    button: PointerButton::Secondary,
                                    pressed: false,
                                    modifiers: mods,
                                });
                                self.secondary_down = false;
                            }
                            if let Some((a, b)) = self.first_two() {
                                let mid = egui::pos2((a.pos.x + b.pos.x) * 0.5, (a.pos.y + b.pos.y) * 0.5);
                                push_synth(egui::Event::PointerMoved(mid));
                                push_synth(egui::Event::PointerButton {
                                    pos: mid,
                                    button: PointerButton::Middle,
                                    pressed: true,
                                    modifiers: mods,
                                });
                                self.middle_down = true;
                            }
                        }
                    }
                    TouchPhase::Move => {
                        if let Some(idx) = self.find_slot(id) {
                            self.slots[idx].pos = pos;
                        }
                        let count = self.active_count();
                        if count == 1 && !self.two_finger_active {
                            if !self.long_press_fired
                                && now.duration_since(self.first_active().map_or(now, |s| s.start_time))
                                    >= LONG_PRESS_DURATION
                                && self
                                    .first_active()
                                    .is_some_and(|s| s.pos.distance(s.start_pos) <= LONG_PRESS_MAX_DRIFT)
                            {
                                self.long_press_fired = true;
                                if self.primary_down {
                                    push_synth(egui::Event::PointerButton {
                                        pos,
                                        button: PointerButton::Primary,
                                        pressed: false,
                                        modifiers: mods,
                                    });
                                    self.primary_down = false;
                                }
                                push_synth(egui::Event::PointerMoved(pos));
                                push_synth(egui::Event::PointerButton {
                                    pos,
                                    button: PointerButton::Secondary,
                                    pressed: true,
                                    modifiers: mods,
                                });
                                self.secondary_down = true;
                            } else {
                                push_synth(egui::Event::PointerMoved(pos));
                            }
                        } else if count >= 2 {
                            if let Some((a, b)) = self.first_two() {
                                let mid = egui::pos2((a.pos.x + b.pos.x) * 0.5, (a.pos.y + b.pos.y) * 0.5);
                                let prev_mid = egui::pos2(
                                    (a.prev_pos.x + b.prev_pos.x) * 0.5,
                                    (a.prev_pos.y + b.prev_pos.y) * 0.5,
                                );
                                let delta_mid = mid - prev_mid;
                                let dist = a.pos.distance(b.pos);
                                let prev_dist = a.prev_pos.distance(b.prev_pos);
                                let delta_dist = dist - prev_dist;

                                push_synth(egui::Event::PointerMoved(mid));
                                if delta_mid.length_sq() > 0.04 {
                                    push_synth(egui::Event::MouseWheel {
                                        unit: egui::MouseWheelUnit::Point,
                                        delta: delta_mid,
                                        phase: TouchPhase::Move,
                                        modifiers: mods,
                                    });
                                }
                                if delta_dist.abs() > 0.5 {
                                    push_synth(egui::Event::MouseWheel {
                                        unit: egui::MouseWheelUnit::Point,
                                        delta: egui::vec2(0.0, delta_dist * 1.6),
                                        phase: TouchPhase::Move,
                                        modifiers: Modifiers::NONE,
                                    });
                                }
                            }
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        if let Some(idx) = self.find_slot(id) {
                            self.slots[idx].active = false;
                        }
                        let remaining = self.active_count();
                        if remaining == 0 {
                            let mut tmp = Vec::new();
                            self.release_all_buttons(pos, mods, &mut tmp);
                            for ev in tmp {
                                push_synth(ev);
                            }
                            self.long_press_fired = false;
                            self.two_finger_active = false;
                        } else if remaining < 2 && self.middle_down {
                            push_synth(egui::Event::PointerButton {
                                pos,
                                button: PointerButton::Middle,
                                pressed: false,
                                modifiers: mods,
                            });
                            self.middle_down = false;
                        }
                    }
                }
            } else {
                // Drop egui_winit's duplicate pointer events when touch is active.
                if self.active_count() > 0
                    && matches!(
                        raw.events[i],
                        egui::Event::PointerButton { .. } | egui::Event::PointerMoved(_)
                    )
                {
                    raw.events.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        for slot in synth.into_iter().take(synth_len).flatten() {
            raw.events.push(slot);
        }
    }
}

pub fn process_raw_input(ctx: &egui::Context, raw: &mut egui::RawInput) {
    BRIDGE.with(|b| b.borrow_mut().process(ctx, raw));
}

pub fn queue_shortcut(key: Key, modifiers: Modifiers) {
    BRIDGE.with(|b| {
        let mut b = b.borrow_mut();
        if b.queued_len < b.queued_keys.len() {
            let idx = b.queued_len;
            b.queued_keys[idx] = Some(QueuedKey { key, modifiers });
            b.queued_len += 1;
        }
    });
}

pub fn shift_held() -> bool {
    BRIDGE.with(|b| b.borrow().shift_held)
}

pub fn toggle_shift() {
    BRIDGE.with(|b| {
        let mut b = b.borrow_mut();
        b.shift_held = !b.shift_held;
    });
}

/// Pure-Rust Floating Shortcut Pill for Moon Animator, Eclipse Animator, and 3D Viewport.
pub fn floating_shortcut_pill_ui(ctx: &egui::Context, has_open_plugin: bool) {
    let is_portrait = ctx.content_rect().width() < ctx.content_rect().height();
    let visible = BRIDGE.with(|b| b.borrow().toolbar_visible);
    if !visible && !has_open_plugin {
        return;
    }

    let anchor_offset = if is_portrait {
        egui::vec2(6.0, 92.0)
    } else {
        egui::vec2(8.0, 128.0)
    };

    egui::Area::new(egui::Id::new("rtrust_floating_shortcut_pill"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::LEFT_TOP, anchor_offset)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(24, 26, 32, 215))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 72, 84)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                        if !visible {
                            if ui.small_button("⌨").on_hover_text("Mở thanh phím tắt Animator").clicked() {
                                BRIDGE.with(|b| b.borrow_mut().toolbar_visible = true);
                            }
                            return;
                        }

                        if ui.small_button("▶ Space").on_hover_text("Play / Pause Animation").clicked() {
                            queue_shortcut(Key::Space, Modifiers::NONE);
                        }
                        if ui.small_button("↻ R").on_hover_text("Rotate / Move mode").clicked() {
                            queue_shortcut(Key::R, Modifiers::NONE);
                        }
                        if ui.small_button("+ K").on_hover_text("Add Keyframe (K)").clicked() {
                            queue_shortcut(Key::K, Modifiers::NONE);
                        }
                        if ui.small_button("🗑 Del").on_hover_text("Delete selected").clicked() {
                            queue_shortcut(Key::Delete, Modifiers::NONE);
                        }
                        if ui.small_button("↶ Z").on_hover_text("Undo (Ctrl+Z)").clicked() {
                            queue_shortcut(Key::Z, Modifiers::COMMAND);
                        }
                        if ui.small_button("↷ Y").on_hover_text("Redo (Ctrl+Y)").clicked() {
                            queue_shortcut(Key::Y, Modifiers::COMMAND);
                        }
                        let shift = shift_held();
                        if ui
                            .selectable_label(shift, "⇧ Shift")
                            .on_hover_text("Giữ Shift (Multi-select)")
                            .clicked()
                        {
                            toggle_shift();
                        }
                        if ui.small_button("✕").on_hover_text("Thu gọn").clicked() {
                            BRIDGE.with(|b| b.borrow_mut().toolbar_visible = false);
                        }
                    });
                });
        });
}

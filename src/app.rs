use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use eframe::egui::{
    self, Color32, Key, KeyboardShortcut, Modifiers, PointerButton, Rect, Sense, Vec2,
};
use glam::Vec2 as ModelVec2;
use serde::{Deserialize, Serialize};

use crate::{
    BodyPart, BrushSize, Camera, LayerVisibility, ModelHit, ModelKind, Skin, SkinDocument,
    StrokeBuilder, pick_model_part,
    renderer::{ModelPaintCallback, ModelRenderer, TextureUpdate},
};

const DEFAULT_PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 255],
    [64, 64, 64, 255],
    [128, 128, 128, 255],
    [192, 192, 192, 255],
    [255, 255, 255, 255],
    [136, 0, 21, 255],
    [237, 28, 36, 255],
    [255, 127, 39, 255],
    [255, 242, 0, 255],
    [34, 177, 76, 255],
    [0, 162, 232, 255],
    [63, 72, 204, 255],
    [163, 73, 164, 255],
    [185, 122, 87, 255],
    [255, 174, 201, 255],
    [0, 0, 0, 0],
];
const PALETTE_COLUMNS: usize = 5;
const PALETTE_SWATCH_SIZE: f32 = 28.0;
const PALETTE_SPACING: f32 = 4.0;
const TOOLS_HORIZONTAL_MARGIN: i8 = 12;
const APP_STATE_VERSION: u32 = 1;

const NEW_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const OPEN_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const COMMAND_SHIFT: Modifiers = Modifiers {
    shift: true,
    command: true,
    ..Modifiers::NONE
};
const SAVE_AS_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(COMMAND_SHIFT, Key::S);
const UNDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(COMMAND_SHIFT, Key::Z);
const REDO_ALTERNATE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentAction {
    New(ModelKind),
    Open,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfirmationChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewGesture {
    Idle,
    Paint,
    Orbit,
    Solo,
    Sample,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorControlTab {
    Hsv,
    Rgba,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PaintTool {
    Brush,
    Fill,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PersistedAppState {
    version: u32,
    palette: [[u8; 4]; 16],
}

pub struct SkinDrawApp {
    document: SkinDocument,
    kind: ModelKind,
    camera: Camera,
    visibility: LayerVisibility,
    tool: PaintTool,
    brush_size: BrushSize,
    active_color: [u8; 4],
    palette: [[u8; 4]; 16],
    color_tab: ColorControlTab,
    hex_color: String,
    hex_color_invalid: bool,
    active_stroke: Option<StrokeBuilder>,
    hovered_hit: Option<ModelHit>,
    solo_part: Option<BodyPart>,
    uploaded_skin: Skin,
    pending_action: Option<DocumentAction>,
    error_message: Option<String>,
    status_message: Option<String>,
    allow_close: bool,
}

impl SkinDrawApp {
    pub fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let document = SkinDocument::new(ModelKind::Classic);
        let render_state = creation
            .wgpu_render_state
            .as_ref()
            .expect("SkinDraw requires the WGPU renderer");
        ModelRenderer::install(render_state, document.skin());
        let mut app = Self::from_document(document, ModelKind::Classic);
        if let Some(path) = app_state_path() {
            app.palette = load_palette(&path).unwrap_or(DEFAULT_PALETTE);
        }
        app
    }

    fn from_document(document: SkinDocument, kind: ModelKind) -> Self {
        Self {
            uploaded_skin: document.skin().clone(),
            document,
            kind,
            camera: Camera {
                yaw: -0.45,
                pitch: 0.18,
                ..Default::default()
            },
            visibility: LayerVisibility::ALL,
            tool: PaintTool::Brush,
            brush_size: BrushSize::One,
            active_color: [237, 28, 36, 255],
            palette: DEFAULT_PALETTE,
            color_tab: ColorControlTab::Hsv,
            hex_color: format_hex_color([237, 28, 36, 255]),
            hex_color_invalid: false,
            active_stroke: None,
            hovered_hit: None,
            solo_part: None,
            pending_action: None,
            error_message: None,
            status_message: None,
            allow_close: false,
        }
    }

    fn toolbar(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let mut requested_action = None;
        let mut save = false;
        let mut save_as = false;
        egui::Panel::top("document_toolbar")
            .resizable(false)
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("New Classic").clicked() {
                        requested_action = Some(DocumentAction::New(ModelKind::Classic));
                    }
                    if ui.button("New Slim").clicked() {
                        requested_action = Some(DocumentAction::New(ModelKind::Slim));
                    }
                    ui.separator();
                    if ui.button("Open…").clicked() {
                        requested_action = Some(DocumentAction::Open);
                    }
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                    if ui.button("Save As…").clicked() {
                        save_as = true;
                    }
                    ui.separator();
                    let mut name = self
                        .document
                        .path()
                        .and_then(|path| path.file_name())
                        .map_or_else(
                            || "Untitled.png".to_owned(),
                            |name| name.to_string_lossy().into(),
                        );
                    if self.document.is_dirty() {
                        name.push_str(" •");
                    }
                    ui.label(name);
                    if let Some(status) = &self.status_message {
                        ui.separator();
                        ui.colored_label(Color32::LIGHT_GREEN, status);
                    }
                });
            });
        if let Some(action) = requested_action {
            self.request_action(action, ctx);
        } else if save_as {
            self.save_as();
        } else if save {
            self.save();
        }
    }

    fn sidebar(&mut self, root: &mut egui::Ui) -> f32 {
        let content_width = tools_content_width(root);
        let panel_width = content_width + f32::from(TOOLS_HORIZONTAL_MARGIN) * 2.0;
        let panel_frame = egui::Frame::side_top_panel(root.style())
            .inner_margin(egui::Margin::symmetric(TOOLS_HORIZONTAL_MARGIN, 2));
        let mut panel_left = root.max_rect().right();
        egui::Panel::right("tools")
            .default_size(panel_width)
            .min_size(panel_width)
            .max_size(panel_width)
            .frame(panel_frame)
            .resizable(false)
            .show_separator_line(false)
            .show(root, |ui| {
                panel_left = ui.clip_rect().left();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.heading("SkinDraw");
                        ui.add_space(8.0);

                        ui.label("Arm model");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.kind, ModelKind::Classic, "Classic");
                            ui.selectable_value(&mut self.kind, ModelKind::Slim, "Slim");
                        });
                        ui.add_space(10.0);

                        ui.label("Visible geometry");
                        ui.checkbox(&mut self.visibility.base, "Base");
                        ui.checkbox(&mut self.visibility.outer, "Outer");
                        if !self.visibility.base && !self.visibility.outer {
                            ui.colored_label(Color32::LIGHT_RED, "Enable a layer to paint.");
                        }
                        ui.add_space(10.0);

                        ui.label("Tool");
                        let mut requested_tool = self.tool;
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut requested_tool, PaintTool::Brush, "Brush (B)");
                            ui.selectable_value(&mut requested_tool, PaintTool::Fill, "Fill (F)");
                        });
                        if requested_tool != self.tool {
                            self.set_tool(requested_tool);
                        }
                        ui.add_space(6.0);

                        ui.label("Brush size");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.brush_size, BrushSize::One, "1");
                            ui.selectable_value(&mut self.brush_size, BrushSize::Two, "2");
                            ui.selectable_value(&mut self.brush_size, BrushSize::Four, "4");
                        });
                        ui.add_space(10.0);

                        ui.label("Palette");
                        egui::Grid::new("palette")
                            .spacing(Vec2::splat(4.0))
                            .show(ui, |ui| {
                                for index in 0..self.palette.len() {
                                    let color = self.palette[index];
                                    let selected = color == self.active_color;
                                    let response = color_swatch(
                                        ui,
                                        Vec2::splat(PALETTE_SWATCH_SIZE),
                                        color,
                                        Sense::click(),
                                        selected,
                                    )
                                    .on_hover_text("Click to select; Shift-click to replace");
                                    if response.clicked() {
                                        if ui.input(|input| input.modifiers.shift) {
                                            self.store_palette_color(index);
                                        } else {
                                            self.set_active_color(color);
                                        }
                                    }
                                    if (index + 1) % PALETTE_COLUMNS == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                        ui.add_space(10.0);

                        ui.label("Color");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.color_tab, ColorControlTab::Hsv, "HSV");
                            ui.selectable_value(
                                &mut self.color_tab,
                                ColorControlTab::Rgba,
                                "RGBA / Hex",
                            );
                        });
                        ui.add_space(4.0);
                        match self.color_tab {
                            ColorControlTab::Hsv => self.hsv_color_controls(ui),
                            ColorControlTab::Rgba => self.rgba_color_controls(ui),
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Active");
                            color_swatch(
                                ui,
                                Vec2::new(72.0, 24.0),
                                self.active_color,
                                Sense::hover(),
                                false,
                            );
                        });

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    self.document.undo_len() > 0,
                                    egui::Button::new("Undo"),
                                )
                                .clicked()
                            {
                                self.finish_stroke();
                                self.document.undo();
                                self.status_message = None;
                            }
                            if ui
                                .add_enabled(
                                    self.document.redo_len() > 0,
                                    egui::Button::new("Redo"),
                                )
                                .clicked()
                            {
                                self.finish_stroke();
                                self.document.redo();
                                self.status_message = None;
                            }
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.label("Brush: primary-button drag");
                        ui.label("Fill: primary-button click");
                        ui.label("Sample: secondary-button click");
                        ui.label("Orbit: Shift + primary drag");
                        if let Some(part) = self.solo_part {
                            ui.label(format!("Solo: {part:?} (Escape to exit)"));
                        } else {
                            ui.label("Solo: Ctrl + primary click");
                        }
                        if let Some(hit) = self.hovered_hit {
                            ui.add_space(8.0);
                            ui.monospace(format!(
                                "{:?} · {:?} · {:?}\ntexel {}, {}",
                                hit.part, hit.layer, hit.face, hit.texel.x, hit.texel.y
                            ));
                        }
                    });
            });
        panel_left
    }

    fn model_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tools_left: f32) {
        let mut rect = ui.available_rect_before_wrap();
        rect.max.x = rect.max.x.min(tools_left);
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let (shift, control, primary_down, primary_pressed, secondary_pressed) =
            ctx.input(|input| {
                (
                    input.modifiers.shift,
                    input.modifiers.ctrl,
                    input.pointer.button_down(PointerButton::Primary)
                        || input.pointer.button_pressed(PointerButton::Primary),
                    input.pointer.button_pressed(PointerButton::Primary),
                    input.pointer.button_pressed(PointerButton::Secondary),
                )
            });
        let primary_released =
            ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

        let pointer_in_view = response.hovered()
            || response.clicked_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Primary);

        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        self.hovered_hit = pointer.and_then(|position| self.hit_at(rect, position));

        let gesture = view_gesture(
            pointer_in_view,
            primary_down,
            primary_pressed,
            secondary_pressed,
            shift,
            control,
        );
        match gesture {
            ViewGesture::Orbit => {
                self.finish_stroke();
                let delta = ctx.input(|input| input.pointer.delta());
                self.camera.orbit(-delta.x * 0.012, delta.y * 0.012);
            }
            ViewGesture::Paint => {
                if let Some(hit) = self.hovered_hit {
                    match self.tool {
                        PaintTool::Brush => {
                            let stroke = self.active_stroke.get_or_insert_with(StrokeBuilder::new);
                            self.document.paint(
                                stroke,
                                self.kind,
                                hit,
                                self.brush_size,
                                self.active_color,
                            );
                            self.status_message = None;
                        }
                        PaintTool::Fill if primary_pressed => {
                            self.finish_stroke();
                            self.document.flood_fill(self.kind, hit, self.active_color);
                            self.status_message = None;
                        }
                        PaintTool::Fill => {}
                    }
                }
            }
            ViewGesture::Solo => {
                if let Some(hit) = self.hovered_hit {
                    self.enter_solo(hit.part);
                }
            }
            ViewGesture::Sample => {
                self.finish_stroke();
                if let Some(hit) = self.hovered_hit {
                    self.sample_color(hit);
                    self.status_message = None;
                }
            }
            ViewGesture::Idle => {}
        }
        if primary_released {
            self.finish_stroke();
        }

        let texture_update = TextureUpdate::between(&self.uploaded_skin, self.document.skin());
        let skin = Arc::new(self.document.skin().clone());
        if texture_update.is_some() {
            self.uploaded_skin = (*skin).clone();
        }
        ui.painter().add(
            ModelPaintCallback {
                rect,
                kind: self.kind,
                visibility: self.visibility,
                camera: self.camera,
                skin,
                texture_update,
                preview_hit: if gesture == ViewGesture::Orbit {
                    None
                } else {
                    self.hovered_hit
                },
                brush_size: if self.tool == PaintTool::Brush {
                    self.brush_size
                } else {
                    BrushSize::One
                },
                solo_part: self.solo_part,
            }
            .paint_callback(),
        );

        if shift && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        } else if self.hovered_hit.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    fn hit_at(&self, rect: Rect, pointer: egui::Pos2) -> Option<ModelHit> {
        let ray = self.camera.ray_for_pointer(
            ModelVec2::new(pointer.x, pointer.y),
            ModelVec2::new(rect.min.x, rect.min.y),
            ModelVec2::new(rect.width(), rect.height()),
        )?;
        pick_model_part(ray, self.kind, self.visibility, self.solo_part)
    }

    fn finish_stroke(&mut self) {
        if let Some(stroke) = self.active_stroke.take() {
            self.document.commit_stroke(stroke);
        }
    }

    fn set_tool(&mut self, tool: PaintTool) {
        if self.tool != tool {
            self.finish_stroke();
            self.tool = tool;
        }
    }

    fn set_active_color(&mut self, color: [u8; 4]) {
        self.active_color = color;
        self.hex_color = format_hex_color(color);
        self.hex_color_invalid = false;
    }

    fn sample_color(&mut self, hit: ModelHit) {
        self.set_active_color(self.document.skin().pixel(hit.texel));
    }

    fn store_palette_color(&mut self, index: usize) {
        self.assign_palette_color(index);
        let Some(path) = app_state_path() else {
            self.error_message =
                Some("Could not find the home directory for palette state.".into());
            return;
        };
        if let Err(error) = save_palette(&path, self.palette) {
            self.error_message = Some(format!(
                "Could not save palette to {}:\n{error}",
                display_path(&path)
            ));
        }
    }

    fn assign_palette_color(&mut self, index: usize) {
        self.palette[index] = self.active_color;
    }

    fn hsv_color_controls(&mut self, ui: &mut egui::Ui) {
        let mut hsva = egui::ecolor::Hsva::from_srgba_unmultiplied(self.active_color);
        ui.spacing_mut().slider_width = ui.available_width().min(180.0);
        if egui::color_picker::color_picker_hsva_2d(
            ui,
            &mut hsva,
            egui::color_picker::Alpha::OnlyBlend,
        ) {
            self.set_active_color(hsva.to_srgba_unmultiplied());
        }
    }

    fn rgba_color_controls(&mut self, ui: &mut egui::Ui) {
        let mut rgba_changed = false;
        egui::Grid::new("exact_rgba").num_columns(2).show(ui, |ui| {
            for (label, channel) in ["R", "G", "B", "A"]
                .into_iter()
                .zip(self.active_color.iter_mut())
            {
                ui.label(label);
                rgba_changed |= ui
                    .add(egui::DragValue::new(channel).range(0..=255))
                    .changed();
                ui.end_row();
            }
        });
        if rgba_changed {
            self.hex_color = format_hex_color(self.active_color);
            self.hex_color_invalid = false;
        }

        ui.horizontal(|ui| {
            ui.label("Hex");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.hex_color)
                    .desired_width(96.0)
                    .char_limit(9),
            );
            if response.changed() {
                if let Some(color) = parse_hex_color(&self.hex_color) {
                    self.active_color = color;
                    self.hex_color_invalid = false;
                } else {
                    self.hex_color_invalid = true;
                }
            } else if !response.has_focus() && self.hex_color_invalid {
                self.hex_color = format_hex_color(self.active_color);
                self.hex_color_invalid = false;
            }
        });
        if self.hex_color_invalid {
            ui.colored_label(Color32::LIGHT_RED, "Use #RRGGBBAA");
        }
    }

    fn enter_solo(&mut self, part: BodyPart) {
        self.finish_stroke();
        self.solo_part = Some(part);
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.solo_part.is_some()
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.solo_part = None;
        }
        let text_input_focused = ctx.text_edit_focused();
        let tool = ctx.input(|input| {
            tool_shortcut(
                input.key_pressed(Key::B),
                input.key_pressed(Key::F),
                text_input_focused,
                input.modifiers == Modifiers::NONE,
            )
        });
        if let Some(tool) = tool {
            let key = match tool {
                PaintTool::Brush => Key::B,
                PaintTool::Fill => Key::F,
            };
            ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
            self.set_tool(tool);
        }
        if consume(ctx, SAVE_AS_SHORTCUT) {
            self.save_as();
        } else if consume(ctx, SAVE_SHORTCUT) {
            self.save();
        } else if consume(ctx, OPEN_SHORTCUT) {
            self.request_action(DocumentAction::Open, ctx);
        } else if consume(ctx, NEW_SHORTCUT) {
            self.request_action(DocumentAction::New(self.kind), ctx);
        } else if consume(ctx, UNDO_SHORTCUT) {
            self.finish_stroke();
            self.document.undo();
            self.status_message = None;
        } else if consume(ctx, REDO_SHORTCUT) || consume(ctx, REDO_ALTERNATE_SHORTCUT) {
            self.finish_stroke();
            self.document.redo();
            self.status_message = None;
        }
    }

    fn request_action(&mut self, action: DocumentAction, ctx: &egui::Context) {
        self.finish_stroke();
        if self.document.is_dirty() {
            self.pending_action = Some(action);
        } else {
            self.execute_action(action, ctx);
        }
    }

    fn execute_action(&mut self, action: DocumentAction, ctx: &egui::Context) {
        match action {
            DocumentAction::New(kind) => {
                self.replace_document(SkinDocument::new(kind), kind);
                self.status_message = Some(format!("Created a new {kind:?} skin."));
            }
            DocumentAction::Open => self.open(),
            DocumentAction::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn replace_document(&mut self, document: SkinDocument, kind: ModelKind) {
        self.finish_stroke();
        self.document = document;
        self.kind = kind;
        self.hovered_hit = None;
        self.solo_part = None;
        self.status_message = None;
    }

    fn open(&mut self) {
        let Some(path) = file_dialog(self.document.path().map(PathBuf::from)).pick_file() else {
            return;
        };
        self.open_path(path);
    }

    fn open_path(&mut self, path: PathBuf) {
        match SkinDocument::load_png(&path) {
            Ok(document) => {
                self.replace_document(document, self.kind);
                self.status_message = Some(format!("Opened {}.", display_path(&path)));
            }
            Err(error) => {
                self.error_message =
                    Some(format!("Could not open {}:\n{error}", display_path(&path)));
            }
        }
    }

    fn save(&mut self) -> bool {
        self.finish_stroke();
        if let Some(path) = self.document.path().map(PathBuf::from) {
            self.save_to(path)
        } else {
            self.save_as()
        }
    }

    fn save_as(&mut self) -> bool {
        self.finish_stroke();
        let current = self.document.path().map(PathBuf::from);
        let mut dialog = file_dialog(current.clone());
        dialog = dialog.set_file_name(
            current
                .as_deref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("skin.png"),
        );
        let Some(path) = dialog.save_file() else {
            return false;
        };
        self.save_to(ensure_png_extension(path))
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        match self.document.save_png(&path) {
            Ok(()) => {
                self.status_message = Some(format!("Saved {}.", display_path(&path)));
                true
            }
            Err(error) => {
                self.error_message =
                    Some(format!("Could not save {}:\n{error}", display_path(&path)));
                false
            }
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) || self.allow_close {
            return;
        }
        if self.document.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_action = Some(DocumentAction::Quit);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending_action {
            let mut choice = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Save changes to {} before continuing?",
                        self.document
                            .path()
                            .and_then(|path| path.file_name())
                            .map_or_else(
                                || "Untitled.png".to_owned(),
                                |name| name.to_string_lossy().into()
                            )
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            choice = Some(ConfirmationChoice::Save);
                        }
                        if ui.button("Discard").clicked() {
                            choice = Some(ConfirmationChoice::Discard);
                        }
                        if ui.button("Cancel").clicked() {
                            choice = Some(ConfirmationChoice::Cancel);
                        }
                    });
                });
            if let Some(choice) = choice {
                match choice {
                    ConfirmationChoice::Save if self.save() => {
                        self.pending_action = None;
                        self.execute_action(action, ctx);
                    }
                    ConfirmationChoice::Discard => {
                        self.pending_action = None;
                        self.execute_action(action, ctx);
                    }
                    ConfirmationChoice::Cancel => self.pending_action = None,
                    ConfirmationChoice::Save => {}
                }
            }
        }

        if let Some(message) = self.error_message.clone() {
            let mut dismiss = false;
            egui::Window::new("SkinDraw error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.colored_label(Color32::LIGHT_RED, message);
                    ui.add_space(8.0);
                    dismiss = ui.button("OK").clicked();
                });
            if dismiss {
                self.error_message = None;
            }
        }
    }
}

impl eframe::App for SkinDrawApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_close_request(&ctx);
        self.shortcuts(&ctx);
        self.toolbar(ui, &ctx);
        let tools_left = self.sidebar(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.model_view(ui, &ctx, tools_left));
        self.dialogs(&ctx);
        let title = self
            .document
            .path()
            .and_then(|path| path.file_name())
            .map_or_else(
                || "Untitled.png".to_owned(),
                |name| name.to_string_lossy().into(),
            );
        let dirty = if self.document.is_dirty() { " •" } else { "" };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "SkinDraw — {title}{dirty}"
        )));
    }
}

fn consume(ctx: &egui::Context, shortcut: KeyboardShortcut) -> bool {
    ctx.input_mut(|input| input.consume_shortcut(&shortcut))
}

fn view_gesture(
    pointer_in_view: bool,
    primary_down: bool,
    primary_pressed: bool,
    secondary_pressed: bool,
    shift: bool,
    control: bool,
) -> ViewGesture {
    match (
        pointer_in_view,
        primary_down,
        primary_pressed,
        secondary_pressed,
        shift,
        control,
    ) {
        (true, _, _, true, _, _) => ViewGesture::Sample,
        (true, true, true, false, _, true) => ViewGesture::Solo,
        (true, true, _, false, true, false) => ViewGesture::Orbit,
        (true, true, _, false, false, false) => ViewGesture::Paint,
        _ => ViewGesture::Idle,
    }
}

fn tool_shortcut(
    brush_pressed: bool,
    fill_pressed: bool,
    text_input_focused: bool,
    unmodified: bool,
) -> Option<PaintTool> {
    if text_input_focused || !unmodified {
        None
    } else if brush_pressed {
        Some(PaintTool::Brush)
    } else if fill_pressed {
        Some(PaintTool::Fill)
    } else {
        None
    }
}

fn format_hex_color(color: [u8; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color[0], color[1], color[2], color[3]
    )
}

fn parse_hex_color(text: &str) -> Option<[u8; 4]> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 8 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(u32::from_str_radix(digits, 16).ok()?.to_be_bytes())
}

fn color_swatch(
    ui: &mut egui::Ui,
    size: Vec2,
    color: [u8; 4],
    sense: Sense,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let square = 6.0;
    let columns = (rect.width() / square).ceil() as usize;
    let rows = (rect.height() / square).ceil() as usize;
    for row in 0..rows {
        for column in 0..columns {
            let min = rect.min + Vec2::new(column as f32 * square, row as f32 * square);
            let tile = Rect::from_min_max(min, (min + Vec2::splat(square)).min(rect.max));
            let fill = if (row + column) % 2 == 0 {
                Color32::from_gray(82)
            } else {
                Color32::from_gray(132)
            };
            ui.painter().rect_filled(tile, 0.0, fill);
        }
    }
    ui.painter().rect_filled(
        rect,
        3.0,
        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        if selected {
            egui::Stroke::new(2.0, Color32::WHITE)
        } else {
            egui::Stroke::new(1.0, Color32::GRAY)
        },
        egui::StrokeKind::Inside,
    );
    response
}

fn app_state_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/state/skindraw.json"))
}

fn load_palette(path: &Path) -> Option<[[u8; 4]; 16]> {
    let bytes = fs::read(path).ok()?;
    let state: PersistedAppState = serde_json::from_slice(&bytes).ok()?;
    (state.version == APP_STATE_VERSION).then_some(state.palette)
}

fn save_palette(path: &Path, palette: [[u8; 4]; 16]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let state = PersistedAppState {
        version: APP_STATE_VERSION,
        palette,
    };
    let bytes = serde_json::to_vec_pretty(&state).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

fn file_dialog(current: Option<PathBuf>) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new().add_filter("Minecraft skin PNG", &["png"]);
    if let Some(path) = current
        && let Some(directory) = path.parent()
    {
        dialog = dialog.set_directory(directory);
    }
    dialog
}

fn ensure_png_extension(mut path: PathBuf) -> PathBuf {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        path.set_extension("png");
    }
    path
}

fn display_path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn tools_content_width(ui: &egui::Ui) -> f32 {
    let body_font = egui::TextStyle::Body.resolve(ui.style());
    let monospace_font = egui::TextStyle::Monospace.resolve(ui.style());
    let body_text_width = [
        "Enable a layer to paint.",
        "Brush: primary-button drag",
        "Fill: primary-button click",
        "Sample: secondary-button click",
        "Orbit: Shift + primary drag",
        "Solo: RightArm (Escape to exit)",
    ]
    .into_iter()
    .map(|text| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), body_font.clone(), Color32::WHITE)
            .size()
            .x
    })
    .fold(0.0_f32, f32::max);
    let hit_text_width = ui
        .painter()
        .layout_no_wrap(
            "RightArm · Outer · Bottom".to_owned(),
            monospace_font,
            Color32::WHITE,
        )
        .size()
        .x;
    let palette_width = PALETTE_COLUMNS as f32 * PALETTE_SWATCH_SIZE
        + (PALETTE_COLUMNS - 1) as f32 * PALETTE_SPACING;
    body_text_width
        .max(hit_text_width)
        .max(palette_width)
        .ceil()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyPart, Face, Layer, Texel};
    use tempfile::tempdir;

    fn dirty_app() -> SkinDrawApp {
        let mut document = SkinDocument::new(ModelKind::Classic);
        let mut stroke = StrokeBuilder::new();
        document.paint(
            &mut stroke,
            ModelKind::Classic,
            ModelHit {
                part: BodyPart::Head,
                layer: Layer::Base,
                face: Face::Front,
                distance: 1.0,
                texel: Texel::new(8, 8),
            },
            BrushSize::One,
            [1, 2, 3, 4],
        );
        assert!(document.commit_stroke(stroke));
        SkinDrawApp::from_document(document, ModelKind::Classic)
    }

    #[test]
    fn presentation_kind_does_not_change_skin_pixels() {
        let document = SkinDocument::new(ModelKind::Classic);
        let pixels = document.skin().clone();
        let mut kind = ModelKind::Classic;
        assert_eq!(kind, ModelKind::Classic);
        kind = ModelKind::Slim;
        assert_eq!(kind, ModelKind::Slim);
        assert_eq!(document.skin(), &pixels);
        assert!(!document.is_dirty());
    }

    #[test]
    fn palette_has_opaque_choices_and_transparent_eraser() {
        assert!(DEFAULT_PALETTE.iter().any(|color| color[3] == 255));
        assert!(DEFAULT_PALETTE.iter().any(|color| color[3] == 0));
        assert_eq!(DEFAULT_PALETTE.len(), 16);
    }

    #[test]
    fn editable_palette_assignment_is_independent_from_document_state() {
        let document = SkinDocument::new(ModelKind::Classic);
        let mut app = SkinDrawApp::from_document(document, ModelKind::Classic);
        let skin = app.document.skin().clone();
        app.set_active_color([9, 8, 7, 0]);
        app.assign_palette_color(3);
        assert_eq!(app.palette[3], [9, 8, 7, 0]);
        assert_eq!(app.active_color, [9, 8, 7, 0]);
        assert_eq!(app.document.skin(), &skin);
        assert!(!app.document.is_dirty());
        assert_eq!(app.document.undo_len(), 0);
        assert_eq!(app.document.redo_len(), 0);
    }

    #[test]
    fn palette_state_round_trips_and_invalid_state_falls_back() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("state/skindraw.json");
        let mut palette = DEFAULT_PALETTE;
        palette[0] = [1, 2, 3, 4];
        palette[15] = [250, 240, 230, 0];
        save_palette(&path, palette).unwrap();
        assert_eq!(load_palette(&path), Some(palette));
        assert!(!path.with_extension("json.tmp").exists());

        fs::write(&path, b"not json").unwrap();
        assert_eq!(
            load_palette(&path).unwrap_or(DEFAULT_PALETTE),
            DEFAULT_PALETTE
        );
        fs::write(
            &path,
            serde_json::to_vec(&PersistedAppState {
                version: APP_STATE_VERSION + 1,
                palette,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            load_palette(&path).unwrap_or(DEFAULT_PALETTE),
            DEFAULT_PALETTE
        );
    }

    #[test]
    fn exact_hex_rgba_round_trips_and_rejects_invalid_input() {
        for color in [
            [0, 0, 0, 0],
            [255, 255, 255, 255],
            [1, 35, 69, 103],
            [237, 28, 36, 255],
        ] {
            let formatted = format_hex_color(color);
            assert_eq!(parse_hex_color(&formatted), Some(color));
            assert_eq!(parse_hex_color(&formatted[1..]), Some(color));
        }
        for invalid in ["", "#123456", "#123456789", "#GG000000", "#12 45678"] {
            assert_eq!(parse_hex_color(invalid), None);
        }
    }

    #[test]
    fn color_tab_changes_do_not_change_active_rgba() {
        let document = SkinDocument::new(ModelKind::Classic);
        let mut app = SkinDrawApp::from_document(document, ModelKind::Classic);
        app.set_active_color([17, 34, 51, 68]);
        let active = app.active_color;
        app.color_tab = ColorControlTab::Rgba;
        assert_eq!(app.active_color, active);
        app.color_tab = ColorControlTab::Hsv;
        assert_eq!(app.active_color, active);
        assert_eq!(app.hex_color, "#11223344");
    }

    #[test]
    fn sampling_copies_exact_rgba_without_changing_the_document() {
        let texel = Texel::new(8, 8);
        let sampled = [17, 34, 51, 0];
        let mut skin = Skin::blank(ModelKind::Classic);
        skin.set_pixel(texel, sampled);
        let document = SkinDocument::from_skin(skin, None);
        let mut app = SkinDrawApp::from_document(document, ModelKind::Classic);
        let original = app.document.skin().clone();

        app.sample_color(ModelHit {
            part: BodyPart::Head,
            layer: Layer::Base,
            face: Face::Front,
            distance: 1.0,
            texel,
        });

        assert_eq!(app.active_color, sampled);
        assert_eq!(app.hex_color, "#11223300");
        assert!(!app.hex_color_invalid);
        assert_eq!(app.document.skin(), &original);
        assert!(!app.document.is_dirty());
        assert_eq!(app.document.undo_len(), 0);
        assert_eq!(app.document.redo_len(), 0);
    }

    #[test]
    fn brush_controls_cover_every_core_size() {
        assert_eq!(
            [BrushSize::One, BrushSize::Two, BrushSize::Four].map(BrushSize::pixels),
            [1, 2, 4]
        );
        assert_eq!(Texel::new(1, 2).x, 1);
    }

    #[test]
    fn brush_and_fill_shortcuts_ignore_text_input_and_modifiers() {
        assert_eq!(
            tool_shortcut(true, false, false, true),
            Some(PaintTool::Brush)
        );
        assert_eq!(
            tool_shortcut(false, true, false, true),
            Some(PaintTool::Fill)
        );
        assert_eq!(tool_shortcut(true, false, true, true), None);
        assert_eq!(tool_shortcut(false, true, true, true), None);
        assert_eq!(tool_shortcut(true, false, false, false), None);
        assert_eq!(tool_shortcut(false, false, false, true), None);
    }

    #[test]
    fn changing_tools_commits_an_active_brush_stroke() {
        let document = SkinDocument::new(ModelKind::Classic);
        let mut app = SkinDrawApp::from_document(document, ModelKind::Classic);
        let mut stroke = StrokeBuilder::new();
        app.document.paint(
            &mut stroke,
            ModelKind::Classic,
            ModelHit {
                part: BodyPart::Head,
                layer: crate::Layer::Base,
                face: crate::Face::Front,
                distance: 1.0,
                texel: Texel::new(8, 8),
            },
            BrushSize::One,
            [1, 2, 3, 4],
        );
        app.active_stroke = Some(stroke);
        assert_eq!(app.document.undo_len(), 0);
        app.set_tool(PaintTool::Fill);
        assert_eq!(app.tool, PaintTool::Fill);
        assert!(app.active_stroke.is_none());
        assert_eq!(app.document.undo_len(), 1);
    }

    #[test]
    fn model_view_gestures_separate_paint_orbit_and_outside_input() {
        assert_eq!(
            view_gesture(true, true, true, false, false, false),
            ViewGesture::Paint
        );
        assert_eq!(
            view_gesture(true, true, true, false, true, false),
            ViewGesture::Orbit
        );
        assert_eq!(
            view_gesture(false, true, true, false, false, false),
            ViewGesture::Idle
        );
        assert_eq!(
            view_gesture(false, true, true, false, true, false),
            ViewGesture::Idle
        );
        assert_eq!(
            view_gesture(true, false, false, false, true, false),
            ViewGesture::Idle
        );

        let drag_transition = [false, false, true, true, false]
            .map(|shift| view_gesture(true, true, false, false, shift, false));
        assert_eq!(
            drag_transition,
            [
                ViewGesture::Paint,
                ViewGesture::Paint,
                ViewGesture::Orbit,
                ViewGesture::Orbit,
                ViewGesture::Paint,
            ]
        );
    }

    #[test]
    fn control_press_solos_once_and_consumes_the_held_drag() {
        assert_eq!(
            view_gesture(true, true, true, false, false, true),
            ViewGesture::Solo
        );
        assert_eq!(
            view_gesture(true, true, false, false, false, true),
            ViewGesture::Idle
        );
        assert_eq!(
            view_gesture(true, true, true, false, true, true),
            ViewGesture::Solo
        );
    }

    #[test]
    fn secondary_press_samples_once_and_only_inside_the_model_view() {
        assert_eq!(
            view_gesture(true, false, false, true, false, false),
            ViewGesture::Sample
        );
        assert_eq!(
            view_gesture(true, false, false, false, false, false),
            ViewGesture::Idle
        );
        assert_eq!(
            view_gesture(false, false, false, true, false, false),
            ViewGesture::Idle
        );
        assert_eq!(
            view_gesture(true, true, true, true, true, true),
            ViewGesture::Sample
        );
    }

    #[test]
    fn solo_mode_is_document_independent_presentation_state() {
        let document = SkinDocument::new(ModelKind::Classic);
        let mut app = SkinDrawApp::from_document(document, ModelKind::Classic);
        let skin = app.document.skin().clone();
        app.enter_solo(BodyPart::RightArm);
        assert_eq!(app.solo_part, Some(BodyPart::RightArm));
        assert_eq!(app.document.skin(), &skin);
        assert!(!app.document.is_dirty());
        assert_eq!(app.document.undo_len(), 0);
        assert_eq!(app.document.redo_len(), 0);
        app.solo_part = None;
        assert_eq!(app.document.skin(), &skin);
        assert!(!app.document.is_dirty());
    }

    #[test]
    fn dirty_replacement_is_guarded_and_discard_replaces_atomically() {
        let mut app = dirty_app();
        app.solo_part = Some(BodyPart::Head);
        app.palette[0] = [11, 22, 33, 44];
        let changed = app.document.skin().clone();
        let ctx = egui::Context::default();
        app.request_action(DocumentAction::New(ModelKind::Slim), &ctx);
        assert_eq!(
            app.pending_action,
            Some(DocumentAction::New(ModelKind::Slim))
        );
        assert_eq!(app.document.skin(), &changed);
        assert_eq!(app.kind, ModelKind::Classic);

        app.pending_action = None;
        app.execute_action(DocumentAction::New(ModelKind::Slim), &ctx);
        assert_eq!(app.kind, ModelKind::Slim);
        assert_eq!(app.solo_part, None);
        assert_eq!(app.palette[0], [11, 22, 33, 44]);
        assert!(!app.document.is_dirty());
        assert_ne!(app.document.skin(), &changed);
        assert_eq!(app.document.undo_len(), 0);
    }

    #[test]
    fn save_and_reopen_preserve_rgba_and_clean_state() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("saved-skin.png");
        let mut app = dirty_app();
        let expected = app.document.skin().clone();
        assert!(app.save_to(path.clone()));
        assert!(!app.document.is_dirty());
        assert_eq!(app.document.path(), Some(path.as_path()));
        let reopened = SkinDocument::load_png(&path).unwrap();
        assert_eq!(reopened.skin(), &expected);
        assert!(!reopened.is_dirty());
        assert!(app.error_message.is_none());
    }

    #[test]
    fn failed_save_preserves_document_and_surfaces_error() {
        let directory = tempdir().unwrap();
        let mut app = dirty_app();
        let expected = app.document.skin().clone();
        assert!(!app.save_to(directory.path().to_path_buf()));
        assert_eq!(app.document.skin(), &expected);
        assert!(app.document.is_dirty());
        assert!(app.document.path().is_none());
        assert!(
            app.error_message
                .as_deref()
                .is_some_and(|message| message.contains("Could not save"))
        );
    }

    #[test]
    fn invalid_open_preserves_document_and_surfaces_validation_error() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("invalid.png");
        std::fs::write(&path, b"not a png").unwrap();
        let mut app = dirty_app();
        let expected = app.document.skin().clone();
        app.open_path(path);
        assert_eq!(app.document.skin(), &expected);
        assert!(app.document.is_dirty());
        assert!(app.document.path().is_none());
        assert!(
            app.error_message
                .as_deref()
                .is_some_and(|message| message.contains("Could not open"))
        );
    }

    #[test]
    fn file_names_gain_png_only_when_missing_an_extension() {
        assert_eq!(
            ensure_png_extension(PathBuf::from("alex")),
            PathBuf::from("alex.png")
        );
        assert_eq!(
            ensure_png_extension(PathBuf::from("alex.PNG")),
            PathBuf::from("alex.PNG")
        );
        assert_eq!(
            ensure_png_extension(PathBuf::from("alex.txt")),
            PathBuf::from("alex.png")
        );
    }

    #[test]
    fn shortcuts_cover_the_desktop_document_commands() {
        let shortcuts = std::hint::black_box([
            NEW_SHORTCUT,
            OPEN_SHORTCUT,
            SAVE_SHORTCUT,
            SAVE_AS_SHORTCUT,
            UNDO_SHORTCUT,
            REDO_SHORTCUT,
            REDO_ALTERNATE_SHORTCUT,
        ]);
        assert_eq!(
            shortcuts.map(|shortcut| shortcut.logical_key),
            [Key::N, Key::O, Key::S, Key::S, Key::Z, Key::Z, Key::Y]
        );
        assert!(shortcuts[3].modifiers.shift);
        assert!(shortcuts[5].modifiers.shift);
        for shortcut in shortcuts {
            assert!(shortcut.modifiers.command);
        }
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod gears;

use eframe::{CreationContext, egui::*};
use glam::Vec2;

#[cfg(not(target_arch = "wasm32"))]

fn main() -> eframe::Result {
    // env_logger::init(); // GPU Logs

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Mumper - 2D Engine",
        options,
        Box::new(|_cc| Ok(Box::new(Mumper::new(_cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(Mumper::new()))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

struct EngineState {}

struct Settings {}

struct Mumper {
    viewport: Rect,
    viewport_painter: Painter,
    smoothed_fps: f32,
    // Settings,
    segments: u16,
    radius: f32,
    // TODO : Add radius limits
    stroke_color: Color32,
    stroke_width: f32,
    camera_sensitivity: f32,
    zoom_sensitivity: f32,
    min_ppm: i16,
    max_ppm: i16,
    // State
    ppm: i16, // Pixel Per Meter = Zoom value
    camera_position: Vec2,
    camera_size_x: f32,
    camera_size_y: f32,
    // Objects Data
    circles_position: Vec<glam::Vec2>,
    circles_points: Vec<Vec<glam::Vec2>>,
    circles_strokes: Vec<Stroke>,
}

impl Mumper {
    fn new(cc: &CreationContext) -> Self {
        let circles_position: Vec<glam::Vec2> = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 1.0),
            Vec2::new(2.0, 1.0),
        ];

        let circle_points1 = gears::circle_points(circles_position[0], 1.0, 20);
        let circle_points2 = gears::circle_points(circles_position[1], 1.5, 20);
        let circle_points3 = gears::circle_points(circles_position[2], 2.0, 20);

        let circles_points = vec![circle_points1, circle_points2, circle_points3];
        let circles_strokes = vec![
            Stroke::new(2.0, Color32::RED),
            Stroke::new(2.0, Color32::GREEN),
            Stroke::new(2.0, Color32::BLUE),
        ];

        Self {
            viewport: cc.egui_ctx.content_rect(),
            viewport_painter: cc.egui_ctx.debug_painter(),
            smoothed_fps: 0.0,
            // Settings
            segments: 20,
            radius: 1.0,
            stroke_color: Color32::RED,
            stroke_width: 2.0,
            camera_sensitivity: 0.001,
            zoom_sensitivity: 0.3,
            min_ppm: 10,
            max_ppm: 1000,
            // State
            ppm: 100,
            camera_position: Vec2::ZERO,
            camera_size_x: 4.0,
            camera_size_y: 4.0,
            circles_position,
            circles_points,
            circles_strokes,
        }
    }

    // SCENE

    fn create_circle(&mut self, position: glam::Vec2, radius: f32, segments: u16, stroke: Stroke) {
        let circle_points = gears::circle_points(position, radius, segments);

        self.circles_position.push(position);
        self.circles_points.push(circle_points);
        self.circles_strokes.push(stroke);
    }

    fn clear_circles(&mut self) {
        self.circles_position.clear();
        self.circles_points.clear();
        self.circles_strokes.clear();
    }

    // RENDERING

    // world_to_screen :
    // 1] Viewport = Camera view
    // let clip_space = (camera.x * -1 + object.position.x) * ppm

    // 2] Camera view -> Viewport
    // 1) world_to_screen
    // let camera_left = camera_position - camera_size_x / 2;
    // let object_viewport_position_x = (-1 * camera_left + world_pos.x) / camera_size_x;
    fn world_to_screen(&self, world_pos: glam::Vec2) -> Pos2 {
        // 2] Camera view -> Viewport
        let camera_left = self.camera_position.x - self.camera_size_x / 2.0;
        let camera_bot = self.camera_position.y - self.camera_size_y / 2.0;

        let fulcrum_x = (-1.0 * camera_left + world_pos.x) / self.camera_size_x;
        let fulcrum_y = 1.0 - (-1.0 * camera_bot + world_pos.y) / self.camera_size_y; // y inverted = ui

        let screen_pos_x = fulcrum_x * self.viewport.width();
        let screen_pos_y = fulcrum_y * self.viewport.height();

        let screen_pos = Pos2::new(screen_pos_x, screen_pos_y);
        return screen_pos;
    }

    fn screen_to_world(&self, screen_pos: Pos2) -> Vec2 {
        let camera_left = self.camera_position.x - self.camera_size_x / 2.0;
        let camera_bot = self.camera_position.y - self.camera_size_y / 2.0;

        let ppm = self.ppm as f32;
        let world_pos_x = camera_left + screen_pos.x / ppm;
        let world_pos_y = camera_bot + (self.viewport.height() - screen_pos.y) / ppm; // y inverted

        return Vec2::new(world_pos_x, world_pos_y);
    }

    fn render_frame(&self) {
        // TODO : Check if object is in frame

        // Draw Circles
        for i in 0..self.circles_position.len() {
            self.draw_circle(self.circles_strokes[i], &self.circles_points[i]);
        }
    }

    // Draw an edge between each circle point
    fn draw_circle(&self, stroke: Stroke, circle_points: &Vec<glam::Vec2>) {
        for index in 0..circle_points.len() {
            let end_index = (index + 1) % circle_points.len();

            let start_pos = self.world_to_screen(circle_points[index]);
            let end_pos = self.world_to_screen(circle_points[end_index]);

            // draw_edge
            self.viewport_painter
                .line_segment([start_pos, end_pos], stroke);
        }
    }

    // UI COMPONENTS

    fn ui_settings(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Segments :");
            ui.add(egui::Slider::new(&mut self.segments, 3..=100));
            ui.label("Radius :");
            ui.add(egui::Slider::new(&mut self.radius, 0.1..=10.0));
        });

        // Stroke Settings
        ui.horizontal(|ui| {
            ui.label("Stroke Width :");
            ui.add(egui::Slider::new(&mut self.stroke_width, 1.0..=10.0));

            let color_label = ui.label("Stroke Color :");
            ui.color_edit_button_srgba(&mut self.stroke_color)
                .labelled_by(color_label.id);
        });
    }

    fn ui_state(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Clear Circles").clicked() {
                self.clear_circles();
            }

            ui.label("Zoom :");
            ui.add(egui::Slider::new(
                &mut self.ppm,
                self.min_ppm..=self.max_ppm,
            ));
        });
    }

    // Displayed on top of the viewport
    fn hud(&mut self, fps: f32) {
        let painter = &mut self.viewport_painter;

        // FPS Display
        let alpha = 0.05;
        self.smoothed_fps = (self.smoothed_fps * (1.0 - alpha)) + (fps * alpha);

        painter.text(
            self.viewport.left_top() + egui::vec2(10.0, 10.0), // 10px padding from top-left
            egui::Align2::LEFT_TOP,
            format!("FPS: {:.2}", self.smoothed_fps),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        // Controls Display
        painter.text(
            self.viewport.left_top() + egui::vec2(10.0, 30.0),
            egui::Align2::LEFT_TOP,
            "Look : Right Click",
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    // CONTROLS

    fn input_handling(&mut self, response: Response, input_state: &InputState) {
        // 1] Input Detection
        let pointer_delta: egui::Vec2 = input_state.pointer.delta();
        let lclick_released = input_state.pointer.primary_released();
        let rclick_hold = input_state.pointer.secondary_down();
        let mut global_pointer_position = Pos2::new(0.0, 0.0);

        if let Some(mouse_position) = input_state.pointer.hover_pos() {
            global_pointer_position = mouse_position;
        }

        // 2] Input Reaction
        // Camera limits -> Depend on viewport size
        self.camera_size_x = self.viewport.width() / self.ppm as f32;
        self.camera_size_y = self.viewport.height() / self.ppm as f32;

        // Change zoom with mousewheel
        let mousewheel_delta = input_state.smooth_scroll_delta * self.zoom_sensitivity;
        self.ppm = (self.ppm + mousewheel_delta.y as i16).clamp(self.min_ppm, self.max_ppm); // Notch based zoom

        // RClick = Move Camera
        if rclick_hold {
            let sensivity = self.camera_sensitivity * (self.max_ppm / self.ppm) as f32;
            self.camera_position.x -= pointer_delta.x * sensivity;
            self.camera_position.y += pointer_delta.y * sensivity;
        }

        // LClick = Create Circle
        if lclick_released && response.hovered() {
            let world_pos = self.screen_to_world(global_pointer_position);
            self.create_circle(
                world_pos,
                self.radius,
                self.segments,
                Stroke::new(self.stroke_width, self.stroke_color),
            );
        }
    }
}

impl eframe::App for Mumper {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS
            let dt = ui.input(|i| i.stable_dt); // DeltaTime in second
            let fps = 1.0 / dt;

            // UI

            self.ui_settings(ui);
            self.ui_state(ui);

            // SCENE

            // Circles Draw Area
            let (response, painter) = ui.allocate_painter(
                ui.available_size(), // All remaining space
                Sense::click(),
            );

            let rect = response.rect;
            self.viewport = rect;
            self.viewport_painter = painter;

            // Border
            self.viewport_painter.rect_stroke(
                rect,
                5.0,
                egui::Stroke::new(2.0, egui::Color32::GREEN),
                egui::StrokeKind::Middle,
            );

            // Inputs Handling
            ui.input(|input_state: &InputState| {
                self.input_handling(response, input_state);
            });

            self.hud(fps);
            self.render_frame();
        });
    }
}

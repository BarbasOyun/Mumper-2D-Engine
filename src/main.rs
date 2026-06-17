#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod gears;

use eframe::{CreationContext, egui::*};

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
    // State
    // world_vector_basis: Vec2
    ppm: u16, // Piwels Per Meter
    camera_position: Vec2,
    camera_size_x: u16,
    camera_size_y: u16,
    // Objects Data
    circles_position: Vec<glam::Vec2>,
    circles_points: Vec<Vec<glam::Vec2>>,
    circles_strokes: Vec<Stroke>,
}

impl Mumper {
    fn new(cc: &CreationContext) -> Self {
        let circles_position: Vec<glam::Vec2> = vec![
            glam::Vec2::new(1.0, 1.0),
            glam::Vec2::new(1.5, 1.0),
            glam::Vec2::new(2.0, 1.0),
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
            radius: 100.0,
            stroke_color: Color32::RED,
            stroke_width: 2.0,
            // State
            ppm: 100,
            camera_position: Vec2::ZERO,
            camera_size_x: 3,
            camera_size_y: 3,
            circles_position,
            circles_points,
            circles_strokes,
        }
    }

    // SCENE

    fn create_circle(&mut self, position: Vec2, radius: f32, segments: u16, stroke: Stroke) {
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

    fn camera_mode() {
        todo!()
        // 1] Viewport = Camera view
        // let clip_space = (camera.x * -1 + object.position.x) * ppm

        // 2] Camera view -> Viewport
        // 1) object_viewport_position_x :
        // let camera_left = camera_position - camera_size_x / 2;
        // let object_viewport_position_x = (abs(camera_left) + object.position.x) / camera_size_x;

        // 2) object_screen_size * viewport.aspect_ratio();
    }

    fn world_to_screen() {}

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

            let start_pos = Pos2::new(circle_points[index].x, circle_points[index].y);
            let end_pos = Pos2::new(circle_points[end_index].x, circle_points[end_index].y);

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
            ui.add(egui::Slider::new(&mut self.radius, 50.0..=300.0));
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
        if ui.button("Clear Circles").clicked() {
            self.clear_circles();
        }
    }

    // Displayed on top of the 3D View
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
        let left_mouse_released = input_state.pointer.primary_released();
        let mut global_mouse_position = Pos2::new(0.0, 0.0);

        if let Some(mouse_position) = input_state.pointer.hover_pos() {
            global_mouse_position = mouse_position;
        }

        // 2] Input Reaction
        // Camera
        // Camera limits -> Depend on viewport size
        self.camera_size_x = (self.viewport.width() / self.ppm as f32) as u16;
        self.camera_size_y = (self.viewport.height() / self.ppm as f32) as u16;

        // Change Radius with mousewheel
        let delta = input_state.smooth_scroll_delta;
        self.radius = (self.radius + delta.y).clamp(50.0, 300.0);

        // Create Circle on Click
        if left_mouse_released && response.hovered() {
            self.create_circle(
                vec2(global_mouse_position.x, global_mouse_position.y),
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

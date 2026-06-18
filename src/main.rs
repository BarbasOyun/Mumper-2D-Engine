#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod gears;

use eframe::{CreationContext, egui::*};
use glam::Vec2;
// Multi-threading -> use tokio for web browser support?
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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

struct Settings {
    segments: u16,
    radius: f32,
    // TODO : Add radius limits
    stroke_color: Color32,
    stroke_width: f32,
    camera_sensitivity: f32,
    zoom_sensitivity: f32,
    min_ppm: i16,
    max_ppm: i16,
}

impl Settings {
    fn new() -> Self {
        return Self {
            segments: 20,
            radius: 1.0,
            stroke_color: Color32::RED,
            stroke_width: 2.0,
            camera_sensitivity: 0.001,
            zoom_sensitivity: 0.003,
            min_ppm: 10,
            max_ppm: 1000,
        };
    }
}

// Shared between Rendering & Physic Threads
struct MumperPhysics {
    positions: Vec<Vec2>,
    speeds: Vec<Vec2>, // meters / frame -> at 60 fps = meters / 0.016sec -> * 60 = meters / sec eg 0.1 = 6 meters / sec
}

impl MumperPhysics {
    fn new(positions: Vec<Vec2>, speeds: Vec<Vec2>) -> Self {
        return Self { positions, speeds };
    }

    // Physics update
    fn tick(&mut self, dt: f32) {
        // simulate
        for i in 0..self.positions.len() {
            self.positions[i].x += self.speeds[i].x * dt;
            self.positions[i].y += self.speeds[i].y * dt;

            // pos.y -= 9.81 * dt; // Simple gravity fall
            // if pos.y < 0.0 { pos.y = 0.0; } // Square collision
        }
    }
}

struct EngineState {
    // View
    viewport: Rect,
    viewport_painter: Painter,
    smoothed_fps: f32,
    // World
    ppm: i16, // Pixel Per Meter = Zoom value
    camera_position: Vec2,
    camera_size_x: f32,
    camera_size_y: f32,
    // Objects Data
    // TODO : ECS
    // Smart IDs
    physics: Arc<Mutex<MumperPhysics>>,
    positions: Vec<Vec2>, // Store positions
    points: Vec<Vec<Vec2>>,
    strokes: Vec<Stroke>,
}

// TODO : Create Physic & Rendering Threads
impl EngineState {
    fn new(viewport: Rect, viewport_painter: Painter) -> Self {
        // Default = Square + 3 Circles
        let positions: Vec<Vec2> = vec![
            Vec2::ZERO,
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 1.0),
            Vec2::new(2.0, 1.0),
        ];

        let speeds: Vec<Vec2> = vec![
            Vec2::ZERO,
            Vec2::new(0.01, 0.0),
            Vec2::new(0.01, 0.01),
            Vec2::new(-0.01, 0.0),
        ];

        let square_points = vec![
            Vec2::new(5.0, -5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(-5.0, 5.0),
            Vec2::new(-5.0, -5.0),
        ];

        let circle_points1 = gears::circle_points(positions[1], 1.0, 20);
        let circle_points2 = gears::circle_points(positions[2], 1.5, 20);
        let circle_points3 = gears::circle_points(positions[3], 2.0, 20);

        let points = vec![
            square_points,
            circle_points1,
            circle_points2,
            circle_points3,
        ];

        let strokes = vec![
            Stroke::new(5.0, Color32::LIGHT_YELLOW),
            Stroke::new(2.0, Color32::RED),
            Stroke::new(2.0, Color32::GREEN),
            Stroke::new(2.0, Color32::BLUE),
        ];

        let physics = Arc::new(Mutex::new(MumperPhysics::new(positions, speeds)));

        let physics_clone = Arc::clone(&physics);
        
        // Start Physic Thread
        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f32();
                last_tick = now;

                {
                    let mut physics = physics_clone.lock().unwrap();
                    physics.tick(dt);
                }

                thread::sleep(Duration::from_millis(8));
            }
        });

        return Self {
            // View
            viewport,
            viewport_painter,
            smoothed_fps: 0.0,
            // World
            ppm: 100,
            camera_position: Vec2::ZERO,
            camera_size_x: 4.0,
            camera_size_y: 4.0,
            physics,
            positions: vec![],
            // speeds,
            points,
            strokes,
        };
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
        let camera_position = self.camera_position;
        let camera_size_x = self.camera_size_x;
        let camera_size_y = self.camera_size_y;

        // 2] Camera view -> Viewport
        let camera_left = camera_position.x - camera_size_x / 2.0;
        let camera_bot = camera_position.y - camera_size_y / 2.0;

        let fulcrum_x = (-1.0 * camera_left + world_pos.x) / camera_size_x;
        let fulcrum_y = 1.0 - (-1.0 * camera_bot + world_pos.y) / camera_size_y; // y inverted = ui

        let screen_pos_x = fulcrum_x * self.viewport.width();
        let screen_pos_y = fulcrum_y * self.viewport.height();

        let screen_pos = Pos2::new(screen_pos_x, screen_pos_y);
        return screen_pos;
    }

    fn screen_to_world(&self, screen_pos: Pos2) -> Vec2 {
        let camera_position = self.camera_position;

        let camera_left = camera_position.x - self.camera_size_x / 2.0;
        let camera_bot = camera_position.y - self.camera_size_y / 2.0;

        let ppm = self.ppm as f32;
        let world_pos_x = camera_left + screen_pos.x / ppm;
        let world_pos_y = camera_bot + (self.viewport.height() - screen_pos.y) / ppm; // y inverted

        return Vec2::new(world_pos_x, world_pos_y);
    }

    fn render_frame(&mut self) {
        // TODO : Check if object is in frame

        self.positions = {
            let physics = self.physics.lock().unwrap();
            physics.positions.clone()
        };

        // Draw Circles
        for i in 0..self.positions.len() {
            self.draw_polygon(self.strokes[i], &self.points[i]);
        }
    }

    // Draw an edge between each circle point
    fn draw_polygon(&self, stroke: Stroke, circle_points: &Vec<glam::Vec2>) {
        for index in 0..circle_points.len() {
            let end_index = (index + 1) % circle_points.len();

            let start_pos = self.world_to_screen(circle_points[index]);
            let end_pos = self.world_to_screen(circle_points[end_index]);

            // draw_edge
            self.viewport_painter
                .line_segment([start_pos, end_pos], stroke);
        }
    }

    // SCENE

    fn create_polygon(&mut self, position: glam::Vec2, radius: f32, segments: u16, stroke: Stroke) {
        let circle_points = gears::circle_points(position, radius, segments);

        {
            let mut physics = self.physics.lock().unwrap();
            physics.positions.push(position);
            physics.speeds.push(Vec2::new(0.0, 0.0)); // TODO : Add Speed Setting
        };

        self.points.push(circle_points);
        self.strokes.push(stroke);
    }

    fn clear_polygons(&mut self) {
        self.positions.clear();
        self.points.clear();
        self.strokes.clear();
    }
}

struct Mumper {
    settings: Settings,
    state: EngineState,
}

impl Mumper {
    fn new(cc: &CreationContext) -> Self {
        Self {
            settings: Settings::new(),
            state: EngineState::new(cc.egui_ctx.content_rect(), cc.egui_ctx.debug_painter()),
        }
    }

    fn reset_settings(&mut self) {
        self.settings = Settings::new();
    }

    fn reset_scene(&mut self) {
        self.state = EngineState::new(self.state.viewport, self.state.viewport_painter.clone());
    }

    // UI COMPONENTS

    fn ui_settings(&mut self, ui: &mut Ui) {
        let settings = &mut self.settings;

        ui.horizontal(|ui| {
            ui.label("Segments :");
            ui.add(egui::Slider::new(&mut settings.segments, 3..=100));
            ui.label("Radius :");
            ui.add(egui::Slider::new(&mut settings.radius, 0.1..=10.0));
        });

        // Stroke Settings
        ui.horizontal(|ui| {
            ui.label("Stroke Width :");
            ui.add(egui::Slider::new(&mut settings.stroke_width, 1.0..=10.0));

            let color_label = ui.label("Stroke Color :");
            ui.color_edit_button_srgba(&mut settings.stroke_color)
                .labelled_by(color_label.id);
        });
    }

    fn ui_state(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Reset Scene").clicked() {
                self.reset_scene();
            }

            let settings = &mut self.settings;
            let state = &mut self.state;

            ui.label("Zoom :");
            ui.add(egui::Slider::new(
                &mut state.ppm,
                settings.min_ppm..=settings.max_ppm,
            ));
        });
    }

    // Displayed on top of the viewport
    fn hud(&mut self, fps: f32) {
        let state = &mut self.state;
        let painter = &mut state.viewport_painter;

        // FPS Display
        let alpha = 0.05;
        state.smoothed_fps = (state.smoothed_fps * (1.0 - alpha)) + (fps * alpha);

        painter.text(
            state.viewport.left_top() + egui::vec2(10.0, 10.0), // 10px padding from top-left
            egui::Align2::LEFT_TOP,
            format!("FPS: {:.2}", state.smoothed_fps),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        // Controls Display
        painter.text(
            state.viewport.left_top() + egui::vec2(10.0, 30.0),
            egui::Align2::LEFT_TOP,
            "Look : Right Click",
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    // CONTROLS

    fn input_handling(&mut self, response: Response, input_state: &InputState) {
        let settings = &mut self.settings;
        let state = &mut self.state;

        // 1] Input Detection
        let pointer_delta: egui::Vec2 = input_state.pointer.delta();
        let lclick_released = input_state.pointer.primary_released();
        let rclick_hold = input_state.pointer.secondary_down();
        let mut global_pointer_position = Pos2::new(0.0, 0.0);

        if let Some(mouse_position) = input_state.pointer.hover_pos() {
            global_pointer_position = mouse_position;
        }

        // 2] Input Reaction
        let ppm = state.ppm as f32;
        // Camera limits -> Depend on viewport size
        state.camera_size_x = state.viewport.width() / ppm;
        state.camera_size_y = state.viewport.height() / ppm;

        // Mousewheel = Zoom
        let scroll_delta = input_state.smooth_scroll_delta.y * settings.zoom_sensitivity * ppm;
        state.ppm = (state.ppm + scroll_delta as i16).clamp(settings.min_ppm, settings.max_ppm); // Notch based zoom

        // RClick = Move Camera
        if rclick_hold {
            let sensivity = settings.camera_sensitivity * (settings.max_ppm / state.ppm) as f32;
            state.camera_position.x -= pointer_delta.x * sensivity;
            state.camera_position.y += pointer_delta.y * sensivity;
        }

        // LClick = Create Circle
        if lclick_released && response.hovered() {
            let world_pos = state.screen_to_world(global_pointer_position);
            state.create_polygon(
                world_pos,
                settings.radius,
                settings.segments,
                Stroke::new(settings.stroke_width, settings.stroke_color),
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
            self.hud(fps);

            // SCENE

            // Circles Draw Area
            let (response, painter) = ui.allocate_painter(
                ui.available_size(), // All remaining space
                Sense::click(),
            );
            let rect = response.rect;

            // Inputs Handling
            ui.input(|input_state: &InputState| {
                self.input_handling(response, input_state);
            });

            let state = &mut self.state;

            state.viewport = rect;
            state.viewport_painter = painter;

            // Border
            state.viewport_painter.rect_stroke(
                rect,
                5.0,
                egui::Stroke::new(2.0, egui::Color32::GREEN),
                egui::StrokeKind::Middle,
            );

            state.render_frame();
        });
    }
}

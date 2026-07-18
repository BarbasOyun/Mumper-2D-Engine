#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use mumper::Mumper;
use mumper::MumperECS;
use mumper::MumperRenderer;
use mumper::gears;

use eframe::{CreationContext, egui::*};
use glam::Vec2;
use std::sync::atomic::Ordering;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MumperDemo - 2D Engine",
        options,
        Box::new(|_cc| Ok(Box::new(MumperDemo::new(_cc)))),
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
                Box::new(|cc| Ok(Box::new(MumperDemo::new(cc)))),
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

struct DemoSettings {
    // Polygon Drawing Settings
    segments: u16,
    radius: f32,
    stroke_color: Color32,
    stroke_width: f32,
    polygon_velocity: Vec2,
}

impl DemoSettings {
    fn new() -> Self {
        return Self {
            // Polygons
            segments: 20,
            radius: 1.0,
            stroke_color: Color32::RED,
            stroke_width: 2.0,
            polygon_velocity: Vec2::new(1.0, 1.0),
        };
    }
}

// TODO : Oscillating Color Component

struct MumperDemo {
    // Mumper Implementation
    mumper: Mumper,
    // Custom App Data
    settings: DemoSettings,
}

impl MumperDemo {
    fn new(cc: &CreationContext) -> Self {
        let settings = DemoSettings::new();
        let mut mumper: Mumper = Mumper::new(cc);

        Self::default_scene(&mut mumper);

        return Self { mumper, settings };
    }

    fn reset_settings(&mut self) {
        self.mumper.reset_settings();
        self.settings = DemoSettings::new();
    }

    fn reset_scene(&mut self) {
        self.mumper.clear_scene();
        Self::default_scene(&mut self.mumper);
    }

    fn input_handling(&mut self, response: Response, input_state: &InputState) {
        self.mumper.camera_controls(input_state);

        // Input Detection
        let lclick_released = input_state.pointer.primary_released();
        let mut global_pointer_position = Pos2::new(0.0, 0.0);

        if let Some(mouse_position) = input_state.pointer.hover_pos() {
            global_pointer_position = mouse_position;
        }

        // Input Reaction

        // LClick = Create Polygon
        if lclick_released && response.hovered() {
            let settings = &mut self.settings;

            let world_pos = MumperRenderer::screen_to_world(&self.mumper.renderer, &global_pointer_position);
            let radius = settings.radius;
            let segments = settings.segments;
            let vertices = gears::circle_vertices(radius, segments);
            let stroke = Stroke::new(settings.stroke_width, settings.stroke_color);

            Self::create_shape(&mut self.mumper, world_pos, vertices, radius, settings.polygon_velocity, stroke);
        }
    }

    // Create a Shape with a Radius Collider & Rigidbody
    fn create_shape(
        mumper: &mut Mumper,
        world_pos: Vec2,
        vertices: Vec<Vec2>,
        radius: f32,
        velocity: Vec2,
        stroke: Stroke,
    ) {
        // Create entity + Add components = Polygon
        let entity_id = MumperECS::create_entity(&mut mumper.ecs);
        MumperECS::add_transform(mumper, entity_id, world_pos, 0.0, Vec2::ONE);
        MumperECS::add_shape_renderer(mumper, entity_id, vertices, stroke);
        MumperECS::add_radius_collider(mumper, entity_id, radius);
        MumperECS::add_rigidbody(mumper, entity_id, velocity, -1.0, 1.0);
    }

    // Create a (static) Wall
    fn create_wall(
        mumper: &mut Mumper,
        world_pos: Vec2,
        vertices: Vec<Vec2>,
        thickness: f32,
        stroke: Stroke,
    ) {
        // Create entity + Add components = Polygon
        let entity_id = MumperECS::create_entity(&mut mumper.ecs);
        MumperECS::add_transform(mumper, entity_id, world_pos, 0.0, Vec2::ONE);
        MumperECS::add_shape_renderer(mumper, entity_id, vertices, stroke);
        MumperECS::add_segments_collider(mumper, entity_id, thickness);
    }

    // UI COMPONENTS

    pub fn ui_settings(&mut self, ui: &mut Ui) {
        let settings = &mut self.settings;
        let mumper_settings = &mut self.mumper.settings;

        // Polygon Creation Settings
        ui.horizontal(|ui| {
            // Shape
            ui.label("POLYGON: ");
            ui.label("Segments");
            ui.add(egui::Slider::new(&mut settings.segments, 3..=100));
            ui.label("Radius");
            ui.add(egui::Slider::new(&mut settings.radius, 0.1..=10.0));
        });

        // Rigid body
        ui.horizontal(|ui| {
            ui.label("RIGID BODY: ");
            ui.label("Velocity ");
            ui.label("X");
            ui.add(egui::Slider::new(
                &mut settings.polygon_velocity.x,
                -10.0..=10.0,
            ));
            ui.label("Y");
            ui.add(egui::Slider::new(
                &mut settings.polygon_velocity.y,
                -10.0..=10.0,
            ));
        });

        // Stroke Settings
        ui.horizontal(|ui| {
            ui.label("STROKE: ");
            ui.label("Width:");
            ui.add(egui::Slider::new(&mut settings.stroke_width, 1.0..=10.0));

            let color_label = ui.label("Color:");
            ui.color_edit_button_srgba(&mut settings.stroke_color)
                .labelled_by(color_label.id);
        });

        // Rendering Settings
        ui.horizontal(|ui: &mut Ui| {
            ui.label("Gizmo: ");
            ui.checkbox(&mut mumper_settings.is_drawing_normals, "Draw Normals");
        });
    }

    pub fn ui_state(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Reset Scene").clicked() {
                self.reset_scene();
            }

            let state = &mut self.mumper;

            ui.label("Zoom :");
            ui.add(egui::Slider::new(
                &mut state.renderer.ppm,
                state.settings.min_ppm..=state.settings.max_ppm,
            ));

            let mut local_pause = state.is_paused.load(Ordering::Relaxed);

            if ui.checkbox(&mut local_pause, "Pause").changed() {
                state.pause_physic(local_pause);
            }

            ui.checkbox(&mut state.settings.default_transform, "Default Transform");
        });
    }

    fn default_scene(mumper: &mut Mumper) {
        // Default Scene

        // Square
        let square_vertices: Vec<Vec2> = vec![
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
            Vec2::new(-10.0, -10.0),
        ];

        let square_stroke = Stroke::new(5.0, Color32::LIGHT_YELLOW);

        Self::create_wall(mumper, Vec2::ZERO, square_vertices, 0.1, square_stroke);

        // state.create_shape(
        //     square_vertices,
        //     false,
        //     0.0,
        //     0.1,
        //     Vec2::ZERO,
        //     0.785,
        //     Vec2::ONE,
        //     Vec2::ZERO,
        //     0.0,
        //     0.0,
        //     Stroke::new(5.0, Color32::LIGHT_YELLOW),
        // );

        // 3 Default Circles
        let (
            radiuses,
            vertices,
            positions,
            rotations,
            scales,
            velocities,
            rotation_speeds,
            bounciness,
            strokes,
        ) = Self::default_polygons();

        for i in 0..radiuses.len() {
            // TODO :
            // add renderer + transform
            // add radius collider
            // add rigidbody

            // state.create_shape(
            //     vertices[i].clone(),
            //     true,
            //     radiuses[i],
            //     0.1,
            //     positions[i],
            //     rotations[i],
            //     scales[i],
            //     velocities[i],
            //     rotation_speeds[i],
            //     bounciness[i],
            //     strokes[i],
            // );

            Self::create_shape(mumper, positions[i], vertices[i].clone(), radiuses[i], velocities[i], strokes[i]);
        }
    }

    // Default Scene = 1 Square + 3 Circles
    fn default_polygons() -> (
        Vec<f32>,
        Vec<Vec<Vec2>>,
        Vec<Vec2>,
        Vec<f32>,
        Vec<Vec2>,
        Vec<Vec2>,
        Vec<f32>,
        Vec<f32>,
        Vec<Stroke>,
    ) {
        let radiuses: Vec<f32> = vec![1.0, 1.5, 2.0];

        // Vertices
        let circle_vertices1 = gears::circle_vertices(radiuses[0], 20);
        let circle_vertices2 = gears::circle_vertices(radiuses[1], 20);
        let circle_vertices3 = gears::circle_vertices(radiuses[2], 20);

        let vertices: Vec<Vec<Vec2>> = vec![circle_vertices1, circle_vertices2, circle_vertices3];

        // Transforms
        let positions: Vec<Vec2> = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.5, 1.0),
            Vec2::new(2.0, 1.0),
        ];

        let rotations: Vec<f32> = vec![0.0, 0.0, 0.0];
        let scales: Vec<Vec2> = vec![Vec2::ONE, Vec2::ONE, Vec2::ONE];

        let velocities: Vec<Vec2> = vec![
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, -1.0),
        ];

        let rotation_speeds: Vec<f32> = vec![1.0, -1.5, 0.5];

        // Rigid body
        let bounciness: Vec<f32> = vec![1.0, 0.9, 0.5];

        // Rendering
        let strokes: Vec<Stroke> = vec![
            Stroke::new(2.0, Color32::RED),
            Stroke::new(2.0, Color32::GREEN),
            Stroke::new(2.0, Color32::BLUE),
        ];

        return (
            radiuses,
            vertices,
            positions,
            rotations,
            scales,
            velocities,
            rotation_speeds,
            bounciness,
            strokes,
        );
    }
}

impl eframe::App for MumperDemo {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (dt, fps) = self.mumper.update(ui);

            // Ui
            self.ui_settings(ui);
            self.ui_state(ui);
            self.mumper.hud(fps);

            // Mumper Rendering
            let response = self.mumper.game_update(ui);

            // Inputs
            ui.input(|input_state: &InputState| {
                self.input_handling(response, input_state);
            });
        });
    }
}

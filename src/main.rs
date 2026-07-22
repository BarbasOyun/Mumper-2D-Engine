#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::{CreationContext, egui::*};
use glam::Vec2;
use std::sync::atomic::Ordering;

use mumper::Mumper;
use mumper::MumperRenderer;
pub(crate) use mumper::component_storage;
use mumper::gears;
use mumper::mumper_ecs::*;

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

const MAX_RAD: f32 = std::f32::consts::PI * 2.0;

// TODO : Oscillating Color Component
struct MumperDemo {
    // Mumper Implementation
    mumper: Mumper,
    // Custom App Data
    settings: DemoSettings,
    ocillating_color_storage_id: usize,
}

impl MumperDemo {
    fn new(cc: &CreationContext) -> Self {
        let settings = DemoSettings::new();
        let mut mumper: Mumper = Mumper::new(cc);

        // Register Custom Component
        let ocillating_color_storage_id = MumperECS::register_storage_id(&mut mumper.ecs);
        let oscillating_color_storage = OscillatingColorStorage::new(ocillating_color_storage_id);
        MumperECS::register_storage(&mut mumper.components, oscillating_color_storage);

        let mut mumper_demo = Self {
            mumper,
            settings,
            ocillating_color_storage_id: ocillating_color_storage_id - 7,
        };

        mumper_demo.default_scene();

        return mumper_demo;
    }

    fn reset_settings(&mut self) {
        self.mumper.reset_settings();
        self.settings = DemoSettings::new();
    }

    fn reset_scene(&mut self) {
        self.mumper.clear_scene();
        self.default_scene();
    }

    fn logic_update(&mut self, dt: &f32) {
        self.oscillating_color_system(dt);
    }

    fn oscillating_color_system(&mut self, dt: &f32) {
        let any_storage =
            self.mumper.components.custom_components[self.ocillating_color_storage_id].as_any_mut();

        let Some(oscillating_color_storage) = any_storage.downcast_mut::<OscillatingColorStorage>()
        else {
            return;
        };

        for i in 0..oscillating_color_storage.entities.len() {
            let entity_id = oscillating_color_storage.entities[i];

            // Oscillate rad
            oscillating_color_storage.r_rad[i] =
                (oscillating_color_storage.r_rad[i] + 1.0 * dt) % MAX_RAD;
            oscillating_color_storage.g_rad[i] =
                (oscillating_color_storage.g_rad[i] + 1.0 * dt) % MAX_RAD;
            oscillating_color_storage.b_rad[i] =
                (oscillating_color_storage.b_rad[i] + 1.0 * dt) % MAX_RAD;

            let r_mult = (1.0 + oscillating_color_storage.r_rad[i].sin()) / 2.0;
            let g_mult = (1.0 + oscillating_color_storage.g_rad[i].sin()) / 2.0;
            let b_mult = (1.0 + oscillating_color_storage.b_rad[i].sin()) / 2.0;

            let r = (255.0 * r_mult) as u8;
            let g = (255.0 * g_mult) as u8;
            let b = (255.0 * b_mult) as u8;

            let image_color = Color32::from_rgb(r, g, b);

            let renderer_component_id = self
                .mumper
                .renderer
                .shape_renderer_storage
                .get_component_id(entity_id);

            self.mumper.renderer.shape_renderer_storage.strokes[renderer_component_id].color =
                image_color;
        }
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
            let settings = &self.settings;

            let world_pos =
                MumperRenderer::screen_to_world(&self.mumper.renderer, &global_pointer_position);
            let radius = settings.radius;
            let segments = settings.segments;
            let vertices = gears::circle_vertices(radius, segments);
            let stroke = Stroke::new(settings.stroke_width, settings.stroke_color);

            let entity_id = Mumper::create_shape(
                &mut self.mumper,
                world_pos,
                vertices,
                radius,
                settings.polygon_velocity,
                stroke,
            );

            // Get Custom Component
            let any_storage = self.mumper.components.custom_components
                [self.ocillating_color_storage_id]
                .as_any_mut();

            if let Some(storage) = any_storage.downcast_mut::<OscillatingColorStorage>() {
                storage.add(&mut self.mumper.ecs, entity_id, 1.0, 0.0, 2.0);
            }
        }
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

    fn default_scene(&mut self) {
        let mumper = &mut self.mumper;

        // Square
        let square_vertices: Vec<Vec2> = vec![
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
            Vec2::new(-10.0, -10.0),
        ];

        let square_stroke = Stroke::new(5.0, Color32::LIGHT_YELLOW);

        Mumper::create_wall(
            mumper,
            Vec2::ZERO,
            0.785,
            square_vertices,
            0.1,
            square_stroke,
        );

        // Circles
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

        for i in 0..radiuses.len() {
            Mumper::create_shape(
                &mut self.mumper,
                positions[i],
                vertices[i].clone(),
                radiuses[i],
                velocities[i],
                strokes[i],
            );
        }
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

            self.logic_update(&dt);

            // Inputs
            ui.input(|input_state: &InputState| {
                self.input_handling(response, input_state);
            });
        });
    }
}

// TOOD : struct Polygon

component_storage!(
    struct OscillatingColorStorage {
        r_rad: f32,
        g_rad: f32,
        b_rad: f32,
    },
    add_default: |storage: &mut OscillatingColorStorage, ecs: &mut MumperECS, entity_id: usize| {
        storage.add(ecs, entity_id, 0.0, 0.0, 0.0);
    }
);

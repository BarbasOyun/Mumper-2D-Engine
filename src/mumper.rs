use eframe::{CreationContext, egui::*};
use glam::Vec2;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread; // only viable for desktop
use std::time::{Duration, Instant};

use crate::MumperPhysics;
use crate::gears;

pub struct Mumper {
    pub settings: Settings,
    pub state: MumperState,
}

impl Mumper {
    pub fn new(cc: &CreationContext) -> Self {
        Self {
            settings: Settings::new(),
            state: MumperState::new(cc.egui_ctx.content_rect(), cc.egui_ctx.debug_painter()),
        }
    }

    pub fn reset_settings(&mut self) {
        self.settings = Settings::new();
    }

    pub fn reset_scene(&mut self) {
        self.state = MumperState::new(self.state.viewport, self.state.viewport_painter.clone());
    }

    // UPDATE

    pub fn update(ui: &Ui) -> (f32, f32) {
        ui.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS
        let dt = ui.input(|i| i.stable_dt); // DeltaTime in second
        let fps = 1.0 / dt;

        return (dt, fps);
    }

    pub fn rendering(&mut self, ui: &mut Ui) -> Response {
        // Draw Area
        let (response, painter) = ui.allocate_painter(
            ui.available_size(), // All remaining space
            Sense::click(),
        );
        let rect = response.rect;

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

        // Web Physics
        // Handle pause
        #[cfg(target_arch = "wasm32")]
        {
            if state.is_paused.load(std::sync::atomic::Ordering::Relaxed) {
                continue;
            }

            let mut physics = state.physics.lock().unwrap();

            physics.tick(dt);
        }

        state.render_frame(&self.settings);

        return response;
    }

    // Displayed on top of the viewport
    pub fn hud(&mut self, fps: f32) {
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

    pub fn camera_controls(&mut self, input_state: &InputState) {
        let settings = &mut self.settings;
        let state = &mut self.state;

        let pointer_delta: egui::Vec2 = input_state.pointer.delta();
        let rclick_hold = input_state.pointer.secondary_down();

        let ppm = state.ppm as f32;
        // Camera limits -> Depend on viewport size
        state.camera_size_x = state.viewport.width() / ppm;
        state.camera_size_y = state.viewport.height() / ppm;

        // Mousewheel = Zoom
        let mut scroll_delta =
            input_state.smooth_scroll_delta.y * settings.zoom_sensitivity * ppm * 0.003; // Notch based zoom

        if scroll_delta != 0.0 {
            gears::reverse_clamp(&mut scroll_delta, -0.1, 0.1);
            state.ppm = (state.ppm + scroll_delta).clamp(settings.min_ppm, settings.max_ppm);
        }

        // RClick = Move Camera
        if rclick_hold {
            let sensivity =
                settings.camera_sensitivity * (settings.max_ppm / state.ppm) as f32 * 0.001;
            state.camera_position.x -= pointer_delta.x * sensivity;
            state.camera_position.y += pointer_delta.y * sensivity;
        }
    }
}

pub struct Settings {
    // Camera
    pub camera_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub min_ppm: f32,
    pub max_ppm: f32,
    // Gizmo
    pub is_drawing_normals: bool,
    // Misc
    pub default_transform: bool, // Use default transform when pausing instead of Freezing
}

impl Settings {
    fn new() -> Self {
        return Self {
            // Camera
            camera_sensitivity: 1.0,
            zoom_sensitivity: 1.0,
            min_ppm: 10.0,
            max_ppm: 1000.0,
            // Gizmo
            is_drawing_normals: false,
            default_transform: false,
        };
    }
}

// Objects Transform
// 1] Default Transform
// Starting Transform -> No Physic

// 2] Physic Transform
// Evolved by Physics System

// Deal with Physics / No Physics
// 1] Freeze
// Keep the Physic When Paused
// On Pause -> Objects Freeze

// 2] Default
// Reset to Default Transform

pub struct MumperState {
    // View
    viewport: Rect,
    viewport_painter: Painter,
    smoothed_fps: f32,
    // World
    pub ppm: f32, // Pixel Per Meter = Zoom value
    pub camera_position: Vec2,
    camera_size_x: f32,
    camera_size_y: f32,
    physics: Arc<Mutex<MumperPhysics>>,
    pub is_paused: Arc<AtomicBool>, // only pause physics but not rendering
    // Objects Default Transform
    default_positions: Vec<Vec2>,
    default_rotations: Vec<f32>,
    default_scales: Vec<Vec2>,
    // Objects Rendering
    positions: Vec<Vec2>,
    calculated_vertices: Vec<Vec<Vec2>>,
    strokes: Vec<Stroke>,
}

// Scene or EngineState
impl MumperState {
    fn new(viewport: Rect, viewport_painter: Painter) -> Self {
        let physics: Arc<Mutex<MumperPhysics>> = Arc::new(Mutex::new(MumperPhysics::new(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )));

        let is_paused: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        Self::start_physic_thread(&physics, &is_paused);

        return Self {
            // View
            viewport,
            viewport_painter,
            smoothed_fps: 0.0,
            // World
            ppm: 100.0,
            camera_position: Vec2::ZERO,
            camera_size_x: 4.0,
            camera_size_y: 4.0,
            physics,
            is_paused,
            // Objects Default Transform
            default_positions: vec![],
            default_rotations: vec![],
            default_scales: vec![],
            // Objects Rendering
            positions: vec![],
            calculated_vertices: vec![],
            strokes: vec![],
        };
    }

    fn start_physic_thread(physics: &Arc<Mutex<MumperPhysics>>, is_paused: &Arc<AtomicBool>) {
        let physics_thread = Arc::clone(physics);
        let is_paused_thread = Arc::clone(is_paused);

        #[cfg(not(target_arch = "wasm32"))] // Spawn Thread on Desktop only
        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f32();
                last_tick = now;

                if is_paused_thread.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(16));
                    continue;
                }

                {
                    let mut physics = physics_thread.lock().unwrap();
                    physics.tick(dt);
                }

                thread::sleep(Duration::from_millis(8));
            }
        });
    }

    pub fn pause_physic(&mut self, is_paused: bool) {
        self.is_paused.store(is_paused, Ordering::Relaxed);
    }

    // RENDERING

    // world_to_screen :
    // 1] Viewport = Camera view
    // let clip_space = (camera.x * -1 + object.position.x) * ppm

    // 2] Camera view -> Viewport
    // 1) world_to_screen
    // let camera_left = camera_position - camera_size_x / 2;
    // let object_viewport_position_x = (-1 * camera_left + world_pos.x) / camera_size_x;
    pub fn world_to_screen(&self, world_pos: glam::Vec2) -> Pos2 {
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

    pub fn screen_to_world(&self, screen_pos: Pos2) -> Vec2 {
        let camera_position = self.camera_position;

        let camera_left = camera_position.x - self.camera_size_x / 2.0;
        let camera_bot = camera_position.y - self.camera_size_y / 2.0;

        let ppm = self.ppm as f32;
        let world_pos_x = camera_left + screen_pos.x / ppm;
        let world_pos_y = camera_bot + (self.viewport.height() - screen_pos.y) / ppm; // y inverted

        return Vec2::new(world_pos_x, world_pos_y);
    }

    pub fn render_frame(&mut self, settings: &Settings) {
        self.draw_origin();

        let is_physics_paused = self.is_paused.load(Ordering::Relaxed);

        // Get Render Data = Position & Vertices
        // From MumperPhysics or Default
        'get_physics_data: {
            let physics = self.physics.lock().unwrap();

            if is_physics_paused && settings.default_transform {
                self.calculated_vertices = physics.vertices.clone();
                break 'get_physics_data;
            }

            self.calculated_vertices = physics.calculated_vertices.clone();
        };

        // Draw normals
        if settings.is_drawing_normals {
            self.draw_normals();
        }

        // Render Shapes
        for i in 0..self.calculated_vertices.len() {
            if is_physics_paused && settings.default_transform {
                let default_image = MumperPhysics::image_vertices(
                    self.default_positions[i],
                    self.default_rotations[i],
                    self.default_scales[i],
                    &self.calculated_vertices[i],
                );

                self.render_shape(&default_image, self.strokes[i]);
                continue;
            }

            self.render_shape(&self.calculated_vertices[i], self.strokes[i]);
        }
    }

    // Draw an edge between each vertices
    fn render_shape(&self, vertices: &Vec<glam::Vec2>, stroke: Stroke) {
        for index in 0..vertices.len() {
            let end_index = (index + 1) % vertices.len();

            let start_world_pos = vertices[index];
            let end_world_pos = vertices[end_index];

            // println!("Vertex = {start_world_pos}");

            let start_pos = self.world_to_screen(start_world_pos);
            let end_pos = self.world_to_screen(end_world_pos);

            // draw_edge
            self.viewport_painter
                .line_segment([start_pos, end_pos], stroke);
        }
    }

    pub fn render_vector(&self, origin: Vec2, vector: Vec2, stroke: Stroke) {
        // TODO
        let start_pos = self.world_to_screen(origin);
        let end_pos = self.world_to_screen(origin + vector);

        // Draw segment
        self.viewport_painter
            .line_segment([start_pos, end_pos], stroke);

        // Draw vector head
        let vector_head = Rect::from_center_size(end_pos, vec2(10.0, 10.0));
        self.viewport_painter
            .rect_filled(vector_head, 0.0, stroke.color);
    }

    pub fn draw_origin(&self) {
        // X
        self.render_vector(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Stroke::new(1.0, egui::Color32::RED),
        );
        // Y
        self.render_vector(
            Vec2::ZERO,
            Vec2::new(0.0, 1.0),
            Stroke::new(1.0, egui::Color32::GREEN),
        );
    }

    pub fn draw_normals(&self) {
        for i in 0..self.calculated_vertices.len() {
            let vertices = &self.calculated_vertices[i];

            // for each object's vertices
            for j in 0..vertices.len() {
                if i >= vertices.len() {
                    continue;
                }

                let vertex: Vec2 = vertices[j];

                // Edge normals
                let next_index = (j + 1) % vertices.len();
                let next_vertex = vertices[next_index];

                let edge_vector = next_vertex - vertex;
                let edge_normal = MumperPhysics::vector_normal(edge_vector);

                let normal_pos = gears::get_average_point(vertex, next_vertex);

                self.render_vector(
                    normal_pos,
                    edge_normal,
                    Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                );
            }
        }
    }

    // SCENE

    /// Create an Object
    pub fn create_shape(
        &mut self,
        vertices: Vec<Vec2>,
        radius: f32,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        velocity: Vec2,
        rotation_speed: f32,
        bounciness: f32,
        stroke: Stroke,
    ) {
        // println!("Create Shape at : {position}");

        self.default_positions.push(position.clone());
        self.default_rotations.push(rotation.clone());
        self.default_scales.push(scale.clone());

        let default_image = MumperPhysics::image_vertices(
            position.clone(),
            rotation.clone(),
            scale.clone(),
            &vertices,
        );

        // Add Shape to Physic engine
        {
            let mut physics = self.physics.lock().unwrap();
            // Object
            physics.vertices.push(vertices);
            physics.edge_normals.push(vec![]);
            physics.calculated_vertices.push(default_image);
            // Transform
            physics.positions.push(position);
            physics.rotations.push(rotation);
            physics.scales.push(scale);
            // Collision
            physics.radiuses.push(radius);
            // Physic
            physics.velocities.push(velocity);
            physics.rotation_speeds.push(rotation_speed);
            physics.bounciness.push(bounciness);
        };

        self.strokes.push(stroke);
    }

    pub fn clear_polygons(&mut self) {
        self.positions.clear();
        self.calculated_vertices.clear();
        self.strokes.clear();
    }
}

use eframe::egui::*;
use glam::Vec2;
use std::sync::atomic::Ordering;

use crate::Mumper;
use crate::gears;
use crate::mumper_ecs::*;

// TODO : Double-Buffered State
// use components?
// pub struct PhysicsStateUpdate {
//     pub entity_id: u32,
//     pub position: Vec2,
//     pub rotation: f32,
// }

// // Wrap a vector of these updates in an Arc<Mutex>
// pub type SharedPhysicsBuffer = Arc<Mutex<Vec<PhysicsStateUpdate>>>;

pub struct MumperRenderer {
    // View
    pub viewport: Rect,
    pub viewport_painter: Painter,
    pub smoothed_fps: f32,
    // Camera
    pub ppm: f32, // Pixel Per Meter = Zoom value
    pub camera_position: Vec2,
    pub camera_size_x: f32,
    pub camera_size_y: f32,
    // Entities Rendering
    pub shape_renderer_storage: ShapeRendererStorage,
    pub normals_renderer_storage: NormalsRendererStorage,
}

impl MumperRenderer {
    pub fn new(viewport: Rect, viewport_painter: Painter) -> Self {
        let shape_renderer_storage = ShapeRendererStorage::new();
        let normals_renderer_storage = NormalsRendererStorage::new();

        Self {
            // View
            viewport,
            viewport_painter,
            smoothed_fps: 0.0,
            // Camera
            ppm: 100.0,
            camera_position: Vec2::ZERO,
            camera_size_x: 4.0,
            camera_size_y: 4.0,
            // Entities Rendering
            shape_renderer_storage,
            normals_renderer_storage,
        }
    }

    pub fn rendering(state: &mut Mumper, ui: &mut Ui) -> Response {
        // Draw Area
        let (response, painter) = ui.allocate_painter(
            ui.available_size(), // All remaining space
            Sense::click(),
        );
        let rect = response.rect;

        state.renderer.viewport = rect;
        state.renderer.viewport_painter = painter;

        // Border
        state.renderer.viewport_painter.rect_stroke(
            rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::GREEN),
            egui::StrokeKind::Middle,
        );

        Self::render_frame(state);

        return response;
    }

    pub fn render_frame(state: &mut Mumper) {
        Self::draw_origin(&state.renderer);

        let is_physics_paused = state.is_paused.load(Ordering::Relaxed);

        // Get Render Data = Position & Vertices From MumperPhysics
        'get_physics_data: {
            let physics = state.physics.lock().unwrap();

            // if is_physics_paused && settings.default_transform {
            //     self.calculated_vertices = physics.vertices.clone();
            //     break 'get_physics_data;
            // }

            // Get calculated_vertices
            state.renderer.shape_renderer_storage.calculated_vertices =
                physics.shape_storage.calculated_vertices.clone();
            
            // Get Normals
            state.renderer.normals_renderer_storage = physics.normals_renderer_storage.clone();

            // Get Transform
            state.transform_storage = physics.transform_storage.clone();
        };

        // Draw normals
        let settings = &state.settings;
        if settings.is_drawing_normals {
            Self::draw_normals(&state.renderer);
        }

        Self::render_shape_components_logic(state);
    }

    fn render_shape_components_logic(state: &mut Mumper) {
        // Render Shape Component Logic
        for i in 0..state.renderer.shape_renderer_storage.entities.len() {
            Self::render_shape(
                &state.renderer,
                &state.renderer.shape_renderer_storage.calculated_vertices[i],
                state.renderer.shape_renderer_storage.strokes[i],
            );
        }
    }

    // Draw a Segment between each vertices
    fn render_shape(renderer: &MumperRenderer, vertices: &Vec<glam::Vec2>, stroke: Stroke) {
        for index in 0..vertices.len() {
            let end_index = (index + 1) % vertices.len();

            let start_world_pos = vertices[index];
            let end_world_pos = vertices[end_index];

            // println!("Vertex = {start_world_pos}");

            let start_pos = Self::world_to_screen(&renderer, &start_world_pos);
            let end_pos = Self::world_to_screen(&renderer, &end_world_pos);

            // draw_edge
            renderer
                .viewport_painter
                .line_segment([start_pos, end_pos], stroke);
        }
    }

    pub fn render_vector(renderer: &MumperRenderer, origin: Vec2, vector: Vec2, stroke: Stroke) {
        // TODO
        let start_pos = Self::world_to_screen(&renderer, &origin);
        let end_world_pos = origin + vector;
        let end_pos = Self::world_to_screen(&renderer, &end_world_pos);

        // Draw segment
        renderer
            .viewport_painter
            .line_segment([start_pos, end_pos], stroke);

        // Draw vector head
        let vector_head = Rect::from_center_size(end_pos, vec2(10.0, 10.0));
        renderer
            .viewport_painter
            .rect_filled(vector_head, 0.0, stroke.color);
    }

    pub fn draw_origin(renderer: &MumperRenderer) {
        // X
        Self::render_vector(
            renderer,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Stroke::new(1.0, egui::Color32::RED),
        );

        // Y
        Self::render_vector(
            renderer,
            Vec2::ZERO,
            Vec2::new(0.0, 1.0),
            Stroke::new(1.0, egui::Color32::GREEN),
        );
    }

    pub fn draw_normals(renderer: &MumperRenderer) {
        // foreach entity
        for i in 0..renderer.normals_renderer_storage.entities.len() {
            // foreach segment
            for j in 0..renderer.normals_renderer_storage.normal_pos[i].len() {
                Self::render_vector(
                    renderer,
                    renderer.normals_renderer_storage.normal_pos[i][j],
                    renderer.normals_renderer_storage.edge_normals[i][j],
                    Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                );
            }
        }
    }

    // UTILS

    // world_to_screen :
    // 1] Viewport = Camera view
    // let clip_space = (camera.x * -1 + object.position.x) * ppm

    // 2] Camera view -> Viewport
    // 1) world_to_screen
    // let camera_left = camera_position - camera_size_x / 2;
    // let object_viewport_position_x = (-1 * camera_left + world_pos.x) / camera_size_x;
    pub fn world_to_screen(renderer: &MumperRenderer, world_pos: &glam::Vec2) -> Pos2 {
        let camera_position = renderer.camera_position;
        let camera_size_x = renderer.camera_size_x;
        let camera_size_y = renderer.camera_size_y;

        // 2] Camera view -> Viewport
        let camera_left = camera_position.x - camera_size_x / 2.0;
        let camera_bot = camera_position.y - camera_size_y / 2.0;

        let fulcrum_x = (-1.0 * camera_left + world_pos.x) / camera_size_x;
        let fulcrum_y = 1.0 - (-1.0 * camera_bot + world_pos.y) / camera_size_y; // y inverted = ui

        let screen_pos_x = fulcrum_x * renderer.viewport.width();
        let screen_pos_y = fulcrum_y * renderer.viewport.height();

        let screen_pos = Pos2::new(screen_pos_x, screen_pos_y);
        return screen_pos;
    }

    pub fn screen_to_world(renderer: &MumperRenderer, screen_pos: &Pos2) -> Vec2 {
        let camera_position = renderer.camera_position;

        let camera_left = camera_position.x - renderer.camera_size_x / 2.0;
        let camera_bot = camera_position.y - renderer.camera_size_y / 2.0;

        let ppm = renderer.ppm as f32;
        let world_pos_x = camera_left + screen_pos.x / ppm;
        let world_pos_y = camera_bot + (renderer.viewport.height() - screen_pos.y) / ppm; // y inverted

        return Vec2::new(world_pos_x, world_pos_y);
    }
}

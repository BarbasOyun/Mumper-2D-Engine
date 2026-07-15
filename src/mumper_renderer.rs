use eframe::{CreationContext, egui::*};
use glam::Vec2;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::Mumper;
use crate::MumperPhysics;
use crate::gears;

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
    pub calculated_vertices: Vec<Vec<Vec2>>,
    pub edge_normals: Vec<Vec<Vec2>>,
    pub strokes: Vec<Stroke>,
}

impl MumperRenderer {
    pub fn new(viewport: Rect, viewport_painter: Painter) -> Self {
        // let mut calculated_vertices = vec![];
        // let mut edge_normals = vec![];

        // for _ in 0..vertices.len() {
        //     calculated_vertices.push(vec![]);
        //     edge_normals.push(vec![]);
        // }

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
            calculated_vertices: vec![],
            edge_normals: vec![],
            strokes: vec![],
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

            // TODO : Double-Buffered State
            // TODO : Get Transforms + calculated_vertices from physics

            // if is_physics_paused && settings.default_transform {
            //     self.calculated_vertices = physics.vertices.clone();
            //     break 'get_physics_data;
            // }

            state.renderer.calculated_vertices = physics.calculated_vertices.clone();

            state.ecs.transform_storage = physics.transform_storage.clone();
        };

        // Draw normals
        let settings = &state.settings;
        if settings.is_drawing_normals {
            Self::draw_normals(&state.renderer);
        }

        // Render Shapes
        for i in 0..state.renderer.calculated_vertices.len() {
            // if is_physics_paused && settings.default_transform {
            //     let default_image = MumperPhysics::image_vertices(
            //         self.default_positions[i],
            //         self.default_rotations[i],
            //         self.default_scales[i],
            //         &self.calculated_vertices[i],
            //     );

            //     self.render_shape(&default_image, self.strokes[i]);
            //     continue;
            // }

            Self::render_shape(
                &state.renderer,
                &state.renderer.calculated_vertices[i],
                state.renderer.strokes[i],
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

            let start_pos = Self::world_to_screen(&renderer, start_world_pos);
            let end_pos = Self::world_to_screen(&renderer, end_world_pos);

            // draw_edge
            renderer
                .viewport_painter
                .line_segment([start_pos, end_pos], stroke);
        }
    }

    pub fn render_vector(renderer: &MumperRenderer, origin: Vec2, vector: Vec2, stroke: Stroke) {
        // TODO
        let start_pos = Self::world_to_screen(&renderer, origin);
        let end_pos = Self::world_to_screen(&renderer, origin + vector);

        // Draw segment
        renderer.viewport_painter
            .line_segment([start_pos, end_pos], stroke);

        // Draw vector head
        let vector_head = Rect::from_center_size(end_pos, vec2(10.0, 10.0));
        renderer.viewport_painter
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
        for i in 0..renderer.calculated_vertices.len() {
            let vertices = &renderer.calculated_vertices[i];

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
                let edge_normal = gears::vector_normal(edge_vector);

                let normal_pos = gears::get_average_point(vertex, next_vertex);

                Self::render_vector(
                    renderer,
                    normal_pos,
                    edge_normal,
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
    pub fn world_to_screen(renderer: &MumperRenderer, world_pos: glam::Vec2) -> Pos2 {
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

    pub fn screen_to_world(renderer: &MumperRenderer, screen_pos: Pos2) -> Vec2 {
        let camera_position = renderer.camera_position;

        let camera_left = camera_position.x - renderer.camera_size_x / 2.0;
        let camera_bot = camera_position.y - renderer.camera_size_y / 2.0;

        let ppm = renderer.ppm as f32;
        let world_pos_x = camera_left + screen_pos.x / ppm;
        let world_pos_y = camera_bot + (renderer.viewport.height() - screen_pos.y) / ppm; // y inverted

        return Vec2::new(world_pos_x, world_pos_y);
    }
}

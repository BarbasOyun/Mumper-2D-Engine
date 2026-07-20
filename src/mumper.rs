use eframe::{CreationContext, egui::*};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread; // Doesn't work on Browser
use std::time::{Duration, Instant};

use crate::MumperPhysics;
use crate::MumperRenderer;
use crate::gears;
use crate::mumper_ecs::*;

// Settings + Rendering
pub struct Mumper {
    pub settings: Settings,
    // editor_state: EditorState, (Camera)
    // ECS
    pub ecs: MumperECS,
    pub components: Components,
    // Physics
    pub physics: Arc<Mutex<MumperPhysics>>,
    pub is_paused: Arc<AtomicBool>, // only pause physics but not rendering
    // Renderer
    pub renderer: MumperRenderer,
}

impl Mumper {
    pub fn new(cc: &CreationContext) -> Self {
        // ECS
        let mut ecs = MumperECS::new();
        let components = Components::new(&mut ecs);

        // Physics
        let physics: Arc<Mutex<MumperPhysics>> = Arc::new(Mutex::new(MumperPhysics::new()));
        let is_paused: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        Self::start_physic_thread(&physics, &is_paused);

        // Renderer
        let renderer = MumperRenderer::new(&mut ecs, cc.egui_ctx.content_rect(), cc.egui_ctx.debug_painter());

        Self {
            settings: Settings::new(),
            // ECS
            ecs,
            components,
            // Physics
            physics,
            is_paused,
            // Renderer
            renderer,
        }
    }

    pub fn reset_settings(&mut self) {
        self.settings = Settings::new();
    }

    pub fn clear_scene(&mut self) {
        MumperECS::clear_entities(self);
    }

    // UPDATE

    // param = ui closure + input closure
    // TODO : game_update()
    // fps
    // ui closure
    // render_frame
    // input closure

    pub fn update(&mut self, ui: &Ui) -> (f32, f32) {
        ui.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS
        let dt = ui.input(|i| i.stable_dt); // DeltaTime in second
        let fps = 1.0 / dt;

        // Web : Tick Physics
        #[cfg(target_arch = "wasm32")]
        {
            if self.is_paused.load(std::sync::atomic::Ordering::Relaxed) {
                // continue;
                return (dt, fps);
            }

            let mut physics = self.physics.lock().unwrap();

            physics.tick(dt);
        }

        return (dt, fps);
    }

    pub fn game_update(&mut self, ui: &mut Ui) -> Response {
        return MumperRenderer::rendering(self, ui);
    }

    // Displayed on top of the viewport
    pub fn hud(&mut self, fps: f32) {
        let renderer = &mut self.renderer;
        let painter = &mut renderer.viewport_painter;

        // FPS Display
        let alpha = 0.05;
        renderer.smoothed_fps = (renderer.smoothed_fps * (1.0 - alpha)) + (fps * alpha);

        painter.text(
            renderer.viewport.left_top() + egui::vec2(10.0, 10.0), // 10px padding from top-left
            egui::Align2::LEFT_TOP,
            format!("FPS: {:.2}", renderer.smoothed_fps),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        // Entities Count Display
        let entities_count = self.ecs.entity_ids.len();

        painter.text(
            renderer.viewport.left_top() + egui::vec2(10.0, 30.0),
            egui::Align2::LEFT_TOP,
            format!("Entities: {}", entities_count),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        // Controls Display
        painter.text(
            renderer.viewport.left_top() + egui::vec2(10.0, 50.0),
            egui::Align2::LEFT_TOP,
            "Look : Right Click",
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    pub fn camera_controls(&mut self, input_state: &InputState) {
        let settings = &mut self.settings;
        let renderer = &mut self.renderer;

        let pointer_delta: egui::Vec2 = input_state.pointer.delta();
        let rclick_hold = input_state.pointer.secondary_down();

        let ppm = renderer.ppm as f32;
        // Camera limits -> Depend on viewport size
        renderer.camera_size_x = renderer.viewport.width() / ppm;
        renderer.camera_size_y = renderer.viewport.height() / ppm;

        // Mousewheel = Zoom
        let mut scroll_delta =
            input_state.smooth_scroll_delta.y * settings.zoom_sensitivity * ppm * 0.003; // Notch based zoom

        if scroll_delta != 0.0 {
            gears::reverse_clamp(&mut scroll_delta, -0.1, 0.1);
            renderer.ppm = (renderer.ppm + scroll_delta).clamp(settings.min_ppm, settings.max_ppm);
        }

        // RClick = Move Camera
        if rclick_hold {
            let sensivity =
                settings.camera_sensitivity * (settings.max_ppm / renderer.ppm) as f32 * 0.001;
            renderer.camera_position.x -= pointer_delta.x * sensivity;
            renderer.camera_position.y += pointer_delta.y * sensivity;
        }
    }

    fn start_physic_thread(physics: &Arc<Mutex<MumperPhysics>>, is_paused: &Arc<AtomicBool>) {
        let physics_thread = Arc::clone(physics);
        let is_paused_thread = Arc::clone(is_paused);

        // TODO : Common Physic Data Buffer

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

    // SCENE

    pub fn pause_physic(&mut self, is_paused: bool) {
        self.is_paused.store(is_paused, Ordering::Relaxed);
    }
}

// Entity Transform
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

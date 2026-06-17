use glam::*;

// Get a position on a Circle
// progress = (progress + 0.01) % (math.pi * 2)
pub fn circle_pos(radius: f32, progress: f32) -> Vec2 {
    let x = progress.cos() * radius;
    let y = progress.sin() * radius;
    return Vec2 { x, y };
}

// Get the points making a circle
pub fn circle_points(base_pos: Vec2, radius: f32, segments: u16) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = vec!{};
    let point_distance = (std::f32::consts::PI * 2.0) / segments as f32;

    for i in 0..segments {
        points.push(base_pos + circle_pos(radius, point_distance * i as f32));
    }

    return points;
}
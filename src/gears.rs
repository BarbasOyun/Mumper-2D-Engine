use glam::*;

// Get a position on a Circle
// progress = (progress + 0.01) % (math.pi * 2)
pub fn circle_pos(radius: f32, progress: f32) -> Vec2 {
    let x = progress.cos();
    let y = progress.sin();
    let pos = Vec2 { x, y } * radius;
    return pos;
}

// Get the points making a circle
pub fn circle_vertices(radius: f32, segments: u16) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = vec![];
    let point_distance = (2.0 * std::f32::consts::PI) / segments as f32;

    // let start_distance = std::f32::consts::PI / 2.0 - std::f32::consts::PI / segments as f32;
    let start_distance = (3.0 * std::f32::consts::PI) / 2.0 + std::f32::consts::PI / segments as f32;

    for i in 0..segments {
        points.push(circle_pos(radius, start_distance + point_distance * i as f32));
    }

    return points;
}
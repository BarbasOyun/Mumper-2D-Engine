use glam::*;

// MATHS

// keep the value outside the specified values
// TODO : <T> where t is number
pub fn reverse_clamp(value: &mut f32, min: f32, max: f32) {
    if min < *value && *value < max {
        let delta_min = (min - *value).abs();
        let delta_max = (max - *value).abs();

        if delta_min < delta_max {
            *value = min;
        } else {
            *value = max;
        }
    }
}

// get the point between 2 points
pub fn get_average_point(point1: Vec2, point2: Vec2) -> Vec2 {
    let average_x = (point1.x + point2.x) / 2.0;
    let average_y = (point1.y + point2.y) / 2.0;

    return Vec2::new(average_x, average_y);
}

// GEOMETRY

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
    let start_distance =
        (3.0 * std::f32::consts::PI) / 2.0 + std::f32::consts::PI / segments as f32;

    for i in 0..segments {
        points.push(circle_pos(
            radius,
            start_distance + point_distance * i as f32,
        ));
    }

    return points;
}

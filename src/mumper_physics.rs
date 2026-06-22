use glam::Vec2;

// Shared between Rendering & Physic Threads
pub struct MumperPhysics {
    // Objects Data
    // TODO : ECS
    // Smart IDs
    pub radiuses: Vec<f32>,
    pub vertices: Vec<Vec<Vec2>>, // TODO : Flatten
    pub calculated_vertices: Vec<Vec<Vec2>>,
    pub edge_normals: Vec<Vec<Vec2>>,
    // Transforms
    pub positions: Vec<Vec2>,
    pub rotations: Vec<f32>, // 2D Object rotate only on Z axe
    pub scales: Vec<Vec2>,
    pub velocities: Vec<Vec2>, // meters / sec
    pub rotation_speeds: Vec<f32>,
    // Rigid bodies
    pub bounciness: Vec<f32>,
}

impl MumperPhysics {
    pub fn new(
        radiuses: Vec<f32>,
        vertices: Vec<Vec<Vec2>>,
        positions: Vec<Vec2>,
        rotations: Vec<f32>,
        scales: Vec<Vec2>,
        velocities: Vec<Vec2>,
        rotation_speeds: Vec<f32>,
        bounciness: Vec<f32>,
    ) -> Self {
        let mut calculated_vertices = vec![];
        let mut edge_normals = vec![];

        for _ in 0..vertices.len() {
            calculated_vertices.push(vec![]);
            edge_normals.push(vec![]);
        }

        return Self {
            radiuses,
            vertices,
            calculated_vertices,
            positions,
            edge_normals,
            rotations,
            scales,
            velocities,
            rotation_speeds,
            bounciness,
        };
    }

    // PHYSICS UPDATE

    pub fn tick(&mut self, dt: f32) {
        let square_lines_thickness = 0.1;

        // Object Collision Data
        let mut object_collisions1: Vec<usize> = vec![]; // Objects Index
        let mut object_collisions2: Vec<usize> = vec![];
        let mut collisions_normals: Vec<Vec2> = vec![];
        let mut collisions_penetration_depth: Vec<f32> = vec![];

        // for each object
        for i in 0..self.positions.len() {
            // object properties
            let velocity = &mut self.velocities[i];
            let rotation = &mut self.rotations[i];
            let rotation_speed = &mut self.rotation_speeds[i];
            let scale = &mut self.scales[i];
            let base_vertices = &self.vertices[i];
            let bounciness = &self.bounciness[i];

            // 1] Transforms
            Self::transform(
                &dt,
                &mut self.positions[i],
                velocity,
                rotation,
                rotation_speed,
            );

            // 2] Frame Image -> vertices * model matrix
            let calculated_vertices =
                Self::image_vertices(self.positions[i], *rotation, *scale, base_vertices);
            self.calculated_vertices[i] = calculated_vertices;

            // 3] Calculate Edges normal
            let vertices = &self.calculated_vertices[i];

            let edge_normals = Self::edges_normal(vertices);
            self.edge_normals[i] = edge_normals;

            // 4] Collisions
            // Walls Collisions
            let square = &self.calculated_vertices[0];
            Self::wall_collisions(
                &mut self.radiuses[i],
                &mut self.positions[i],
                velocity,
                bounciness,
                square,
                &square_lines_thickness,
            );

            // Objects Collision
            const SOLVER_ITERATIONS: usize = 6;
            // 1] Collision Detection
            let mut index_list: Vec<usize> = vec![]; // Indexes already captured

            for j in 0..object_collisions2.len() {
                if object_collisions2[j] == i {
                    index_list.push(object_collisions1[j]);
                }
            }

            // for each other object -> detect collision
            for j in 0..self.positions.len() {
                if j == i || index_list.contains(&j) {
                    continue;
                }

                let other_object_pos = self.positions[j];
                let direction = self.positions[i] - other_object_pos;
                let distance = direction.length();
                let distance_threshold = self.radiuses[i] + self.radiuses[j];

                if distance <= distance_threshold {
                    // Collision
                    println!("Collision Detected");

                    let penetration_depth = distance - self.radiuses[i];

                    object_collisions1.push(i);
                    object_collisions2.push(i);
                    collisions_normals.push(direction.normalize());
                    collisions_penetration_depth.push(penetration_depth);
                }
            }
        }

        // 2] Collisions Solver
        // for each collision
        // for mut i in 0..object_collisions1.len() {
        //     let mut next_index = (i + 1) % object_collisions1.len();

        //     let mut collision_normals: Vec<Vec2> = vec![];
        //     collision_normals.push(collision_normals[i]);

        //     // every identical index are next to each other
        //     while i == object_collisions1[next_index] {
        //         collision_normals.push(collision_normals[next_index]);
        //         i += 1; // Advance the for loop
        //         next_index += 1;
        //     }

        //     // Add all normals
        //     let mut sum_normals = Vec2::ZERO;

        //     for j in 0..collision_normals.len() {
        //         sum_normals += collision_normals[j];
        //     }

        //     let escape_direction = sum_normals.normalize() * -1.0; // reverse
        //     let mut escape_scalar = 0.0;

        //     for j in 0..collisions_penetration_depth.len() {
        //         escape_scalar += collisions_penetration_depth[j];
        //     }

        //     self.positions[i] += escape_direction * escape_scalar;
        // }
    }

    // Take an object and apply its transform -> called every frame
    fn transform(
        dt: &f32,
        position: &mut Vec2,
        velocity: &Vec2,
        rotation: &mut f32,
        rotation_speed: &f32,
    ) {
        // Position
        // Apply Velocity
        let velocity_frame = *velocity * dt;

        position.x += velocity_frame.x;
        position.y += velocity_frame.y;

        // Gravity
        // pos.y -= 9.81 * dt;

        // Rotation
        *rotation += rotation_speed * dt;
    }

    // Multiply base vertices with model matrix
    // return calculated_vertices
    fn image_vertices(
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        base_vertices: &Vec<Vec2>,
    ) -> Vec<Vec2> {
        let mut calculated_vertices = vec![];
        let model_matrix = glam::Mat3::from_scale_angle_translation(scale, rotation, position);

        // for each base vertices
        for j in 0..base_vertices.len() {
            let vertex = base_vertices[j];
            let homogeneous_vertex = vertex.extend(1.0);

            let transformed_vertex_3d = model_matrix * homogeneous_vertex;

            let world_position: Vec2 = transformed_vertex_3d.truncate();
            calculated_vertices.push(world_position);
        }

        return calculated_vertices;
    }

    // Calculate and return the normals of edges
    fn edges_normal(vertices: &Vec<Vec2>) -> Vec<Vec2> {
        let mut edge_normals = vec![];

        for j in 0..vertices.len() {
            let vertex = vertices[j];
            let next_index = (j + 1) % vertices.len();
            let next_vertex = vertices[next_index];

            let edge_vector = next_vertex - vertex;
            let edge_normal = Self::vector_normal(edge_vector);

            edge_normals.push(edge_normal);
        }

        return edge_normals;
    }

    fn wall_collisions(
        radius: &f32,
        position: &mut Vec2,
        velocity: &mut Vec2,
        bounciness: &f32,
        square: &Vec<Vec2>,
        square_lines_thickness: &f32,
    ) {
        // Detection
        if *radius == 0.0 {
            return;
        }

        let distance_threshold = square_lines_thickness + radius;

        // for each square edge -> check collision
        for j in 0..square.len() {
            let next_index = (j + 1) % square.len();
            let square_vertex1 = square[j];
            let square_vertex2 = square[next_index];

            let edge_to_point = Self::edge_to_point(square_vertex1, square_vertex2, *position);
            let distance_edge = edge_to_point.length();

            // Solve Square
            if distance_edge <= distance_threshold {
                // println!("Collision with Edge : {j}");

                let collision_normal = edge_to_point / distance_edge;
                let penetration_depth = radius - distance_edge;

                // let vel_along_normal = velocity.dot(collision_normal);

                // if vel_along_normal < 0.0 {
                //     let impulse_scalar = -(1.0 + bounciness) * vel_along_normal;
                //     *velocity += collision_normal * impulse_scalar;

                //     *position += collision_normal * penetration_depth;
                // }

                Self::bounce(collision_normal, penetration_depth, velocity, bounciness, position);
            }
        }
    }

    // make an objetc bounce from a normal
    fn bounce(collision_normal: Vec2, penetration_depth: f32, velocity: &mut Vec2, bounciness: &f32, position: &mut Vec2) {
        let vel_along_normal = velocity.dot(collision_normal);

        if vel_along_normal < 0.0 {
            let impulse_scalar = -(1.0 + bounciness) * vel_along_normal; // TODO : Multiply by other object speed -> walls/static speed = 1
            *velocity += collision_normal * impulse_scalar;

            *position += collision_normal * penetration_depth;
        }
    }

    // UTILS

    // Get the vector between a point and its projection on an edge (limited size)
    pub fn edge_to_point(line_start: Vec2, line_end: Vec2, point: Vec2) -> Vec2 {
        let ab = line_end - line_start;
        let ap = point - line_start;

        let ab_len_sq = ab.length_squared();

        if ab_len_sq == 0.0 {
            return point - line_start;
        }

        let t = ap.dot(ab) / ab_len_sq;
        let t_clamped = t.clamp(0.0, 1.0);
        let closest_point = line_start + ab * t_clamped;

        let to_point = point - closest_point;

        return to_point;
    }

    // Detect if a point collide with a line (infinite)
    // use dot product between line_normal & point
    pub fn line_collision(line_start: Vec2, line_end: Vec2, thickness: f32, point: Vec2) -> bool {
        let line_direction = line_end - line_start;
        let line_normal = Self::vector_normal(line_direction);
        let ap = point - line_start;

        let distance = line_normal.dot(ap);

        return distance <= thickness;
    }

    // return the Counterclockwise normal of a 2D Vector
    pub fn vector_normal(vector: Vec2) -> Vec2 {
        // Clockwise
        // let vector_normal = Vec2::new(-vector.y, vector.x).normalize();
        // Counterclockwise
        let vector_normal = Vec2::new(vector.y, -vector.x).normalize();

        return vector_normal;
    }
}

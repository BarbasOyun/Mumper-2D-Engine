use glam::Vec2;

use crate::Mumper;
use crate::mumper_ecs::TransformStorage;

const SOLVER_ITERATIONS: usize = 6;

// TODO :
// Quadtree -> Separate Space

pub struct MumperPhysics {
    pub transform_storage: TransformStorage, // Physics have its own version
    // Rigid bodies
    pub velocities: Vec<Vec2>, // meters / sec
    pub rotation_speeds: Vec<f32>,
    pub bounciness: Vec<f32>,
}

impl MumperPhysics {
    pub fn new(
        velocities: Vec<Vec2>,
        rotation_speeds: Vec<f32>,
        bounciness: Vec<f32>,
    ) -> Self {
        let transform_storage = TransformStorage::new();

        return Self {
            transform_storage,
            velocities,
            rotation_speeds,
            bounciness,
        };
    }

    // PHYSICS UPDATE

    pub fn tick(state: Mumper, dt: f32) {
        // TODO :
        // Physics side
        // 1) foreach collider(use calculated_vertices) -> Build collisions data
        // 2) foreach rigidbody -> Physics + Change Transform (Use Collisions)
        // Rendering side
        // 3) Calculate vertex

        let square_lines_thickness = 0.1;

        // Object Collisions Data
        let mut object_collisions1: Vec<usize> = vec![]; // Objects Index
        let mut object_collisions2: Vec<usize> = vec![];
        let mut collisions_normals: Vec<Vec2> = vec![];
        let mut collisions_penetration_depth: Vec<f32> = vec![];

        // for each object
        for i in 0..state.ecs.transform_storage.entities.len() {
            // object properties
            let velocity = &mut self.velocities[i];
            let rotation = &mut self.transform_storage.rotations[i];
            let rotation_speed = &mut self.rotation_speeds[i];
            let scale = &mut self.transform_storage.scales[i];

            let base_vertices = &self.vertices[i];
            let bounciness = &self.bounciness[i];

            Self::transform(
                &dt,
                &mut self.transform_storage.positions[i],
                velocity,
                rotation,
                rotation_speed,
            );

            // 2] Frame Image -> vertices * model matrix
            let calculated_vertices = Self::image_vertices(
                self.transform_storage.positions[i],
                *rotation,
                *scale,
                base_vertices,
            );
            self.calculated_vertices[i] = calculated_vertices;

            // 3] Calculate Edges normal
            let vertices = &self.calculated_vertices[i];

            let edge_normals = Self::edges_normal(vertices);
            self.edge_normals[i] = edge_normals;

            // 4] Collisions
            if self.radius_collider_storage.radiuses[i] == 0.0 {
                continue;
            }

            // Walls Collisions
            let square = &self.calculated_vertices[0];
            Self::wall_collisions(
                &mut self.radius_collider_storage.radiuses[i],
                &mut self.transform_storage.positions[i],
                velocity,
                bounciness,
                square,
                &square_lines_thickness,
            );

            // 1] Collision Detection
            self.object_collisions(
                i,
                &mut object_collisions1,
                &mut object_collisions2,
                &mut collisions_normals,
                &mut collisions_penetration_depth,
            );
        }

        // 2] Solve Collisions
        self.collision_solver(
            &mut object_collisions1,
            &mut object_collisions2,
            &mut collisions_normals,
            &mut collisions_penetration_depth,
        );
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
    pub fn image_vertices(
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

                Self::bounce(
                    collision_normal,
                    penetration_depth,
                    velocity,
                    bounciness,
                    position,
                );
            }
        }
    }

    // make an object bounce from a normal
    fn bounce(
        collision_normal: Vec2,
        penetration_depth: f32,
        velocity: &mut Vec2,
        bounciness: &f32,
        position: &mut Vec2,
    ) {
        let vel_along_normal = velocity.dot(collision_normal);

        if vel_along_normal < 0.0 {
            let impulse_scalar = -(1.0 + bounciness) * vel_along_normal;
            *velocity += collision_normal * impulse_scalar;

            *position += collision_normal * penetration_depth;
        }
    }

    // Check all the collisions of an Entity
    // Build Collisions list
    fn object_collisions(
        &mut self,
        entity_id: usize,
        object_collisions1: &mut Vec<usize>,
        object_collisions2: &mut Vec<usize>,
        collisions_normals: &mut Vec<Vec2>,
        collisions_penetration_depth: &mut Vec<f32>,
    ) {
        let mut ignore_list: Vec<usize> = vec![]; // Indexes already captured

        for i in 0..object_collisions2.len() {
            if object_collisions2[i] == entity_id {
                ignore_list.push(object_collisions1[i]);
            }
        }

        // for each other object -> detect collision
        for i in 0..self.transform_storage.entities.len() {
            if i == entity_id
                || self.radius_collider_storage.radiuses[i] == 0.0
                || ignore_list.contains(&i)
            {
                continue;
            }

            let object2_pos = self.transform_storage.positions[i];
            let direction = object2_pos - self.transform_storage.positions[entity_id]; // direction from object1 -> object2
            let distance = direction.length();
            let distance_threshold = self.radius_collider_storage.radiuses[entity_id]
                + self.radius_collider_storage.radiuses[i];

            if distance <= distance_threshold {
                // Collision
                let penetration_depth = distance - self.radius_collider_storage.radiuses[entity_id];

                object_collisions1.push(entity_id);
                object_collisions2.push(i);
                collisions_normals.push(direction.normalize());
                collisions_penetration_depth.push(penetration_depth);
            }
        }
    }

    // TODO : Collisions Context
    fn collision_solver(
        &mut self,
        object_collisions1: &mut Vec<usize>,
        object_collisions2: &mut Vec<usize>,
        collisions_normals: &mut Vec<Vec2>,
        collisions_penetration_depth: &mut Vec<f32>,
    ) {
        for _iteration in 0..SOLVER_ITERATIONS {
            for i in 0..object_collisions1.len() {
                // inv_mass = invariant mass
                let a_inv_mass = 1.0;
                let b_inv_mass = 1.0;
                let total_inv_mass = a_inv_mass + b_inv_mass;

                let index1 = object_collisions1[i];
                let index2 = object_collisions2[i];
                let penetration_depth = collisions_penetration_depth[i];
                let normal = collisions_normals[i];

                // If both objects are unmovable
                if total_inv_mass == 0.0 {
                    continue;
                }

                // 1] Positional Correction
                let percent = 0.05; // Resolve X% of the penetration per iteration
                let slop = 0.01; // Allow 1 centimeter of penetration before fixing

                let correction_magnitude =
                    (penetration_depth - slop).max(0.0) / total_inv_mass * percent;
                let correction_vector = normal * correction_magnitude;

                // Separation
                self.transform_storage.positions[index1] -= correction_vector * a_inv_mass;
                self.transform_storage.positions[index2] += correction_vector * b_inv_mass;

                // 2] Impulse Resolution
                // Relative velocity
                let rel_velocity = self.velocities[index2] - self.velocities[index1];

                let vel_along_normal = rel_velocity.dot(normal);

                // Do not resolve if velocities are already moving apart
                if vel_along_normal < 0.0 {
                    // Choose the lower bounciness between the two circles
                    let restitution = self.bounciness[index1].min(self.bounciness[index2]);

                    // Calculate impulse scalar
                    let mut impulse_scalar = -(1.0 + restitution) * vel_along_normal;
                    impulse_scalar /= total_inv_mass;

                    // Apply impulse to each circle
                    let impulse = normal * impulse_scalar;
                    self.velocities[index1] -= impulse * a_inv_mass;
                    self.velocities[index2] += impulse * b_inv_mass;
                }
            }
        }
    }

    // UTILS

    // Detect if a point collide with a line (infinite)
    // use dot product between line_normal & point
    pub fn line_collision(line_start: Vec2, line_end: Vec2, thickness: f32, point: Vec2) -> bool {
        let line_direction = line_end - line_start;
        let line_normal = Self::vector_normal(line_direction);
        let ap = point - line_start;

        let distance = line_normal.dot(ap);

        return distance <= thickness;
    }

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
}

// TODO :
// Quadtree -> Separate Space

use glam::Vec2;

use crate::gears;
use crate::mumper_ecs::*;
use crate::mumper_renderer::NormalsRendererStorage;

const SOLVER_ITERATIONS: usize = 6;

// Physics own a version of the physics components
pub struct MumperPhysics {
    pub transform_storage: TransformStorage,
    // Entities
    pub shape_storage: PhysicsShapeStorage,
    pub normals_renderer_storage: NormalsRendererStorage,
    // Colliders
    pub radius_collider_storage: RadiusColliderStorage,
    pub segments_collider_storage: SegmentColliderStorage,
    // Rigid bodies
    pub rigidbody_storage: RigidbodyStorage,
}

struct CollisionsData {
    // Rigidbody / Rigidbody Collisions
    rr_collisions1: Vec<usize>, // entity_id
    rr_collisions2: Vec<usize>,
    rr_collisions_normals: Vec<Vec2>,
    rr_collisions_penetration_depth: Vec<f32>,

    // Rigidbody / Static Collisions
    rs_collisions1: Vec<usize>,
    rs_collisions2: Vec<usize>,
    rs_collisions_normals: Vec<Vec2>,
    rs_collisions_penetration_depth: Vec<f32>,
}

impl CollisionsData {
    fn new() -> Self {
        return Self {
            // Rigidbody / Rigidbody Collisions
            rr_collisions1: vec![],
            rr_collisions2: vec![],
            rr_collisions_normals: vec![],
            rr_collisions_penetration_depth: vec![],

            // Rigidbody / Static Collisions
            rs_collisions1: vec![],
            rs_collisions2: vec![],
            rs_collisions_normals: vec![],
            rs_collisions_penetration_depth: vec![],
        };
    }
}

struct CollisionData {
    entity_id1: usize,
    entity_id2: usize,
    collision_normal: Vec2,
    collision_penetration_depth: f32,
}

struct EntityCollisionData<'a> {
    entity_id: &'a usize,
    position: &'a Vec2,
    ignore_list: &'a Vec<usize>,
    is_rigidbody: &'a bool,
}

impl MumperPhysics {
    pub fn new() -> Self {
        // Add physics components in the Physics world
        // Avoid incrementing ECS component count
        let transform_storage = TransformStorage::new(ComponentType::Transform as usize);

        let shape_storage = PhysicsShapeStorage::new(ComponentType::Renderer as usize);
        let normals_renderer_storage =
            NormalsRendererStorage::new(ComponentType::NormalsRenderer as usize);

        let radius_collider_storage =
            RadiusColliderStorage::new(ComponentType::RadiusCollider as usize);
        let segments_collider_storage =
            SegmentColliderStorage::new(ComponentType::SegmentsCollider as usize);
        let rigidbody_storage = RigidbodyStorage::new(ComponentType::Rigidbody as usize);

        return Self {
            transform_storage,
            shape_storage,
            normals_renderer_storage,
            radius_collider_storage,
            segments_collider_storage,
            rigidbody_storage,
        };
    }

    /* #region PHYSICS UPDATE */

    pub fn tick(&mut self, dt: f32) {
        // TODO :
        // 1) Calculate vertex //
        // 2) foreach collider(use calculated_vertices) -> Build collisions data
        // 3) foreach rigidbody -> Physics + Change Transform (Use Collisions)

        // println!("Physics Tick");

        self.shape_components_logic();
        self.normal_components_logic();

        // 1] Detect Collision
        // let square_lines_thickness = 0.1;

        let mut collisions_data = CollisionsData::new();

        self.radius_collider_components_logic(&mut collisions_data);
        // segment_collider_components_logic();
        self.rigidbody_component_logic(dt);

        // for each Entity
        // for i in 0..self.transform_storage.entities.len() {
        //     // Entity properties
        //     // Rigidbody
        //     let velocity = &mut self.rigidbody_storage.velocities[i];
        //     let bounciness = &self.rigidbody_storage.bounciness[i];

        //     // 4] Collisions
        //     // Walls Collisions
        //     let square = &self.shape_storage.calculated_vertices[0];
        //     Self::wall_collisions(
        //         &mut self.radius_collider_storage.radiuses[i],
        //         &mut self.transform_storage.positions[i],
        //         velocity,
        //         bounciness,
        //         square,
        //         &square_lines_thickness,
        //     );
        // }

        // 2] Solve Collisions
        self.rs_collisions_solver(&mut collisions_data);

        self.collision_solver(&mut collisions_data);
    }

    // Calculate Vertices
    fn shape_components_logic(&mut self) {
        for i in 0..self.shape_storage.entities.len() {
            // Get transform
            let entity_id = self.shape_storage.entities[i];
            let (position, rotation, scale) = self.transform_storage.get_component(entity_id);

            let calculated_vertices = Self::image_vertices(
                *position,
                *rotation,
                *scale,
                &self.shape_storage.vertices[i],
            );

            self.shape_storage.calculated_vertices[i] = calculated_vertices;
        }
    }

    // Calculate Segments Normals
    fn normal_components_logic(&mut self) {
        for i in 0..self.normals_renderer_storage.entities.len() {
            // Get Calculated Vertices
            let entity_id = self.normals_renderer_storage.entities[i];
            let (_vertices, calculated_vertices) = self.shape_storage.get_component(entity_id);

            let (normal_pos, segments_normals) = Self::edges_normal(calculated_vertices);

            self.normals_renderer_storage.normal_pos[i] = normal_pos;
            self.normals_renderer_storage.edge_normals[i] = segments_normals;
        }
    }

    /* #endregion */

    /* #region RENDER DATA */

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
    fn edges_normal(vertices: &Vec<Vec2>) -> (Vec<Vec2>, Vec<Vec2>) {
        let mut normal_positions = vec![];
        let mut segments_normals = vec![];

        for j in 0..vertices.len() {
            let vertex = vertices[j];
            let next_index = (j + 1) % vertices.len();
            let next_vertex = vertices[next_index];

            let normal_pos = gears::get_average_point(vertex, next_vertex);

            let edge_vector = next_vertex - vertex;
            let edge_normal = gears::vector_normal(edge_vector);

            normal_positions.push(normal_pos);
            segments_normals.push(edge_normal);
        }

        return (normal_positions, segments_normals);
    }

    /* #endregion */

    /* #region COLLISIONS DETECTION */

    // Each collider collides with itself and every collider after
    // eg Collider1, Collider2, Collider3
    // Collider1 = Collider1, Collider2, Collider3
    // Collider2 = Collider2, Collider3
    // Collider3 = Collider3

    // Collisions Functions = Check Collisions between 2 Colliders
    // collider1_collider2_collision_detection(Collider1, Collider1) -> (Collision)
    // TODO : Collision struct

    // Check if 2 Circles are Colliding
    // return collision_normal + penetration_depth
    fn circle_circle_collision_detection(
        position1: &Vec2,
        radius1: &f32,
        position2: &Vec2,
        radius2: &f32,
    ) -> (Option<Vec2>, Option<f32>) {
        let direction = position2 - position1; // direction circle1 -> circle2
        let distance = direction.length();
        let distance_threshold = radius1 + radius2;

        let is_colliding = distance <= distance_threshold;

        if is_colliding {
            let penetration_depth = distance - radius1;

            return (Some(direction.normalize()), Some(penetration_depth));
        }

        return (None, None);
    }

    fn segment_segment_collision_detection() -> (Option<Vec2>, Option<f32>) {
        todo!()
    }

    fn circle_segment_collision_detection(
        circle_pos: &Vec2,
        circle_radius: &f32,
        point1: &Vec2,
        point2: &Vec2,
        thickness: &f32,
    ) -> (Option<Vec2>, Option<f32>) {
        let distance_threshold = thickness + circle_radius;

        let edge_to_point_vec = Self::edge_to_point(point1, point2, circle_pos);
        let distance_edge = edge_to_point_vec.length();

        let is_colliding = distance_edge <= distance_threshold;

        if is_colliding {
            let collision_normal = edge_to_point_vec / distance_edge;
            let penetration_depth = circle_radius - distance_edge;

            return (Some(collision_normal), Some(penetration_depth));
        }

        return (None, None);
    }

    fn radius_collider_components_logic(&self, collisions_data: &mut CollisionsData) {
        // foreach entities with Radius Collider Component
        // TODO : use iterate_over_component

        for i in 0..self.radius_collider_storage.entities.len() {
            let entity_radius = self.radius_collider_storage.radiuses[i];

            if entity_radius == 0.0 {
                continue;
            }

            // Current Entity
            let entity_id = self.radius_collider_storage.entities[i];
            let (position, _rotation, _scale) = self.transform_storage.get_component(entity_id);
            // TODO : use has_component
            let is_rigidbody = self.rigidbody_storage.entities.contains(&entity_id);

            // Ignore List
            // If this entity collided with another -> Ignore this other Entity
            // TODO : Optimize
            let mut ignore_list: Vec<usize> = vec![]; // Indexes already captured

            if is_rigidbody {
                for j in 0..collisions_data.rr_collisions2.len() {
                    if collisions_data.rr_collisions2[j] == entity_id {
                        ignore_list.push(collisions_data.rr_collisions1[j]);
                    }
                }
            } else {
                for j in 0..collisions_data.rs_collisions1.len() {
                    if collisions_data.rs_collisions2[j] == entity_id {
                        ignore_list.push(collisions_data.rs_collisions1[j]);
                    }
                }
            }

            let entity_collision_data = EntityCollisionData {
                entity_id: &entity_id,
                position,
                ignore_list: &ignore_list,
                is_rigidbody: &is_rigidbody,
            };

            self.detect_other_radius_components_collision(
                collisions_data,
                &entity_collision_data,
                &entity_radius,
            );

            self.detect_other_segment_components_collision(
                collisions_data,
                &entity_collision_data,
                &entity_radius,
            )
        }
    }

    // Take a Single Radius Component and Detect its collisions with others
    fn detect_other_radius_components_collision(
        &self,
        collisions_data: &mut CollisionsData,
        entity_collision_data: &EntityCollisionData,
        entity_radius: &f32,
    ) {
        let entity_id = entity_collision_data.entity_id;
        let is_rigidbody = entity_collision_data.is_rigidbody;

        for j in 0..self.radius_collider_storage.entities.len() {
            let other_radius = self.radius_collider_storage.radiuses[j];

            if j == *entity_id
                || other_radius == 0.0
                || entity_collision_data.ignore_list.contains(&j)
            {
                continue;
            }

            let other_entity_id = self.transform_storage.entities[j];

            // Other Entity Circle
            let (other_position, _other_rotation, _other_scale) =
                self.transform_storage.get_component(other_entity_id);

            let (collision_normal, penetration_depth) = Self::circle_circle_collision_detection(
                &entity_collision_data.position,
                &entity_radius,
                &other_position,
                &other_radius,
            );

            // Collision Detected -> Register
            if let Some(collision_normal) = collision_normal
                && let Some(penetration_depth) = penetration_depth
            {
                let is_other_rigidbody =
                    self.rigidbody_storage.entities.contains(&(other_entity_id));

                // Static / Static Collision -> Do nothing
                if !is_rigidbody && !is_other_rigidbody {
                    continue;
                }

                // Register Rigidbody / Rigidbody Collision
                if *is_rigidbody && is_other_rigidbody {
                    collisions_data.rr_collisions1.push(*entity_id);
                    collisions_data.rr_collisions2.push(other_entity_id);
                    collisions_data.rr_collisions_normals.push(collision_normal);
                    collisions_data
                        .rr_collisions_penetration_depth
                        .push(penetration_depth);
                    continue;
                }

                // Register Rigidbody / Static Collision
                collisions_data.rs_collisions1.push(if *is_rigidbody {
                    *entity_id
                } else {
                    other_entity_id
                });
                collisions_data.rs_collisions2.push(if *is_rigidbody {
                    other_entity_id
                } else {
                    *entity_id
                });
                collisions_data.rs_collisions_normals.push(collision_normal);
                collisions_data
                    .rs_collisions_penetration_depth
                    .push(penetration_depth);
            }
        }
    }

    fn detect_other_segment_components_collision(
        &self,
        collisions_data: &mut CollisionsData,
        entity_collision_data: &EntityCollisionData,
        entity_radius: &f32,
    ) {
        for j in 0..self.segments_collider_storage.entities.len() {
            let thickness = self.segments_collider_storage.edge_thicknesses[j];

            if self.radius_collider_storage.radiuses[j] == 0.0
                || entity_collision_data.ignore_list.contains(&j)
            {
                continue;
            }

            if thickness == 0.0 {
                continue;
            }

            let other_entity_id = self.segments_collider_storage.entities[j];
            // get vertices
            let (vertices, calculated_vertices) = self.shape_storage.get_component(other_entity_id);

            // foreach segment
            for k in 0..calculated_vertices.len() {
                let point1 = vertices[k];
                let next_index = (k + 1) % vertices.len();
                let point2 = vertices[next_index];

                let (collision_normal, penetration_depth) =
                    Self::circle_segment_collision_detection(
                        &entity_collision_data.position,
                        &entity_radius,
                        &point1,
                        &point2,
                        &thickness,
                    );

                // Collision Detected -> Register
                if let Some(collision_normal) = collision_normal
                    && let Some(penetration_depth) = penetration_depth
                {
                    let is_other_rigidbody =
                        self.rigidbody_storage.entities.contains(&(other_entity_id));

                    // Register Rigidbody / Rigidbody Collision
                    if *entity_collision_data.is_rigidbody && is_other_rigidbody {
                        collisions_data
                            .rr_collisions1
                            .push(*entity_collision_data.entity_id);
                        collisions_data.rr_collisions2.push(other_entity_id);
                        collisions_data.rr_collisions_normals.push(collision_normal);
                        collisions_data
                            .rr_collisions_penetration_depth
                            .push(penetration_depth);
                        break;
                    }

                    // Register Rigidbody / Static Collision
                    collisions_data
                        .rs_collisions1
                        .push(if *entity_collision_data.is_rigidbody {
                            *entity_collision_data.entity_id
                        } else {
                            other_entity_id
                        });
                    collisions_data
                        .rs_collisions2
                        .push(if *entity_collision_data.is_rigidbody {
                            other_entity_id
                        } else {
                            *entity_collision_data.entity_id
                        });
                    collisions_data.rs_collisions_normals.push(collision_normal);
                    collisions_data
                        .rs_collisions_penetration_depth
                        .push(penetration_depth);

                    break; // Collide with only 1 Segment per Entity
                }
            }
        }
    }

    // TODO : segment_collider_components_logic

    // Detect Entity Collisions with Walls
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

            let edge_to_point = Self::edge_to_point(&square_vertex1, &square_vertex2, &position);
            let distance_edge = edge_to_point.length();

            // Collision
            if distance_edge <= distance_threshold {
                // println!("Collision with Edge : {j}");

                let collision_normal = edge_to_point / distance_edge;
                let penetration_depth = radius - distance_edge;

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

    /* #endregion */

    /* #region RIGIDBODY / SOLVER */

    fn rigidbody_component_logic(&mut self, dt: f32) {
        for i in 0..self.rigidbody_storage.entities.len() {
            let entity_id = self.rigidbody_storage.entities[i];

            // 1] Update Transform
            // Position
            // Apply Velocity
            let velocity_frame = self.rigidbody_storage.velocities[i] * dt;
            let rotation_speed_frame = self.rigidbody_storage.rotation_speeds[i] * dt;

            let position = &mut self.transform_storage.positions[entity_id];
            let rotation = &mut self.transform_storage.rotations[entity_id];

            position.x += velocity_frame.x;
            position.y += velocity_frame.y;

            // Gravity
            // position.y -= 9.81 * dt;

            // Rotation
            *rotation += rotation_speed_frame;
        }
    }

    // make a Rigidbody bounce from a normal
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

    // Solve Rigidbody / Static Collisions (Bounce)
    fn rs_collisions_solver(&mut self, collisions_data: &mut CollisionsData) {
        for i in 0..collisions_data.rs_collisions1.len() {
            let entity_id = collisions_data.rs_collisions1[i];

            let (velocity, _rotation_speed, bounciness) =
                self.rigidbody_storage.get_mut_component(entity_id);

            let (position, _rotation, _scale) = self.transform_storage.get_mut_component(entity_id);

            Self::bounce(
                collisions_data.rs_collisions_normals[i],
                collisions_data.rs_collisions_penetration_depth[i],
                velocity,
                bounciness,
                position,
            );
        }
    }

    // TODO : Collisions Context
    // Solve Rigidbody / Rigidbody Collisions
    fn collision_solver(&mut self, collisions_data: &mut CollisionsData) {
        for _iteration in 0..SOLVER_ITERATIONS {
            for i in 0..collisions_data.rr_collisions1.len() {
                // Get properties

                // inv_mass = invariant mass
                let a_inv_mass = 1.0; // TODO : Rigidbody property
                let b_inv_mass = 1.0;
                let total_inv_mass = a_inv_mass + b_inv_mass;

                // If both objects are unmovable
                if total_inv_mass == 0.0 {
                    continue;
                }

                // Entity1 Properties
                let entity_id1 = collisions_data.rr_collisions1[i];
                let transform_id1 = self.transform_storage.get_component_id(entity_id1);
                let rigidbody_id1 = self.rigidbody_storage.get_component_id(entity_id1);
                let velocity1 = self.rigidbody_storage.velocities[rigidbody_id1];
                let bounciness1 = self.rigidbody_storage.bounciness[rigidbody_id1];

                // Entity2 Properties
                let entity_id2 = collisions_data.rr_collisions2[i];
                let transform_id2 = self.transform_storage.get_component_id(entity_id2);
                let rigidbody_id2 = self.rigidbody_storage.get_component_id(entity_id2);
                let velocity2 = self.rigidbody_storage.velocities[rigidbody_id2];
                let bounciness2 = self.rigidbody_storage.bounciness[rigidbody_id2];

                // Collision Properties
                let normal = collisions_data.rr_collisions_normals[i];
                let penetration_depth = collisions_data.rr_collisions_penetration_depth[i];

                // 1] Positional Correction
                let percent = 0.05; // Resolve X% of the penetration per iteration
                let slop = 0.01; // Allow 1 centimeter of penetration before fixing

                let correction_magnitude =
                    (penetration_depth - slop).max(0.0) / total_inv_mass * percent;
                let correction_vector = normal * correction_magnitude;

                // Separation
                self.transform_storage.positions[transform_id1] -= correction_vector * a_inv_mass;
                self.transform_storage.positions[transform_id2] += correction_vector * b_inv_mass;

                // 2] Impulse Resolution
                // Relative velocity
                let rel_velocity = velocity2 - velocity1;

                let vel_along_normal = rel_velocity.dot(normal);

                // Do not resolve if velocities are already moving apart
                if vel_along_normal < 0.0 {
                    // Choose lower bounciness between Entities
                    let restitution = bounciness1.min(bounciness2);

                    // Calculate impulse scalar
                    let mut impulse_scalar = -(1.0 + restitution) * vel_along_normal;
                    impulse_scalar /= total_inv_mass;

                    // Apply impulse to each circle
                    let impulse = normal * impulse_scalar;
                    self.rigidbody_storage.velocities[rigidbody_id1] -= impulse * a_inv_mass;
                    self.rigidbody_storage.velocities[rigidbody_id2] += impulse * b_inv_mass;
                }
            }
        }
    }

    /* #endregion */

    /* #region UTILS  */

    // Detect if a point collide with a line (infinite)
    // use dot product between line_normal & point
    pub fn line_collision(
        line_start: &Vec2,
        line_end: &Vec2,
        thickness: &f32,
        point: &Vec2,
    ) -> bool {
        let line_direction = line_end - line_start;
        let line_normal = gears::vector_normal(line_direction);
        let ap = point - line_start;

        let distance = line_normal.dot(ap);

        return distance <= *thickness;
    }

    // Get the vector between a point and its projection on an edge (finite)
    pub fn edge_to_point(line_start: &Vec2, line_end: &Vec2, point: &Vec2) -> Vec2 {
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

    /* #endregion */
}

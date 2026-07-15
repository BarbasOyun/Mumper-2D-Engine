// MumperECS :
// Entity = ID + Version
// Components
// Systems = Components Logic

// Custom App (Implementing Mumper)
// define CustomComponents

// TODO :
// Impl Component Trait in macro
// define_components! macro
// remove_entity! macro -> make a function that remove entity's CustomComponents

use glam::Vec2;

use crate::Mumper;
use crate::MumperPhysics;

pub struct MumperECS {
    pub entity_ids: Vec<u32>,
    // versions: Vec<u32>,
    entities_bitmask: Vec<u64>, // Avoid checking if every ComponentStorage have an entity on Removing it

    // Components
    // renderer_storage
    // Physics
    pub transform_storage: TransformStorage,
    pub radius_collider_storage: RadiusColliderStorage,
    // pub segments_collider_storage: RadiusColliderStorage,
    // Rigid bodies,
}

impl MumperECS {
    pub fn new() -> Self {
        // Components
        let transform_storage = TransformStorage::new();
        let radius_collider_storage = RadiusColliderStorage::new();

        Self {
            entity_ids: vec![],
            entities_bitmask: vec![],
            transform_storage,
            radius_collider_storage,
        }
    }

    // ENTITIES

    pub fn create_entity(entity_ids: &mut Vec<u32>) -> u32 {
        let entity_id = entity_ids.len() as u32;
        entity_ids.push(entity_id);

        return entity_id;
    }

    pub fn create_entity_comp<T: Component>(
        entity_ids: &mut Vec<u32>,
        components: &mut Vec<T>,
    ) -> u32 {
        let entity_id = Self::create_entity(entity_ids);

        for i in 0..components.len() {
            components[i].add_default(entity_id);
        }

        return entity_id;
    }

    // TODO : use create_entity_comp
    // create an entity with transform
    pub fn create_physics_entity(
        state: &mut Mumper,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        vertices: Vec<Vec2>,
        // Collider
        radius: f32,
        // Rigidbody
        velocity: Vec2,
        rotation_speed: f32,
        bounciness: f32,
    ) -> u32 {
        // Create Entity
        let entity_id = Self::create_entity(&mut state.ecs.entity_ids);

        let ecs = &mut state.ecs;

        // ECS Components
        ecs.transform_storage
            .add(entity_id, position, rotation, scale);

        ecs.radius_collider_storage.add(entity_id, radius);

        let default_image = MumperPhysics::image_vertices(
            position.clone(),
            rotation.clone(),
            scale.clone(),
            &vertices,
        );

        // Add Shape to Physic engine
        {
            let mut physics = state.physics.lock().unwrap();

            // Add Physics Components
            physics
                .transform_storage
                .add(entity_id, position, rotation, scale);

            physics.radius_collider_storage.add(entity_id, radius);

            // Entity
            physics.vertices.push(vertices);
            physics.edge_normals.push(vec![]);
            physics.calculated_vertices.push(default_image);

            // Rigidbody
            physics.velocities.push(velocity);
            physics.rotation_speeds.push(rotation_speed);
            physics.bounciness.push(bounciness);
        };

        return entity_id;
    }

    // Cascade Deletion : Entity -> Components
    pub fn remove_entity(ecs: &mut MumperECS, entity_id: u32) {
        // Remove all Components
        let ent_id = entity_id as usize;
        if ent_id >= ecs.entities_bitmask.len() {
            return;
        }

        let mask = ecs.entities_bitmask[ent_id];

        // Bitwise operation
        // if (mask & TransformStorage::TYPE.mask()) != 0 {
        //     ecs.transform_storage.remove(entity_id);
        // }

        // Remove all physics components
        // {
        //     let mut physics = state.physics.lock().unwrap();
        //     // remove physic components
        // };

        // Reset mask
        ecs.entities_bitmask[ent_id] = 0;

        ecs.entity_ids.remove(entity_id as usize);
    }

    pub fn clear_entities(ecs: &mut MumperECS) {
        for i in 0..ecs.entity_ids.len() {
            Self::remove_entity(ecs, i as u32);
        }
    }

    // COMPONENTS

    pub fn add_comp(ecs: &mut MumperECS, entity_id: u32, component_type: ComponentType) {
        let ent_id = entity_id as usize;
        if ent_id >= ecs.entities_bitmask.len() {
            return;
        }

        let mask = ecs.entities_bitmask[ent_id];

        // Bitwise operation
    }

    pub fn add_component<T: Component>(entity_id: u32, component: &mut T) {
        component.add_default(entity_id);
        // Update entities_bitmask
    }

    pub fn get_component_id<T: Component>(entity_id: u32, component: T) -> u32 {
        return component.get_component_id(entity_id);
    }

    pub fn remove_component<T: Component>(entity_id: u32, component: &mut T) {
        component.remove(entity_id);
        // Update entities_bitmask
    }

    // Create a pool of Entities -> Object Pooling
    pub fn create_pool() {
        // TODO
    }
}

#[repr(u8)]
pub enum ComponentType {
    Transform = 0,
    RadiusCollider = 1,
    SegmentsCollider = 2,
    Rigidbody = 3,
    Renderer = 4,
}

impl ComponentType {
    #[inline(always)]
    pub fn mask(self) -> u64 {
        1u64 << (self as u8)
    }
}

pub trait Component {
    const TYPE: ComponentType;

    // Add component with default values
    fn add_default(&mut self, entity_id: u32);

    fn get_component_id(&self, entity_id: u32) -> u32;

    fn remove(&mut self, entity_id: u32);
}

// COMPONENTS

// Main Components

component_storage!(
    struct ShapeRendererStorage {
        // TODO : Flatten
        vertices: Vec<Vec2>,
        calculated_vertices: Vec<Vec2>,
        edge_normals: Vec<Vec2>,
    }
);

// CameraStorage

// Physics Components

// Every Physics Component depend on Transform
component_storage!(
    struct TransformStorage {
        positions: Vec2,
        rotations: f32,
        scales: Vec2,
    }
);

// PhysicsTransform
// No default transform

component_storage!(
    struct RadiusColliderStorage {
        radiuses: f32,
    }
);

component_storage!(
    struct SegmentColliderStorage {
        edge_thicknesses: f32,
    }
);

component_storage!(
    struct RigidbodyStorage {
        velocities: Vec2, // meters / sec
        rotation_speeds: f32,
        bounciness: f32,
    }
);

// Each system hold one ComponentStorage / Component
#[macro_export]
macro_rules! component_storage {
    (
        struct $storage_name:ident {
            $($field_name:ident : $field_type:ty),+ $(,)? // TODO : Default Values
        }
    ) => {
        #[derive(Clone)]
        pub struct $storage_name {
            // Default vectors
            pub sparse: Vec<usize>,
            pub entities: Vec<u32>,

            // Custom vectors
            $( pub $field_name: Vec<$field_type>, )+
        }

        // TODO impl Component for $storage_name
        impl $storage_name {
            pub fn new() -> Self {
                Self {
                    sparse: Vec::new(),
                    entities: Vec::new(),
                    $( $field_name: Vec::new(), )+
                }
            }

            pub fn add(&mut self, entity_id: u32, $($field_name: $field_type),+) {
                let dense_idx = self.entities.len();

                if entity_id as usize >= self.sparse.len() {
                    self.sparse.resize(entity_id as usize + 1, usize::MAX);
                }

                self.sparse[entity_id as usize] = dense_idx;
                self.entities.push(entity_id);

                // Push custom vectors
                $( self.$field_name.push($field_name); )+
            }

            pub fn get_component(&self, entity_id: u32) -> ($( &$field_type ),+) {
                let component_id = self.sparse[entity_id as usize];

                return ($( &self.$field_name[component_id] ),+);
            }

            pub fn iterate_over_components<F: FnMut(u32, $( &mut $field_type ),+)>(&mut self, mut action: F) {
                for i in 0..self.entities.len() {
                    let entity_id = self.entities[i];

                    // Mutate Component properties
                    action(entity_id, $( &mut self.$field_name[i] ),+)
                }
            }

            pub fn remove(&mut self, entity_id: u32) {
                let ent_id = entity_id as usize;

                if ent_id >= self.sparse.len() || self.sparse[ent_id] == usize::MAX {
                    return;
                }

                let index_to_remove = self.sparse[ent_id];
                let last_idx = self.entities.len() - 1;

                // if index_to_remove != last_index { // worth it?
                // }

                // Swap Last Entity -> Remove Index
                // index_to_remove -> moved entity
                self.entities.swap(index_to_remove, last_idx);
                $( self.$field_name.swap(index_to_remove, last_idx); )+

                // Update sparse array
                let moved_entity_id = self.entities[index_to_remove];
                self.sparse[moved_entity_id as usize] = index_to_remove;

                // Remove (swapped) Last Entity
                self.sparse[ent_id] = usize::MAX;
                self.entities.pop();
                $( self.$field_name.pop(); )+
            }
        }
    };
}

pub(crate) use component_storage;

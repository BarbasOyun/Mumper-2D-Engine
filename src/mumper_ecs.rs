// MumperECS :
// Entity = ID + Version
// Components
// Systems = Components Logic

// Custom App (Implementing Mumper)
// define CustomComponents

// TODO :
// Archetype ECS -> Group Entities by Archetype, Archetype = Entities with the same Components
// Impl Component Trait in macro
// define_components! macro
// remove_entity! macro -> make a function that remove entity's CustomComponents

use eframe::egui::*;
use glam::Vec2;

use crate::Mumper;
use crate::MumperPhysics;

pub struct MumperECS<A: Component> {
    component_count: u32,
    custom_components: Vec<A>,
    pub entity_ids: Vec<u32>, // TODO : usize
    // versions: Vec<u32>,
    entities_bitmask: Vec<u64>, // Avoid checking every ComponentStorage for an entity on Removing it
}

impl<A: Component> MumperECS<A> {
    pub fn new() -> Self {
        Self {
            component_count: 6,
            custom_components: vec![],
            entity_ids: vec![],
            entities_bitmask: vec![],
        }
    }

    // MASK

    pub fn get_mask(id: usize) -> u64 {
        return 1u64 << (id as u8);
    }

    pub fn add_mask(ecs: &mut MumperECS, entity_id: usize, component_mask: u64) {
        ecs.entities_bitmask[entity_id] |= component_mask;
    }

    pub fn has_component(ecs: &mut MumperECS, entity_id: usize, component_mask: u64) -> bool {
        let mask = ecs.entities_bitmask[entity_id];

        return (mask & component_mask) != 0;
    }

    pub fn remove_mask(ecs: &mut MumperECS, entity_id: usize, component_mask: u64) {
        ecs.entities_bitmask[entity_id] &= !component_mask;
    }

    // ENTITIES

    pub fn create_entity(ecs: &mut MumperECS) -> u32 {
        let entity_id = ecs.entity_ids.len() as u32;
        ecs.entity_ids.push(entity_id);

        return entity_id;
    }

    pub fn create_entity_comp<T: Component>(ecs: &mut MumperECS, components: &mut Vec<T>) -> u32 {
        let entity_id = Self::create_entity(ecs);

        for i in 0..components.len() {
            components[i].add_default(ecs, entity_id);
        }

        return entity_id;
    }

    // Cascade Deletion : Entity -> Components
    pub fn remove_entity(ecs: &mut MumperECS<A>, entity_id: u32) {
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

        // Remove custom components
        for i in 0..ecs.custom_components.len() {
            let mut custom_component = ecs.custom_components[i];

            let component_mask = Self::get_mask(custom_component.get_id());

            if Self::has_component(ecs, entity_id as usize, component_mask) {
                custom_component.remove(ecs, entity_id);
            }
        }

        // Remove
        ecs.entities_bitmask.remove(ent_id);
        ecs.entity_ids.remove(entity_id as usize);
    }

    pub fn clear_entities(state: &mut Mumper) {
        let ecs = &mut state.ecs;

        for i in 0..ecs.entity_ids.len() {
            Self::remove_entity(ecs, i as u32);
        }
    }

    // COMPONENTS

    // add component of a type to an entity with default values
    pub fn add_component(state: &mut Mumper, entity_id: u32, component_id: usize) {
        let ecs = &mut state.ecs;

        // Default Transform
        if component_id == 0 {
            state.default_transforms.add_default(ecs, entity_id);
        }

        // Transform
        if component_id == 1 {
            Self::add_default_transform(state, entity_id);
        }

        // match

        // Get Custom storage = at component_type_id -> add_default
    }

    // TODO : add_components

    pub fn remove_component(state: &mut Mumper, entity_id: u32, component_type_id: usize) {
        let ecs = &mut state.ecs;

        // Default Transform
        if component_type_id == 0 {
            state.default_transforms.remove(ecs, entity_id);
        }

        // Transform
        if component_type_id == 1 {
            state.transform_storage.remove(ecs, entity_id);
        }

        // match
    }

    // Add Component + sync physics

    pub fn add_default_transform(state: &mut Mumper, entity_id: u32) {
        let ecs = &mut state.ecs;

        state.transform_storage.add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.transform_storage.add_default(ecs, entity_id);
        }
    }

    pub fn add_transform(
        state: &mut Mumper,
        entity_id: u32,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
    ) {
        let ecs = &mut state.ecs;

        state
            .transform_storage
            .add(ecs, entity_id, position, rotation, scale);

        {
            let mut physics = state.physics.lock().unwrap();

            physics
                .transform_storage
                .add(ecs, entity_id, position, rotation, scale);
        }
    }

    // TODO : remove_components

    // Create a pool of Entities -> Object Pooling
    pub fn create_pool() {
        todo!()
    }
}

pub enum EngineComponent<A: Component> {
    Transform(ShapeRendererStorage),
    Velocity(NormalsRendererStorage),
    Extension(A),
}

#[repr(u8)]
pub enum ComponentType {
    DefaultTransform = 0,
    Transform = 1,
    RadiusCollider = 2,
    SegmentsCollider = 3,
    Rigidbody = 4,
    Renderer = 5,
}

// COMPONENTS DEFINITIONS

// Main Components

component_storage!(
    struct ShapeRendererStorage {
        // TODO : Flatten
        calculated_vertices: Vec<Vec2>,
        strokes: Stroke,
    },
    add_default: |storage: &mut ShapeRendererStorage<A>, ecs: &mut MumperECS<A>, entity_id: u32| {
        storage.add(ecs, entity_id, vec![], Stroke::new(1.0, Color32::RED));
    }
);

component_storage!(
    struct NormalsRendererStorage {
        normal_pos: Vec<Vec2>,
        edge_normals: Vec<Vec2>,
    },
    add_default: |storage: &mut NormalsRendererStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, vec![], vec![]);
    }
);

// CameraStorage

// Physics Components

// Only on physics side
component_storage!(
    struct PhysicsShapeStorage {
        vertices: Vec<Vec2>,
        calculated_vertices: Vec<Vec2>,
    },
    add_default: |storage: &mut PhysicsShapeStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, vec![], vec![]);
    }
);

// Every Physics Component depend on Transform
component_storage!(
    struct TransformStorage {
        positions: Vec2,
        rotations: f32,
        scales: Vec2,
    },
    add_default: |storage: &mut TransformStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, Vec2::ZERO, 0.0, Vec2::ONE);
    }
);

component_storage!(
    struct RadiusColliderStorage {
        radiuses: f32,
        // is_trigger: bool,
    },
    add_default: |storage: &mut RadiusColliderStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, 1.0);
    }
);

component_storage!(
    struct SegmentColliderStorage {
        edge_thicknesses: f32,
        // is_trigger: bool,
    },
    add_default: |storage: &mut SegmentColliderStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, 1.0);
    }
);

component_storage!(
    struct RigidbodyStorage {
        // Mass,
        velocities: Vec2, // meters / sec
        rotation_speeds: f32,
        bounciness: f32,
    },
    add_default: |storage: &mut RigidbodyStorage, ecs: &mut MumperECS, entity_id: u32| {
        storage.add(ecs, entity_id, Vec2::ZERO, 0.0, 0.8);
    }
);

// Allow custom components
pub trait Component: 'static {
    // Add component with default values
    fn add_default(&mut self, ecs: &mut MumperECS, entity_id: u32);

    fn remove(&mut self, ecs: &mut MumperECS, entity_id: u32);

    fn get_id(&self) -> usize;
}

// Each system hold one ComponentStorage / Component
#[macro_export]
macro_rules! component_storage {
    (
        struct $storage_name:ident {
            $($field_name:ident : $field_type:ty),+ $(,)?
        },
        add_default: $add_default:expr
    ) => {
        #[derive(Clone)]
        pub struct $storage_name {
            // Default data
            pub id: usize, // id is defined per instance (storage) rather than type
            pub sparse: Vec<usize>,
            pub entities: Vec<u32>,

            // Custom data
            $( pub $field_name: Vec<$field_type>, )+
        }

        impl $storage_name {
            pub fn new(ecs: &mut MumperECS<$storage_name>) -> Self {
                let id = ecs.component_count.clone() as usize;
                ecs.component_count += 1;

                Self {
                    id,
                    sparse: Vec::new(),
                    entities: Vec::new(),
                    $( $field_name: Vec::new(), )+
                }
            }

            // Add Component

            pub fn add(&mut self, ecs: &mut MumperECS<$storage_name>, entity_id: u32, $($field_name: $field_type),+) {
                let dense_idx = self.entities.len();

                if entity_id as usize >= self.sparse.len() {
                    self.sparse.resize(entity_id as usize + 1, usize::MAX);
                }

                self.sparse[entity_id as usize] = dense_idx;
                self.entities.push(entity_id);

                // Push custom vectors
                $( self.$field_name.push($field_name); )+

                // Update bitmask
                let mask = MumperECS::get_mask(self.id);
                MumperECS::add_mask(ecs, entity_id as usize, mask);
            }

            // Use Component

            pub fn get_component(&self, entity_id: &u32) -> ($( &$field_type ),+) {
                let component_id = self.sparse[*entity_id as usize];

                return ($( &self.$field_name[component_id] ),+);
            }

            pub fn get_mut_component(&mut self, entity_id: &u32) -> ($( &mut $field_type ),+) {
                let component_id = self.sparse[*entity_id as usize];

                return ($( &mut self.$field_name[component_id] ),+);
            }

            pub fn iterate_over_components<F: FnMut(u32, $( &mut $field_type ),+)>(&mut self, mut action: F) {
                for i in 0..self.entities.len() {
                    let entity_id = self.entities[i];

                    // Mutate Component properties
                    action(entity_id, $( &mut self.$field_name[i] ),+)
                }
            }

            // Remove Component

            pub fn clear_components(&mut self) {
                self.sparse.clear();
                self.entities.clear();

                $( self.$field_name.clear(); )+
            }
        }

        impl Component for $storage_name {
            // TODO : add_default / storage instance
            fn add_default(&mut self, ecs: &mut MumperECS<$storage_name>, entity_id: u32) {
                ($add_default)(self, ecs, entity_id);
            }

            fn remove(&mut self, ecs: &mut MumperECS<$storage_name>, entity_id: u32) {
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

                // Update bitmask
                let mask = MumperECS::get_mask(self.id);
                MumperECS::remove_mask(ecs, entity_id as usize, mask);
            }

            fn get_id(&self) -> usize {
                return self.id;
            }
        }
    };
}

pub(crate) use component_storage;

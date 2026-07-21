// TODO :
// Storage enum -> Avoid Box
// Component Dependencies -> On Adding : Add components that the component Depend on -> Sync Physics Components
// Archetype ECS -> Group Entities by Archetype, Archetype = Entities with the same Components
// Object pooling = create pool of Archetype + Disable

use egui::Stroke;
use glam::Vec2;
use std::any::Any;

use crate::Mumper;

pub struct MumperECS {
    pub component_count: usize,
    pub entity_ids: Vec<usize>,
    // versions: Vec<u32>,
    pub entities_bitmask: Vec<u64>, // Avoid checking every ComponentStorage for an entity on Removing it
}

impl MumperECS {
    pub fn new() -> Self {
        return Self {
            component_count: 0,
            entity_ids: vec![],
            entities_bitmask: vec![],
        };
    }

    // STORAGE

    // get the next storage id
    // ensure every storage have a unique id
    pub fn register_storage_id(ecs: &mut MumperECS) -> usize {
        let id = ecs.component_count.clone();
        ecs.component_count += 1;

        return id;
    }

    pub fn register_storage<T: Component + 'static>(components: &mut Components, storage: T) {
        let boxed_storage: Box<dyn Component> = Box::new(storage);
        components.custom_components.push(boxed_storage);
    }

    // MASK

    pub fn get_component_mask(component_id: usize) -> u64 {
        return 1u64 << (component_id as u8);
    }

    pub fn add_mask(entity_mask: &mut u64, component_mask: u64) {
        *entity_mask |= component_mask;
    }

    pub fn remove_mask(entity_mask: &mut u64, component_mask: u64) {
        *entity_mask &= !component_mask;
    }

    // ENTITIES

    pub fn create_entity(ecs: &mut MumperECS) -> usize {
        let entity_id = ecs.entity_ids.len();
        ecs.entity_ids.push(entity_id);
        ecs.entities_bitmask.push(0);

        return entity_id;
    }

    pub fn create_entity_comp<T: Component>(ecs: &mut MumperECS, components: &mut Vec<T>) -> usize {
        let entity_id = Self::create_entity(ecs);

        for i in 0..components.len() {
            components[i].add_default(ecs, entity_id);
        }

        return entity_id;
    }

    // Cascade Deletion : Entity -> Components
    pub fn remove_entity(state: &mut Mumper, entity_id: usize) {
        if entity_id >= state.ecs.entities_bitmask.len() {
            return;
        }

        // Remove Entity
        state.ecs.entities_bitmask.remove(entity_id);
        state.ecs.entity_ids.remove(entity_id);

        let entity_mask = state.ecs.entities_bitmask[entity_id];

        // Remove all Engine Components
        if Self::has_component(&entity_mask, ComponentType::Transform as usize) {
            MumperECS::remove_transform(state, entity_id);
        }

        if Self::has_component(&entity_mask, ComponentType::RadiusCollider as usize) {
            MumperECS::remove_radius_collider(state, entity_id);
        }

        if Self::has_component(&entity_mask, ComponentType::SegmentsCollider as usize) {
            MumperECS::remove_segments_collider(state, entity_id);
        }

        if Self::has_component(&entity_mask, ComponentType::Rigidbody as usize) {
            MumperECS::remove_rigidbody(state, entity_id);
        }

        if Self::has_component(&entity_mask, ComponentType::Renderer as usize) {
            MumperECS::remove_shape_renderer(state, entity_id);
        }

        if Self::has_component(&entity_mask, ComponentType::NormalsRenderer as usize) {
            MumperECS::remove_shape_renderer(state, entity_id);
        }

        // Remove custom components
        for i in 0..state.components.custom_components.len() {
            // Limit interaction with the heap
            if Self::has_component(&entity_mask, i) {
                state.components.custom_components[i].remove(&mut state.ecs, entity_id);
            }
        }
    }

    pub fn clear_entities(state: &mut Mumper) {
        // Clear Engine Storages
        state.components.transform_storage.clear_components();
        state.components.radius_collider_storage.clear_components();
        state
            .components
            .segments_collider_storage
            .clear_components();
        state.components.rigidbody_storage.clear_components();
        state.renderer.shape_renderer_storage.clear_components();
        state.renderer.normals_renderer_storage.clear_components();

        {
            let mut physics = state.physics.lock().unwrap();

            physics.transform_storage.clear_components();
            physics.radius_collider_storage.clear_components();
            physics.segments_collider_storage.clear_components();
            physics.rigidbody_storage.clear_components();
            physics.shape_storage.clear_components();
            physics.normals_renderer_storage.clear_components();
        }

        // Clear Custom Storages
        for i in 0..state.components.custom_components.len() {
            state.components.custom_components[i].clear_components();
        }

        let ecs = &mut state.ecs;

        ecs.entities_bitmask.clear();
        ecs.entity_ids.clear();
    }

    // ANY COMPONENTS

    pub fn has_component(entity_mask: &u64, component_id: usize) -> bool {
        let component_mask = Self::get_component_mask(component_id);

        return (entity_mask & component_mask) != 0;
    }

    // add component of a type to an entity with default values
    pub fn add_component(state: &mut Mumper, entity_id: usize, component_id: usize) {
        match component_id {
            0 => Self::add_default_transform(state, entity_id),
            1 => Self::add_default_radius_collider(state, entity_id),
            2 => Self::add_default_segments_collider(state, entity_id),
            3 => Self::add_default_rigidbody(state, entity_id),
            4 => Self::add_default_shape_renderer(state, entity_id),
            5 => Self::add_normals_renderer(state, entity_id),
            _ => todo!(), // println!("Add Component : No component with ID = {component_id}"),
        }

        // Add Custom Component
        let ecs = &mut state.ecs;
        state.components.custom_components[component_id].add_default(ecs, entity_id);
    }

    // TODO : add_components -> Lock physics once

    pub fn remove_component(state: &mut Mumper, entity_id: usize, component_id: usize) {
        match component_id {
            0 => Self::remove_transform(state, entity_id),
            1 => Self::remove_radius_collider(state, entity_id),
            2 => Self::remove_segments_collider(state, entity_id),
            3 => Self::remove_rigidbody(state, entity_id),
            4 => Self::remove_shape_renderer(state, entity_id),
            5 => Self::remove_normals_renderer(state, entity_id),
            _ => todo!(), // println!("Add Component : No Engine component with ID = {component_id}"),
        }

        // Remove Custom Component
        let ecs = &mut state.ecs;
        state.components.custom_components[component_id].remove(ecs, entity_id);
    }

    // TODO : remove_components

    // ENGINE COMPONENTS
    // sync physics

    // TODO : DRY

    // Transform
    pub fn add_default_transform(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .transform_storage
            .add_default(ecs, entity_id);
        state
            .components
            .default_transforms
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.transform_storage.add_default(ecs, entity_id);
        }
    }

    pub fn add_transform(
        state: &mut Mumper,
        entity_id: usize,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
    ) {
        let ecs = &mut state.ecs;

        state
            .components
            .transform_storage
            .add(ecs, entity_id, position, rotation, scale);

        // Also add Default transform
        state
            .components
            .default_transforms
            .add(ecs, entity_id, position, rotation, scale);

        // Also add Physics transform
        {
            let mut physics = state.physics.lock().unwrap();

            physics
                .transform_storage
                .add(ecs, entity_id, position, rotation, scale);
        }
    }

    pub fn remove_transform(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state.components.transform_storage.remove(ecs, entity_id);
        state.components.default_transforms.remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.transform_storage.remove(ecs, entity_id);
        }
    }

    // Radius Collider
    pub fn add_default_radius_collider(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .radius_collider_storage
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.radius_collider_storage.add_default(ecs, entity_id);
        }
    }

    pub fn add_radius_collider(state: &mut Mumper, entity_id: usize, radius: f32) {
        let ecs = &mut state.ecs;

        state
            .components
            .radius_collider_storage
            .add(ecs, entity_id, radius);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.radius_collider_storage.add(ecs, entity_id, radius);
        }
    }

    pub fn remove_radius_collider(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .radius_collider_storage
            .remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.radius_collider_storage.remove(ecs, entity_id);
        }
    }

    // Segment Collider
    pub fn add_default_segments_collider(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .segments_collider_storage
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();
            physics
                .segments_collider_storage
                .add_default(ecs, entity_id);
        }
    }

    pub fn add_segments_collider(state: &mut Mumper, entity_id: usize, thickness: f32) {
        let ecs = &mut state.ecs;

        state
            .components
            .segments_collider_storage
            .add(ecs, entity_id, thickness);

        {
            let mut physics = state.physics.lock().unwrap();
            physics
                .segments_collider_storage
                .add(ecs, entity_id, thickness);
        }
    }

    pub fn remove_segments_collider(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .segments_collider_storage
            .remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();
            physics.segments_collider_storage.remove(ecs, entity_id);
        }
    }

    // Rigidbody
    pub fn add_default_rigidbody(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .components
            .rigidbody_storage
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();
            physics.rigidbody_storage.add_default(ecs, entity_id);
        }
    }

    pub fn add_rigidbody(
        state: &mut Mumper,
        entity_id: usize,
        velocity: Vec2,
        rotation_speed: f32,
        bounciness: f32,
    ) {
        let ecs = &mut state.ecs;

        state.components.rigidbody_storage.add(
            ecs,
            entity_id,
            velocity,
            rotation_speed,
            bounciness,
        );

        {
            let mut physics = state.physics.lock().unwrap();
            physics
                .rigidbody_storage
                .add(ecs, entity_id, velocity, rotation_speed, bounciness);
        }
    }

    pub fn remove_rigidbody(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state.components.rigidbody_storage.remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();
            physics.rigidbody_storage.remove(ecs, entity_id);
        }
    }

    // Renderer
    pub fn add_default_shape_renderer(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .renderer
            .shape_renderer_storage
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.shape_storage.add_default(ecs, entity_id);
        }
    }

    pub fn add_shape_renderer(
        state: &mut Mumper,
        entity_id: usize,
        vertices: Vec<Vec2>,
        stroke: Stroke,
    ) {
        let ecs = &mut state.ecs;

        state
            .renderer
            .shape_renderer_storage
            .add(ecs, entity_id, vertices.clone(), stroke);

        {
            let mut physics = state.physics.lock().unwrap();

            physics
                .shape_storage
                .add(ecs, entity_id, vertices.clone(), vertices);
        }
    }

    pub fn remove_shape_renderer(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state.renderer.shape_renderer_storage.remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.shape_storage.remove(ecs, entity_id);
        }
    }

    // Normals Renderer
    pub fn add_normals_renderer(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .renderer
            .normals_renderer_storage
            .add_default(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.normals_renderer_storage.add_default(ecs, entity_id);
        }
    }

    pub fn remove_normals_renderer(state: &mut Mumper, entity_id: usize) {
        let ecs = &mut state.ecs;

        state
            .renderer
            .normals_renderer_storage
            .remove(ecs, entity_id);

        {
            let mut physics = state.physics.lock().unwrap();

            physics.normals_renderer_storage.remove(ecs, entity_id);
        }
    }

    // ARCHETYPES + POOLING
    // TODO

    // Create a pool of Entities
    pub fn create_pool() {
        todo!()
    }
}

// Allow custom components
pub trait Component: 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn get_id(&self) -> usize;

    // Add component with default values
    fn add_default(&mut self, ecs: &mut MumperECS, entity_id: usize);

    fn remove(&mut self, ecs: &mut MumperECS, entity_id: usize);

    fn clear_components(&mut self);
}

// Each system hold one ComponentStorage / Component
// TODO : Component Object
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
            pub entities: Vec<usize>,

            // Custom data
            $( pub $field_name: Vec<$field_type>, )+
        }

        impl $storage_name {
            pub fn new(id : usize) -> Self {
                Self {
                    id,
                    sparse: Vec::new(),
                    entities: Vec::new(),
                    $( $field_name: Vec::new(), )+
                }
            }

            // Add Component

            pub fn add(&mut self, ecs: &mut MumperECS, entity_id: usize, $($field_name: $field_type),+) {
                let dense_idx = self.entities.len();

                if entity_id >= self.sparse.len() {
                    self.sparse.resize(entity_id + 1, usize::MAX);
                }

                self.sparse[entity_id] = dense_idx;
                self.entities.push(entity_id);

                // Push custom vectors
                $( self.$field_name.push($field_name); )+

                // Update bitmask
                let mask = MumperECS::get_component_mask(self.id);
                MumperECS::add_mask(&mut ecs.entities_bitmask[entity_id], mask);
            }

            // Use Component

            pub fn get_component_id(&self, entity_id: usize) -> usize {
                return self.sparse[entity_id];
            }

            pub fn get_component(&self, entity_id: usize) -> ($( &$field_type ),+) {
                let component_id = self.sparse[entity_id];

                return ($( &self.$field_name[component_id] ),+);
            }

            pub fn get_mut_component(&mut self, entity_id: usize) -> ($( &mut $field_type ),+) {
                let component_id = self.sparse[entity_id];

                return ($( &mut self.$field_name[component_id] ),+);
            }

            pub fn iterate_over_components<F: FnMut(usize, $( &mut $field_type ),+)>(&mut self, mut action: F) {
                for i in 0..self.entities.len() {
                    let entity_id = self.entities[i];

                    // Mutate Component properties
                    action(entity_id, $( &mut self.$field_name[i] ),+)
                }
            }
        }

        impl Component for $storage_name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn get_id(&self) -> usize {
                return self.id;
            }

            // TODO : add_default per storage instance
            fn add_default(&mut self, ecs: &mut MumperECS, entity_id: usize) {
                ($add_default)(self, ecs, entity_id);
            }

            // Remove Component

            fn remove(&mut self, ecs: &mut MumperECS, entity_id: usize) {
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
                self.sparse[moved_entity_id] = index_to_remove;

                // Remove (swapped) Last Entity
                self.sparse[ent_id] = usize::MAX;
                self.entities.pop();
                $( self.$field_name.pop(); )+

                // Update bitmask
                let mask = MumperECS::get_component_mask(self.id);
                MumperECS::remove_mask(&mut ecs.entities_bitmask[entity_id], mask);
            }

            fn clear_components(&mut self) {
                self.sparse.clear();
                self.entities.clear();

                $( self.$field_name.clear(); )+
            }
        }
    };
}

pub(crate) use component_storage;

#[repr(u8)]
pub enum ComponentType {
    Transform = 0,
    RadiusCollider = 1,
    SegmentsCollider = 2,
    Rigidbody = 3,
    Renderer = 4,
    NormalsRenderer = 5,
}

// TODO : Centralize Components
pub struct Components {
    // Components
    pub default_transforms: TransformStorage,
    // Physics Components (Shared with physics)
    pub transform_storage: TransformStorage,
    pub radius_collider_storage: RadiusColliderStorage,
    pub segments_collider_storage: SegmentColliderStorage,
    pub rigidbody_storage: RigidbodyStorage,
    pub custom_components: Vec<Box<dyn Component>>,
}

impl Components {
    pub fn new(ecs: &mut MumperECS) -> Self {
        let default_transforms = TransformStorage::new(MumperECS::register_storage_id(ecs));
        let transform_storage = TransformStorage::new(MumperECS::register_storage_id(ecs));

        let radius_collider_storage =
            RadiusColliderStorage::new(MumperECS::register_storage_id(ecs));
        let segments_collider_storage =
            SegmentColliderStorage::new(MumperECS::register_storage_id(ecs));
        let rigidbody_storage = RigidbodyStorage::new(MumperECS::register_storage_id(ecs));

        return Self {
            default_transforms,
            transform_storage,
            radius_collider_storage,
            segments_collider_storage,
            rigidbody_storage,
            custom_components: vec![],
        };
    }
}

// PHYSICS COMPONENTS

// Every Physics Component depend on Transform
// Used for transform & default transform storage
component_storage!(
    struct TransformStorage {
        positions: Vec2,
        rotations: f32,
        scales: Vec2,
    },
    add_default: |storage: &mut TransformStorage, ecs: &mut MumperECS, entity_id: usize| {
        storage.add(ecs, entity_id, Vec2::ZERO, 0.0, Vec2::ONE);
    }
);

// Only on physics side
component_storage!(
    struct PhysicsShapeStorage {
        vertices: Vec<Vec2>,
        calculated_vertices: Vec<Vec2>,
    },
    add_default: |storage: &mut PhysicsShapeStorage, ecs: &mut MumperECS, entity_id: usize| {
        storage.add(ecs, entity_id, vec![], vec![]);
    }
);

component_storage!(
    struct RadiusColliderStorage {
        radiuses: f32,
        // is_trigger: bool,
    },
    add_default: |storage: &mut RadiusColliderStorage, ecs: &mut MumperECS, entity_id: usize| {
        storage.add(ecs, entity_id, 1.0);
    }
);

component_storage!(
    struct SegmentColliderStorage {
        edge_thicknesses: f32,
        // is_trigger: bool,
    },
    add_default: |storage: &mut SegmentColliderStorage, ecs: &mut MumperECS, entity_id: usize| {
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
    add_default: |storage: &mut RigidbodyStorage, ecs: &mut MumperECS, entity_id: usize| {
        storage.add(ecs, entity_id, Vec2::ZERO, 0.0, 0.8);
    }
);

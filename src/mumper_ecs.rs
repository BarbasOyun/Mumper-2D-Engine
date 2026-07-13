// MumperECS :
// ECS = create_entity, remove_entity, add_component, remove_component
// Entity = ID + Version
// Components
// Systems = Hold Components + their Data, Components Update Logic -> System Own Data = Add custom System
// Entity Pooling -> create_pool

// Mumper
// Systems : Mumper = Rendering, MumperPhysics = Physics

pub struct MumperECS {
    pub entity_ids: Vec<u32>,
    // versions: Vec<u32>
    on_remove_entity: Vec<fn(u32)>,
}

impl MumperECS {
    pub fn new() -> Self {
        return Self {
            entity_ids: vec![],
            on_remove_entity: vec![],
        };
    }

    pub fn create_entity<T: Component + Clone>(&mut self, components: Vec<T>) {
        let entity_id = self.entity_ids.len() as u32;
        self.entity_ids.push(entity_id);

        for i in 0..components.len() {
            self.add_component(entity_id, components[i].clone())
        }
    }

    pub fn remove_entity(&mut self, entity_id: u32) {
        // Remove Entity Event
        for listener in &self.on_remove_entity {
            listener(entity_id);
        }

        self.entity_ids.remove(entity_id as usize);
    }

    // where T = Component
    pub fn add_component<T: Component>(&mut self, entity_id: u32, component: T) {
        component.add(entity_id);
    }

    pub fn get_component<T: Component>(&mut self, entity_id: u32, component: T) {
        component.get(entity_id);
    }

    pub fn remove_component<T: Component>(&mut self, entity_id: u32, component: T) {
        component.remove(entity_id);
    }
}

pub trait Component {
    // Subscribe to on_remove_entity -> remove
    // Define which ComponentStorage to call insert on
    fn add(&self, entity_id: u32);

    fn get(&self, entity_id: u32);

    fn remove(&self, entity_id: u32);
}

// Each system hold one ComponentStorage / Component
#[macro_export]
macro_rules! define_component_storage {
    (
        struct $storage_name:ident {
            $($field_name:ident : $field_type:ty),+ $(,)?
        }
    ) => {
        pub struct $storage_name {
            // Default vectors
            pub sparse: Vec<usize>,
            pub entities: Vec<u32>,

            // Custom vectors
            $( pub $field_name: Vec<$field_type>, )+
        }

        impl $storage_name {
            pub fn new() -> Self {
                Self {
                    sparse: Vec::new(),
                    entities: Vec::new(),
                    $( $field_name: Vec::new(), )+
                }
            }

            pub fn insert(&mut self, entity_id: u32, $($field_name: $field_type),+) {
                let dense_idx = self.entities.len();

                if entity_id as usize >= self.sparse.len() {
                    self.sparse.resize(entity_id as usize + 1, usize::MAX);
                }

                self.sparse[entity_id as usize] = dense_idx;
                self.entities.push(entity_id);

                // Push custom vectors
                $( self.$field_name.push($field_name); )+
            }

            pub fn get_component(&self, entity_id: u32) -> ( $( &$field_type ),+ ) {
                let component_id = self.sparse[entity_id as usize];
                // let component = &self.components[component_id];

                // return component_id;
                return ($( &self.$field_name[component_id] ),+);
            }

            pub fn iterate_over_components<F: FnMut(u32, $($field_type),+)>(&mut self, mut action: F) {
                for i in 0..self.entities.len() {
                    let entity_id = self.entities[i];
                    
                    // Mutate Component properties
                    action(entity_id, $( self.$field_name[i] ),+)
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

pub(crate) use define_component_storage;

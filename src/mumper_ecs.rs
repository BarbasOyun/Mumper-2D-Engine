// MumperECS :
// ECS = create_entity, remove_entity, add_component, remove_component
// Entity = ID + Version
// Components
// Systems = Hold Components + their Data, Components Update Logic -> System Own Data = Add custom System
// Entity Pooling -> create_pool

// Mumper
// Systems : Mumper = Rendering, MumperPhysics = Physics

pub struct MumperECS {
    entity_ids: Vec<u32>,
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
pub struct ComponentStorage<T: Component> {
    pub components: Vec<T>,
    pub entities: Vec<u32>,

    pub sparse: Vec<usize>,
}

pub trait ECSSystem<T: Component> {
    pub fn insert(&mut self, entity_id: u32, component: T) {
        let dense_index = self.components.len();

        // Resize sparse
        if entity_id as usize >= self.sparse.len() {
            self.sparse.resize(entity_id as usize + 1, usize::MAX);
        }

        self.sparse[entity_id as usize] = dense_index;

        self.components.push(component);
        self.entities.push(entity_id);
    }

    pub fn get_component(&self, entity_id: u32) -> &T {
        let component_id = self.sparse[entity_id as usize];
        // let component = &self.components[component_id];

        return component
    }

    pub fn iterate_over_components(&mut self) {
        for i in 0..self.components.len() {
            let component = &mut self.components[i];
            let owner_entity = self.entities[i];

            // mutate_component_using_entity(component, owner_entity);
            // Event?
        }
    }

    pub fn remove(&mut self, entity_id: u32) {
        let entity_id = entity_id as usize;

        if entity_id >= self.sparse.len() || self.sparse[entity_id] == usize::MAX {
            return;
        }

        let index_to_remove = self.sparse[entity_id];
        let last_index = self.components.len() - 1;

        // if index_to_remove != last_index { // worth it?
        // }

        // Swap -> index_to_remove = moved entity
        self.components.swap(index_to_remove, last_index);
        self.entities.swap(index_to_remove, last_index);

        // Update sparse array
        let moved_entity_id = self.entities[index_to_remove];
        self.sparse[moved_entity_id as usize] = index_to_remove;

        // Remove Last Entity
        self.sparse[entity_id] = usize::MAX;
        self.components.pop();
        self.entities.pop();
    }
}

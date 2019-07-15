/// ! Miscellanious methods for modifying entities in the world.
use crate::*;

impl DiskManager {
    /// Sets the label of a partition.
    pub fn label<S: Into<Box<str>>>(&mut self, entity: Entity, label: S) {
        self.queued_changes.labels.insert(entity, label.into());
        self.flags |= ManagerFlags::LABEL;
    }

    /// Marks the entity for removal, along with all of its children, and their children.
    pub fn remove(&mut self, entity: Entity) {
        self.entities[entity] |= Flags::REMOVE;

        fn recurse(
            entities: &mut HopSlotMap<Entity, Flags>,
            storage: &SecondaryMap<Entity, Vec<Entity>>,
            child: Entity,
        ) {
            for &child in storage.get(child).into_iter().flatten() {
                entities[child] |= Flags::REMOVE;
                recurse(entities, storage, child);
            }
        }

        recurse(&mut self.entities, &self.components.children, entity);
        self.flags |= ManagerFlags::REMOVE;
    }
}

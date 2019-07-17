/// ! Miscellanious methods for modifying entities in the world.
use crate::*;

impl DiskManager {
    /// Sets the label of a partition.
    pub fn label<S: Into<Box<str>>>(&mut self, entity: DeviceEntity, label: S) {
        self.components.queued_changes.labels.insert(entity, label.into());
        self.flags |= ManagerFlags::LABEL;
    }

    /// Marks the entity for removal, along with all of its children, and their children.
    pub fn remove(&mut self, entity: DeviceEntity) {
        self.entities.devices[entity] |= EntityFlags::REMOVE;

        fn recurse(
            entities: &mut HopSlotMap<DeviceEntity, EntityFlags>,
            storage: &SecondaryMap<DeviceEntity, Vec<DeviceEntity>>,
            child: DeviceEntity,
        ) {
            for &child in storage.get(child).into_iter().flatten() {
                entities[child] |= EntityFlags::REMOVE;
                recurse(entities, storage, child);
            }
        }

        recurse(&mut self.entities.devices, &self.components.devices.children, entity);
        self.flags |= ManagerFlags::REMOVE;
    }
}

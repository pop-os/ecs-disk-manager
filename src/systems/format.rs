use crate::*;
use std::path::Path;

// TODO:
// - Format devices in parallel.

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to format {:?} with {}", _0, _1)]
    Mkfs(Box<Path>, FileSystem, #[error(cause)] io::Error),
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let entities = &mut world.entities;
    let queued_changes = &mut world.queued_changes;
    let &mut DeviceComponents { ref devices, ref mut partitions, .. } = &mut world.components;

    for entity in entities.keys() {
        if let Some(fs) = queued_changes.formats.remove(entity) {
            let device = &devices[entity];

            disk_ops::partition::format(device.path.as_ref(), fs)
                .map_err(|why| Error::Mkfs(device.path.clone(), fs, why))?;

            partitions[entity].filesystem = Some(fs);
        }
    }

    Ok(())
}

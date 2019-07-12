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
    let &mut DiskComponents { ref devices, ref partitions, .. } = &mut world.components;

    for (entity, flags) in entities {
        if flags.contains(Flags::FORMAT) {
            let device = &devices[entity];
            let partition = &partitions[entity];

            let fs = partition.filesystem.expect(
                "device marked for formatting \
                 which did not have a file system defined",
            );

            disk_ops::partition::format(device.path.as_ref(), fs)
                .map_err(|why| Error::Mkfs(device.path.clone(), fs, why))?;

            *flags -= Flags::FORMAT;
        }
    }

    Ok(())
}

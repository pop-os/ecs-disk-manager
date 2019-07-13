use crate::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write label")]
    LabelWrite(#[error(cause)] PartitionError),
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let entities = &mut world.entities;
    let &mut DiskComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut device_maps,
        ref mut loopbacks,
        ref mut luks,
        ref mut partitions,
        ref mut pvs,
        ref mut lvs,
        ref mut vgs,
    } = &mut world.components;

    for (parent_entity, children) in children.iter() {
        let parent_device = &devices[parent_entity];
        if let Some(ref disk) = disks.get(parent_entity) {
            let disk = &disks[parent_entity];
            let path = parent_device.path();
            super::open_partitioner(disk, path, |partitioner, table| {
                let partitioner = partitioner
                    .map_err(|why| Error::TableRead(
                        table,
                        path.into(),
                        why
                    ))?;

                for &child in children {
                    if entities[child].contains(Flags::LABEL) {
                        let partition = &partitions[child];
                        let label = partition.partlabel.as_ref().map_or("", AsRef::as_ref);
                        partitioner.label(partition.offset + 1, label)
                            .map_err(Error::LabelWrite)?;
                    }
                }

                Ok(())
            })?;
        } else {
            unimplemented!("unsupported device type")
        }
    }

    for flags in entities.values_mut() {
        *flags -= Flags::LABEL;
    }

    Ok(())
}

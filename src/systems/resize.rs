use crate::*;

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let entities = &mut world.entities;
    let &mut DeviceComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut device_maps,
        ref mut loopbacks,
        ref mut luks,
        ref luks_passphrases,
        ref mut partitions,
        ref mut pvs,
        ref mut lvs,
        ..
    } = &mut world.components;

    unimplemented!()
}

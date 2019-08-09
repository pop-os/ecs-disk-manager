use super::*;
use crate::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "placeholder")]
    Placeholder,
}

#[derive(Debug, Default)]
pub struct ResizeSystem;

impl System for ResizeSystem {
    type Err = Error;

    fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Self::Err> {
        let entities = &mut entities.devices;
        let &mut DeviceComponents {
            ref mut children,
            ref mut devices,
            ref mut disks,
            ref mut device_maps,
            ref mut loopbacks,
            ref mut luks,
            ref mut partitions,
            ref mut pvs,
            ref mut lvs,
            ..
        } = &mut components.devices;

        // TODO: Resize volumes on volume groups
        // TODO: Resize volumes on physical disks
        // TODO: Resize LVM PVs and their LVM VGs
        // TODO: Resize LUKS devices and their associated device maps

        unimplemented!()
    }
}

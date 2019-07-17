/// ! Methods for fetching information in the world.
use crate::*;

impl DiskManager {
    // If the device is a loopback, this will kdisplay the backing file.
    pub fn backing_file(&self, entity: DeviceEntity) -> Option<&Path> {
        self.components.devices.loopbacks.get(entity).map(AsRef::as_ref)
    }

    // Fetches the children of a device, for devices that have them.
    pub fn children(&self, entity: DeviceEntity) -> Option<&[DeviceEntity]> {
        self.components.devices.children.get(entity).map(Vec::as_slice)
    }

    /// Fetches a device entity by its entity ID.
    pub fn device(&self, entity: DeviceEntity) -> &Device {
        self.components
            .devices
            .devices
            .get(entity)
            .expect("invalid device entity; report this as a bug")
    }

    /// Find a device by its path.
    pub fn device_by_path(&self, path: &Path) -> Option<(DeviceEntity, &Device)> {
        self.components.devices.devices.iter().find(|(_, device)| device.path.as_ref() == path)
    }

    // If the device is a device map, this will return its name.
    pub fn device_map_name(&self, entity: DeviceEntity) -> Option<&str> {
        self.components.devices.device_maps.get(entity).map(AsRef::as_ref)
    }

    /// All entities are device entities in the world.
    pub fn devices<'a>(&'a self) -> impl Iterator<Item = (DeviceEntity, &'a Device)> + 'a {
        self.components.devices.devices.iter()
    }

    /// If the device is a disk, information about that disk can be retrieved here.
    pub fn disk(&self, entity: DeviceEntity) -> Option<&Disk> {
        self.components.devices.disks.get(entity)
    }

    /// Some device entities are LUKS crypto devices.
    pub fn crypto_luks<'a>(&'a self) -> impl Iterator<Item = DeviceEntity> + 'a {
        self.components.devices.luks.keys()
    }

    /// Some device entities are physical disks.
    pub fn disks<'a>(&'a self) -> impl Iterator<Item = (DeviceEntity, &'a Disk)> + 'a {
        self.components.devices.disks.iter()
    }

    /// For LV devices which are associated with a VG.
    pub fn lv(&self, entity: DeviceEntity) -> Option<&(LvmLv, VgEntity)> {
        self.components.devices.lvs.get(entity)
    }

    /// Some device entities are logical volumes.
    pub fn lvm_logical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (DeviceEntity, &'a LvmLv, &'a LvmVg)> + 'a {
        self.components.devices.lvs.iter().map(move |(id, (lvs, vgent))| {
            let vg = &self.components.vgs.volume_groups[*vgent];
            (id, lvs, vg)
        })
    }

    /// Some device entities are LVM physical volumes.
    pub fn lvm_physical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (DeviceEntity, &'a LvmPv, Option<&'a LvmVg>)> + 'a {
        self.components.devices.pvs.iter().map(move |(id, (pvs, vgent))| {
            let vg = vgent.map(|vgent| &self.components.vgs.volume_groups[vgent]);
            (id, pvs, vg)
        })
    }

    pub fn lvm_volume_group(&self, name: &str) -> Option<(VgEntity, &LvmVg)> {
        self.components.vgs.volume_groups.iter().find(|(_, vg)| vg.name.as_ref() == name)
    }

    pub fn lvm_volume_groups(&self) -> impl Iterator<Item = (VgEntity, &LvmVg)> {
        self.components.vgs.volume_groups.iter()
    }

    pub fn lvm_pvs_of_vg(&self, entity: VgEntity) -> impl Iterator<Item = (DeviceEntity, &LvmPv)> {
        self.components
            .devices
            .pvs
            .iter()
            .filter(move |(_, (_, pv))| *pv == Some(entity))
            .map(move |(id, (pv, _))| (id, pv))
    }

    pub fn lvm_lvs_of_vg(&self, entity: VgEntity) -> impl Iterator<Item = (DeviceEntity, &LvmLv)> {
        self.components
            .devices
            .lvs
            .iter()
            .filter(move |(_, (_, lv))| *lv == entity)
            .map(move |(id, (pv, _))| (id, pv))
    }

    /// Return the parent of this device.
    pub fn parents<'b>(&'b self, entity: DeviceEntity) -> impl Iterator<Item = DeviceEntity> + 'b {
        self.components
            .devices
            .children
            .iter()
            .filter(move |(_, pchildren)| pchildren.contains(&entity))
            .map(|(pentity, _)| pentity)
    }

    pub fn partition<'b>(&'b self, entity: DeviceEntity) -> Option<&'b Partition> {
        self.components.devices.partitions.get(entity)
    }

    /// Some device entities are partitions.
    pub fn partitions<'a>(&'a self) -> impl Iterator<Item = (DeviceEntity, &'a Partition)> + 'a {
        self.components.devices.partitions.iter()
    }

    /// For PVs which may be associated with a VG.
    pub fn pv<'b>(&'b self, entity: DeviceEntity) -> Option<(&'b LvmPv, Option<&'b LvmVg>)> {
        self.components.devices.pvs.get(entity).map(|(pv, vg_entity)| {
            let vg = vg_entity.map(|ent| &self.components.vgs.volume_groups[ent]);
            (pv, vg)
        })
    }

    /// Checks if the given sector is allocated in the partition table of the device.
    ///
    /// # Notes
    ///
    /// If the device does not support children, `false` is returned.
    pub fn sector_overlaps(&self, entity: DeviceEntity, sector: u64) -> bool {
        let devices = &self.components.devices.devices;
        let partitions = &self.components.devices.partitions;

        self.components.devices.children.get(entity).map_or(false, |children| {
            children.iter().any(|&child| {
                let device = &devices[child];
                let partition = &partitions[child];

                sector >= partition.offset && sector <= device.sectors
            })
        })
    }
}

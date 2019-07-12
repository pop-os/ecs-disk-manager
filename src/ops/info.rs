///! Methods for fetching information in the world.
use crate::*;

impl DiskManager {
    // If the device is a loopback, this will kdisplay the backing file.
    pub fn backing_file(&self, entity: Entity) -> Option<&Path> {
        self.components.loopbacks.get(entity).map(AsRef::as_ref)
    }

    // Fetches the children of a device, for devices that have them.
    pub fn children(&self, entity: Entity) -> Option<&[Entity]> {
        self.components.children.get(entity).map(Vec::as_slice)
    }

    /// Fetches a device entity by its entity ID.
    pub fn device(&self, entity: Entity) -> &Device {
        self.components.devices.get(entity).expect("invalid device entity; report this as a bug")
    }

    /// Find a device by its path.
    pub fn device_by_path(&self, path: &Path) -> Option<(Entity, &Device)> {
        self.components.devices.iter().find(|(_, device)| device.path.as_ref() == path)
    }

    // If the device is a device map, this will return its name.
    pub fn device_map_name(&self, entity: Entity) -> Option<&str> {
        self.components.device_maps.get(entity).map(AsRef::as_ref)
    }

    /// All entities are device entities in the world.
    pub fn devices<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Device)> + 'a {
        self.components.devices.iter()
    }

    /// If the device is a disk, information about that disk can be retrieved here.
    pub fn disk(&self, entity: Entity) -> Option<&Disk> {
        self.components.disks.get(entity)
    }

    /// Some device entities are LUKS crypto devices.
    pub fn crypto_luks<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Luks)> + 'a {
        self.components.luks.iter()
    }

    /// Some device entities are physical disks.
    pub fn disks<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Disk)> + 'a {
        self.components.disks.iter()
    }

    // If the device is a LUKS partition, information about the LUKS device is here.
    pub fn luks(&self, entity: Entity) -> Option<&Luks> {
        self.components.luks.get(entity)
    }

    /// For LV devices which are associated with a VG.
    pub fn lv(&self, entity: Entity) -> Option<(&LvmVg, &LvmLv)> {
        self.components
            .lvs
            .get(entity)
            .map(|(lv, vg_entity)| (self.components.vgs.get(*vg_entity), lv))
    }

    /// Some device entities are logical volumes.
    pub fn lvm_logical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Entity, &'a LvmLv, &'a LvmVg)> + 'a {
        self.components.lvs.iter().map(move |(id, (lvs, vgent))| {
            let vg = self.components.vgs.get(*vgent);
            (id, lvs, vg)
        })
    }

    /// Some device entities are LVM physical volumes.
    pub fn lvm_physical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Entity, &'a LvmPv, Option<&'a LvmVg>)> + 'a {
        self.components.pvs.iter().map(move |(id, (pvs, vgent))| {
            let vg = vgent.map(|vgent| self.components.vgs.get(vgent));
            (id, pvs, vg)
        })
    }

    pub fn lvm_volume_groups(&self) -> impl Iterator<Item = (VgEntity, &LvmVg)> {
        self.components.vgs.iter()
    }

    pub fn lvm_pvs_of_vg(&self, entity: VgEntity) -> impl Iterator<Item = (Entity, &LvmPv)> {
        self.components
            .pvs
            .iter()
            .filter(move |(_, (_, pv))| *pv == Some(entity))
            .map(move |(id, (pv, _))| (id, pv))
    }

    pub fn lvm_lvs_of_vg(&self, entity: VgEntity) -> impl Iterator<Item = (Entity, &LvmLv)> {
        self.components
            .lvs
            .iter()
            .filter(move |(_, (_, lv))| *lv == entity)
            .map(move |(id, (pv, _))| (id, pv))
    }

    /// Return the parent of this device.
    pub fn parents<'b>(&'b self, entity: Entity) -> impl Iterator<Item = Entity> + 'b {
        self.components
            .children
            .iter()
            .filter(move |(pentity, pchildren)| pchildren.contains(&entity))
            .map(|(pentity, _)| pentity)
    }

    pub fn partition<'b>(&'b self, entity: Entity) -> Option<&'b Partition> {
        self.components.partitions.get(entity)
    }

    /// Some device entities are partitions.
    pub fn partitions<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Partition)> + 'a {
        self.components.partitions.iter()
    }

    /// For PVs which may be associated with a VG.
    pub fn pv<'b>(&'b self, entity: Entity) -> Option<(Option<&'b LvmVg>, &'b LvmPv)> {
        self.components.pvs.get(entity).map(|(pv, vg_entity)| {
            let vg = vg_entity.map(|ent| self.components.vgs.get(ent));
            (vg, pv)
        })
    }
}

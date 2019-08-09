/// ! Method for creating a new partition entities in the world.
use crate::*;
use disk_types::*;
use loopdev::LoopControl;
use std::path::PathBuf;

/// The default LVM2 header size is 1MiB.
const LVM_DEFAULT_HEADER_SIZE: u64 = 1024 * 1024;
/// The default LVM2 extent size is 4MiB.
const LVM_DEFAULT_EXTENT_SIZE: u64 = 4 * 1024 * 1024;

/// An error that may occur when adding creation operations to the queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error(display = "the new partition exceeds the size of the parent device")]
    ExceedsDevice,
    #[error(display = "a supplied device entity was expected to be a LVM PV")]
    ExpectedLvmPv,
    #[error(display = "the new partition overlaps an existing partition")]
    PartitionOverlap,
    #[error(display = "the end sector lies before the start sector")]
    InputsInverted,
    #[error(display = "parent device is not partitionable")]
    NotPartitionable,
    #[error(display = "cannot create table on device")]
    TablesUnsupported,
}

#[derive(Debug, Error)]
pub enum LoopbackError {
    #[error(display = "failed to attach file to loopback device")]
    Attach(#[error(cause)] io::Error),
    #[error(display = "failed to probe newly-attached loopback device")]
    BlockProber(#[error(cause)] BlockProbeError),
    #[error(display = "failed to open loopback control")]
    ControlOpen(#[error(cause)] io::Error),
    #[error(display = "failed to create loopback file")]
    FileCreate(#[error(cause)] io::Error),
    #[error(display = "failed to set length of loopback file")]
    FileSetLen(#[error(cause)] io::Error),
    #[error(display = "failed to find next free loopback device address")]
    NextFree,
    #[error(display = "failed to find the loopback device to probe")]
    ProbeNotFound,
}

/// Defines to create either a plain partition, or a LUKS-encrypted partition.
pub enum PartitionCreate {
    /// Create a simple, plain file system on the partition.
    Plain(FileSystem),
    /// Create a LUKS device, with an optional passphrase.
    Luks(LuksParams),
}

/// Defines how the new file system will be created.
pub enum CreateAs {
    /// Creating a logical volume on a volume group.
    LogicalVolume { parent: VgEntity, length: Sector, name: Box<str> },

    /// Creating a partition on a device
    PartitionedDevice { parent: DeviceEntity, start: Sector, end: Sector },
}

impl DiskManager {
    /// Create a new loopback device, instantly.
    pub fn loopback_create(
        &mut self,
        path: Box<Path>,
        size_in_bytes: u64,
    ) -> Result<DeviceEntity, LoopbackError> {
        use std::{fs::File, io::Write};

        File::create(&path)
            .map_err(LoopbackError::FileCreate)?
            .set_len(size_in_bytes)
            .map_err(LoopbackError::FileSetLen)?;

        self.loopback_open(path)
    }

    pub fn loopback_open(&mut self, path: Box<Path>) -> Result<DeviceEntity, LoopbackError> {
        let control = LoopControl::open().map_err(LoopbackError::ControlOpen)?;
        let device = control.next_free().map_err(|_| LoopbackError::NextFree)?;
        device.attach_file(&path).map_err(LoopbackError::Attach)?;
        let device_path: Box<Path> =
            device.path().expect("attached loopback device does not have a path").into();

        // TODO: This section is partially copied from the scanning system.
        let prober = BlockProber::new().map_err(LoopbackError::BlockProber)?;

        for res in prober.into_iter() {
            if let Some(probed) = res.map_err(LoopbackError::BlockProber)? {
                if probed.path == device_path {
                    let info = probed.probe().map_err(LoopbackError::BlockProber)?;

                    let entity = self.entities.devices.insert(EntityFlags::SUPPORTS_TABLE);
                    self.components.devices.loopbacks.insert(entity, path);
                    self.components.devices.devices.insert(
                        entity,
                        Device {
                            name:                 Box::from(info.device),
                            path:                 device_path,
                            sectors:              info.sectors,
                            logical_sector_size:  info.logical_sector_size,
                            physical_sector_size: info.physical_sector_size,
                        },
                    );

                    if let Some(fstype) = info.fstype {
                        self.components.devices.partitions.insert(
                            entity,
                            Partition {
                                offset:      0,
                                number:      0,
                                filesystem:  fstype.parse().ok(),
                                partuuid:    None,
                                partlabel:   None,
                                mbr_variant: PartitionType::Primary,
                                uuid:        info.uuid,
                            },
                        );
                    }

                    // TODO: Handle loopback devices with LUKS and LVM partitions.

                    return Ok(entity);
                }
            }
        }

        let _ = device.detach();
        Err(LoopbackError::ProbeNotFound)
    }

    /// Create a file system directly on a device.
    pub fn create_on(&mut self, device: DeviceEntity, what: PartitionCreate) -> Result<(), Error> {
        self.assert_not_creating_table_on(device);

        let (sectors, logical_sector_size, physical_sector_size) = {
            let device_info = &self.components.devices.devices[device];
            (device_info.sectors, device_info.logical_sector_size, device_info.physical_sector_size)
        };

        let offset = match self.components.devices.partitions.get(device) {
            Some(partition) => partition.offset,
            None => 0,
        };

        self.create_partition(
            what,
            device,
            None,
            offset,
            sectors,
            logical_sector_size,
            physical_sector_size,
            |manager| {
                let device_components = &manager.components.devices;
                let device = manager
                    .components
                    .queued_changes
                    .devices
                    .get(device)
                    .or_else(|| device_components.devices.get(device))
                    .expect("device entity without device component");

                device.path.to_path_buf().into()
            },
        );

        self.entities.devices[device] |= EntityFlags::CREATE;

        self.flags |= ManagerFlags::CREATE;

        Ok(())
    }

    /// Create a new logical volume on a volume group.
    pub fn create_as_logical_volume_of(
        &mut self,
        parent: VgEntity,
        sector: Sector,
        name: Box<str>,
        what: PartitionCreate,
    ) -> Result<DeviceEntity, Error> {
        let mut lazy_lvpath = None;
        let (length, dmname);

        {
            let vg_components = &self.components.vgs;
            let vg = self
                .components
                .queued_changes
                .volume_groups
                .get(parent)
                .or_else(|| vg_components.volume_groups.get(parent))
                .expect("vg entity without vg component");

            length = self.can_create_on_vg(parent, vg, sector)?;

            dmname = [vg.name.replace("-", "--").as_str(), "-", name.replace("-", "--").as_str()]
                .concat()
                .into();
        }

        let fetch_lv_path = |manager: &Self, device: VgEntity, lvname: &str| -> Box<Path> {
            let vg_components = &manager.components.vgs;

            let vg = manager
                .components
                .queued_changes
                .volume_groups
                .get(device)
                .or_else(|| vg_components.volume_groups.get(device))
                .expect("vg entity without vg component");

            PathBuf::from(["/dev/mapper/", &vg.name, "/", lvname].concat()).into()
        };

        // Create a new device entity for the new partition.
        let entity = self.entities.devices.insert(EntityFlags::CREATE);

        let lvpf = |manager: &mut Self| {
            let path = fetch_lv_path(&manager, parent, &name);
            lazy_lvpath = Some(path.clone());
            path
        };

        self.create_partition(what, entity, None, 0, length, 512, 512, lvpf);

        // Associate the newly-queued device with the parent.
        self.components.queued_changes.vg_parents.insert(entity, parent);

        let path: Box<Path> =
            lazy_lvpath.take().unwrap_or_else(|| fetch_lv_path(&self, parent, &name));

        let lv = LvmLv { name, path, uuid: Box::from("") };

        let queued = &mut self.components.queued_changes;
        queued.device_maps.insert(entity, dmname);
        queued.lvs.insert(entity, (lv, parent));

        Ok(entity)
    }

    /// Create a new partition on a partitionable device.
    pub fn create_as_child_of(
        &mut self,
        parent: DeviceEntity,
        start: Sector,
        end: Sector,
        label: Box<str>,
        what: PartitionCreate,
    ) -> Result<DeviceEntity, Error> {
        self.assert_not_creating_table_on(parent);

        let (offset, length, logical_sector_size, physical_sector_size) = {
            let device_components = &self.components.devices;
            let device = if self.entities.devices[parent].contains(EntityFlags::CREATE) {
                &self.components.queued_changes.devices[parent]
            } else {
                &device_components.devices[parent]
            };

            let sectors = self.can_create_on_device(parent, device, start, end)?;

            (sectors.0, sectors.1, device.logical_sector_size, device.physical_sector_size)
        };

        // Create a new device entity for the new partition.
        let entity = self.entities.devices.insert(EntityFlags::CREATE);

        self.create_partition(
            what,
            entity,
            Some(label),
            offset,
            length,
            logical_sector_size,
            physical_sector_size,
            |manager| {
                let device_components = &manager.components.devices;
                let device = manager
                    .components
                    .queued_changes
                    .devices
                    .get(parent)
                    .or_else(|| device_components.devices.get(parent))
                    .expect("device entity without device component");

                device.path.to_path_buf().into()
            },
        );

        self.components.queued_changes.parents.insert(entity, parent);

        self.entities.devices[parent] |= EntityFlags::CREATE | EntityFlags::CREATE_CHILDREN;

        // Remind the manager that the creation system must be run.
        self.flags |= ManagerFlags::CREATE;

        Ok(entity)
    }

    /// Define that a new partition table will be written to this device.
    pub fn create_table(
        &mut self,
        entity: DeviceEntity,
        kind: PartitionTable,
    ) -> Result<(), Error> {
        self.assert_not_creating_partition_on(entity);

        if !self.entities.devices[entity].contains(EntityFlags::SUPPORTS_TABLE) {
            return Err(Error::TablesUnsupported);
        }

        self.components.queued_changes.tables.insert(entity, kind);

        self.entities.devices[entity] |= EntityFlags::CREATE;
        self.flags |= ManagerFlags::CREATE;

        // Mark this device to be wiped, and its children freed.
        self.remove(entity);

        Ok(())
    }

    /// Define that a new volume group is to be created
    pub fn volume_group_create(
        &mut self,
        name: &str,
        with: &HashSet<DeviceEntity>,
    ) -> Result<(), Error> {
        let mut extents = 0;

        {
            let devices = &self.components.devices;
            let queued = &self.components.queued_changes;

            for &entity in with {
                match devices.pvs.get(entity).or_else(|| queued.pvs.get(entity)) {
                    Some(pvdata) => {
                        extents += pvdata.0.size_bytes / LVM_DEFAULT_EXTENT_SIZE;
                    }
                    None => return Err(Error::ExpectedLvmPv),
                }
            }
        }

        let vg_entity = self.entities.vgs.insert(EntityFlags::CREATE);

        let lvm_vg = LvmVg {
            name: Box::from(name),
            extent_size: LVM_DEFAULT_EXTENT_SIZE,
            extents,
            extents_free: extents,
        };

        self.components.queued_changes.volume_groups.insert(vg_entity, lvm_vg);

        for &entity in with {
            self.components.queued_changes.pv_parents.insert(entity, vg_entity);
        }

        Ok(())
    }

    fn assert_not_creating_table_on(&self, device: DeviceEntity) {
        debug_assert!(
            self.components.queued_changes.tables.contains_key(device),
            "attempted to create a file system on a device that is marked for creation of a \
             partition table"
        );
    }

    fn assert_not_creating_partition_on(&self, device: DeviceEntity) {
        debug_assert!(
            self.components.queued_changes.partitions.contains_key(device),
            "attempted to create a partition on a device that was marked to be formatted"
        );
    }

    /// Checks if a partition can be inserted into the sectors of this device.
    ///
    /// Returns the sectors where this partition will be created, if it can be created.
    fn can_create_on_device(
        &self,
        parent: DeviceEntity,
        device: &Device,
        start: Sector,
        end: Sector,
    ) -> Result<(u64, u64), Error> {
        let entities = &self.entities.devices;

        fn validate(
            partdev: &Device,
            partition: &Partition,
            offset: u64,
            end: u64,
        ) -> Result<(), Error> {
            // The end of the new partition is before the start of the current.
            let before = || end < partition.offset;

            // The start of the new partition is after the end of the current.
            let after = || offset > partition.offset + partdev.sectors;

            if before() || after() {
                Ok(())
            } else {
                Err(Error::PartitionOverlap)
            }
        }

        // For all children of the device, guarantee that there is no overlap.
        match self.components.devices.children.get(parent) {
            Some(children) => {
                let offset = device.get_sector(start);
                let end = device.get_sector(end);

                // The start sector must be less than end sector.
                if offset >= end {
                    return Err(Error::InputsInverted);
                }

                // The end sector must also exist on the device.
                if device.sectors < end {
                    return Err(Error::ExceedsDevice);
                }

                // Check if there is any overlap, ignoring children marked for removal.
                for &child in children {
                    let child_flags = entities[child];
                    if !child_flags.contains(EntityFlags::REMOVE) {
                        let partdev = &self.components.devices.devices[child];
                        let partition = &self.components.devices.partitions[child];
                        validate(partdev, partition, offset, end)?;
                    }
                }

                // Check if there is an overlap with other partitions queued to be created.
                for (child, &cparent) in self.components.queued_changes.parents.iter() {
                    if cparent == parent {
                        let partdev = &self.components.queued_changes.devices[child];
                        let partition = &self.components.queued_changes.partitions[child];
                        validate(partdev, partition, offset, end)?;
                    }
                }

                Ok((offset, end - offset))
            }
            None => Err(Error::NotPartitionable),
        }
    }

    fn can_create_on_vg(
        &self,
        entity: VgEntity,
        parent: &LvmVg,
        length: Sector,
    ) -> Result<u64, Error> {
        let queued = &self.components.queued_changes;
        let length = parent.get_sector(length);

        // Other LVs may be queued for addition, so we will also consider their lengths.
        let adding: u64 = queued
            .lvs
            .iter()
            .filter(|(_, (_, centity))| *centity == entity)
            .map(|(lv, _)| {
                let device = &queued.devices[lv];
                device.logical_sector_size() * device.sectors() / 512
            })
            .sum();

        if length <= parent.sectors_free() - adding {
            Ok(length)
        } else {
            Err(Error::ExceedsDevice)
        }
    }

    fn create_partition<F: FnOnce(&mut Self) -> Box<Path>>(
        &mut self,
        what: PartitionCreate,
        entity: DeviceEntity,
        label: Option<Box<str>>,
        offset: u64,
        sectors: u64,
        logical_sector_size: u64,
        physical_sector_size: u64,
        lvm_path: F,
    ) {
        // Followed by the partition component.
        let mut partition = Partition { partlabel: label, offset, ..Default::default() };

        // Are you are a LUKS device, or a plain-old-filesystem?
        match what {
            PartitionCreate::Plain(filesystem) => {
                // Mark the device as a future PV if it is LVM.
                if let FileSystem::Lvm = filesystem {
                    let path: Box<Path> = lvm_path(self);

                    let size_bytes = logical_sector_size * sectors - LVM_DEFAULT_HEADER_SIZE;
                    let adjusted = (size_bytes / LVM_DEFAULT_EXTENT_SIZE) * LVM_DEFAULT_EXTENT_SIZE;

                    let pv = LvmPv { path, uuid: Box::from(""), size_bytes: adjusted };

                    self.components.queued_changes.pvs.insert(entity, (pv, None));
                }

                partition.filesystem = Some(filesystem);
            }
            PartitionCreate::Luks(luks) => {
                // Specify that the partition is a LUKS device, and create a new device entity
                // which will represent the newly-activated device.
                partition.filesystem = Some(FileSystem::Luks);
                let flags = EntityFlags::CREATE | EntityFlags::LUKS_CHILD;
                let child = self.entities.devices.insert(flags);
                self.components.queued_changes.luks.insert(entity, (child, luks));
            }
        }

        let queued = &mut self.components.queued_changes;

        // Then create the device component for the new entity.
        queued.devices.insert(
            entity,
            Device {
                name: Box::from(""),
                path: Box::from(Path::new("")),
                sectors,
                logical_sector_size,
                physical_sector_size,
            },
        );

        // Add the partition component to the entity.
        queued.partitions.insert(entity, partition);
    }
}

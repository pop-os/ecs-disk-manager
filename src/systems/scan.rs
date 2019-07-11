#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "linux")]
mod linux {
    use crate::{*, Error as DiskError};
    use disk_types::*;
    use std::fs::read_link;

    pub fn scan(world: &mut DiskManager) -> Result<(), DiskError> {
        let prober = BlockProber::new().map_err(DiskError::BlockProber)?;
        for res in prober.into_iter().filter_map(Result::transpose) {
            let probed = res.map_err(DiskError::BlockProber)?;
            let info = probed.probe().map_err(DiskError::BlockProber)?;

            let whole_entity = world.entities.insert(Flags::empty());

            world.components.devices.insert(
                whole_entity,
                Device {
                    name: Box::from(info.device),
                    path: Box::from(info.path),
                    sectors: info.sectors,
                    logical_sector_size: info.logical_sector_size,
                    physical_sector_size: info.physical_sector_size,
                },
            );

            match info.variant {
                DeviceVariant::Loopback(backing_file) => {
                    world.components.loopbacks.insert(whole_entity, backing_file);
                }
                DeviceVariant::Map(devmapper) => {
                    world.components.device_maps.insert(whole_entity, devmapper);
                }
                DeviceVariant::Physical(table) => {
                    world.components.disks.insert(whole_entity, Disk { serial: "".into(), table });
                }
            }

            if let Some(fstype) = info.fstype {
                world.components.partitions.insert(
                    whole_entity,
                    Partition {
                        offset: 0,
                        number: 0,
                        filesystem: fstype.parse().ok(),
                        partuuid: None,
                        partlabel: None,
                        mbr_variant: PartitionType::Primary,
                        uuid: info.uuid,
                    },
                );
            }

            let mut children = Vec::new();
            for partition in info.partitions {
                let part_entity = world.entities.insert(Flags::empty());
                children.push(part_entity);

                world.components.devices.insert(
                    part_entity,
                    Device {
                        name: partition.device,
                        path: partition.path,
                        sectors: partition.sectors,
                        logical_sector_size: info.logical_sector_size,
                        physical_sector_size: info.physical_sector_size,
                    },
                );

                world.components.partitions.insert(
                    part_entity,
                    Partition {
                        offset: partition.offset,
                        number: partition.no,
                        filesystem: partition.fstype.and_then(|fstype| fstype.parse().ok()),
                        partuuid: partition.partuuid,
                        partlabel: partition.partlabel,
                        mbr_variant: PartitionType::Primary,
                        uuid: partition.uuid,
                    },
                );
            }

            world.components.children.insert(whole_entity, children);
        }

        associate_children(world);

        if let Err(why) = associate_lvm_devices(world) {
            eprintln!("failed to associate lvm devices: {}", why);
            eprintln!("    is the lvmdbus1 daemon installed?");
        }

        Ok(())
    }

    fn associate_children(world: &mut DiskManager) {
        let &mut DiskComponents { ref devices, ref mut children, .. } =
            &mut world.components;

        for (entity, device) in devices {
            for slave in slaves_iter(&device.name) {
                for (other_entity, other_device) in devices {
                    if other_device.name == slave {
                        println!(
                            "mapping parent-child association: {} <-> {}",
                            device.path.display(),
                            other_device.path.display()
                        );

                        match children.get_mut(other_entity) {
                            Some(associations) => associations.push(entity),
                            None => drop(children.insert(other_entity, vec![entity])),
                        }
                    }
                }
            }
        }
    }

    fn associate_lvm_devices(world: &mut DiskManager) -> Result<(), DiskError> {
        let lvm_prober = LvmProber::new().map_err(DiskError::LvmProber)?;

        let &mut DiskComponents {
            ref device_maps,
            ref devices,
            ref partitions,
            ref mut lvs,
            ref mut pvs,
            ref mut vgs,
            ..
        } = &mut world.components;

        let mut found_pvs = Vec::new();

        let mut append_pv = |vg_entity, node: u32, pv: LvmPv| {
            let pv_dm_name = pv.path.file_name().expect("PV without name");
            for entity in partitions.keys() {
                if let Some(dm_name) = device_maps.get(entity) {
                    if pv_dm_name == dm_name.as_ref() {
                        if !found_pvs.iter().any(|&pv| pv == node) {
                            found_pvs.push(node);
                            let device = devices.get(entity).unwrap();
                            eprintln!(
                                "associating {} to {}",
                                pv.path.display(),
                                device.path.display()
                            );
                            pvs.insert(entity, (pv.clone(), vg_entity));
                        }
                        break;
                    }
                }
            }
        };

        for vg in lvm_prober.iter_vgs() {
            let vg = vg.map_err(DiskError::LvmProber)?;

            let vg_entity = vgs.insert(LvmVg {
                name: vg.name.clone().into(),
                extent_size: vg.extent_size,
                extents: vg.extents,
                extents_free: vg.extents_free,
            });

            for lv in vg.lvs {
                let lv_path = read_link(&lv.path).expect("LV path is not a symlink");
                for entity in partitions.keys() {
                    if device_maps.contains_key(entity) {
                        let device = devices.get(entity).unwrap();
                        if lv_path.file_name() == device.path.file_name() {
                            lvs.insert(entity, (lv.clone(), vg_entity));
                            break;
                        }
                    }
                }
            }

            for (node, pv) in vg.pvs {
                append_pv(Some(vg_entity), node, pv);
            }
        }

        for result in lvm_prober.iter_pvs() {
            let (node, pv) = result.map_err(DiskError::LvmProber)?;
            append_pv(None, node, pv);
        }

        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn scan(world: &mut DiskManager) -> Result<(), DiskError> {
    compile_error!("Only Linux is supported at the moment");
}

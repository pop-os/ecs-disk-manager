use crate::*;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn create_loopback(manager: &mut DiskManager) -> DeviceEntity {
    manager
        .loopback_create(Box::from(Path::new("loopback.bin")), 2 * 1024 * 1024 * 1024)
        .expect("failed to create loopback device in manager")
}

fn setup<F: FnOnce(DiskManager, DeviceEntity)>(func: F) {
    let mut manager = DiskManager::default();
    let entity = create_loopback(&mut manager);
    func(manager, entity)
}

fn apply(manager: &mut DiskManager) {
    manager.apply(&Arc::new(AtomicBool::new(false))).unwrap();

    for (entity, flags) in &manager.entities.devices {
        if flags.contains(EntityFlags::CREATE) {
            panic!("device still has a create flag");
        }
    }
}

#[test]
fn create_fs_on_loopback() {
    setup(|mut manager, entity| {
        manager.create_on(entity, ops::create::PartitionCreate::Plain(FileSystem::Ext4)).unwrap();
    });
}

#[test]
fn create_partition_table() {
    setup(|mut manager, entity| {
        // Create a GUID partition table on the loopback device.
        manager.create_table(entity, PartitionTable::Guid).unwrap();

        // Create the EFI partition.
        let entity_efi = manager
            .create_as_child_of(
                entity,
                Sector::Start,
                Sector::Megabyte(100),
                Box::from("EFI"),
                ops::create::PartitionCreate::Plain(FileSystem::Vfat),
            )
            .unwrap();

        // Create a root partition.
        let entity_root = manager
            .create_as_child_of(
                entity,
                Sector::Megabyte(100),
                Sector::MegabyteFromEnd(1000),
                Box::from("Root"),
                ops::create::PartitionCreate::Plain(FileSystem::Ext4),
            )
            .unwrap();

        let entity_swap = manager
            .create_as_child_of(
                entity,
                Sector::MegabyteFromEnd(1000),
                Sector::End,
                Box::from("Swap"),
                ops::create::PartitionCreate::Plain(FileSystem::Swap),
            )
            .unwrap();

        // Apply the operations
        apply(&mut manager);

        // Validate that what was applied, now exists in the manager.
        let children = manager.children(entity).expect("did not create children");
        for &child in children {
            let device = &manager.components.devices.devices[child];
            let partition = &manager.components.devices.partitions[child];

            if child == entity_efi {
                eprintln!("EFI sector length: {}", device.sectors);
                assert_eq!(device.sectors, 100 * 1024 * 1024 / 512, "EFI sector length is off");
                assert_eq!(partition.filesystem, Some(FileSystem::Vfat));
            } else if child == entity_root {
                assert_eq!(partition.filesystem, Some(FileSystem::Ext4));
            } else if child == entity_swap {
                assert_eq!(device.sectors, 1000 * 1024 * 1024 / 512, "Swap sector length is off");
                assert_eq!(partition.filesystem, Some(FileSystem::Swap));
            } else {
                panic!("device has an entity which the test did not create");
            }
        }

        // TODO: Validate that what is in the manager actually exists in the system.
    });
}

#[test]
fn partitions_add() {}

#[test]
fn partitions_remove() {}

#[test]
fn partitions_add_and_remove() {}

#[test]
fn partitions_resize() {}

#[test]
fn partitions_move() {}

#[test]
fn fs_on_luks() {}

#[test]
fn fs_on_lvm() {}

#[test]
fn luks_on_lvm_create() {}

#[test]
fn luks_on_lvm_extend() {}

#[test]
fn luks_on_lvm_modify() {}

#[test]
fn lvm_on_luks_create() {}

#[test]
fn lvm_on_luks_extend() {}

#[test]
fn lvm_on_luks_modify() {}

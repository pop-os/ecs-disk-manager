# ECS Disk Manager

This is an experiment to create a complete disk management library in Rust around an ECS architecture, exposing a simple API for programs to interact with. In addition to serving as a library, it will also be provided as a daemon with RPC capabilities to interact with software written in other languages, or that would prefer not to link a disk manager into their application.

## Features

- Device scanning
  - Disk devices
  - Loopback devices
  - LVM devices
  - LUKS devices

### Planned

- RPC services
  - TARPC
  - DBUS
- Device management
  - Create LVM VGs, PVs, and LVs
  - Partition table management
    - MBR
    - GUID

## Goal

The goal of this project will be to replace all of the complex disk management logic in `distinst`. In order for `distinst` to do what it needs to do with disks, it contains a complete disk management solution embedded into it. However, this disk management solution is too complex, difficult to maintain, and only handles a limited number of disk configuration scenarios. In addition, it relies heavily on `libarted`, which is incredibly slow and cannot be used in a multi-threaded context.

A byproduct of having a standalone solution as a library would be the possibility of the development of new disk management tools written in Rust. GParted is showing its age and lacks support for Wayland, LVM, and LUKS. GNOME Disks supports Wayland, LVM, and LUKS but has limited disk management capabilities. New tools based on this could potentially be as useful on Linux as it could on Redox OS, as well.

## Architecture

Currently, the ECS is custom-built using the `slotmap` crate as EC half of the ECS. It provides a `SlotMap` for storing entities, and `SecondaryMap`s for storing the components associated with our entities. There was a previous attempt using SPECS as the ECS, but it proved to be too complex and inflexible for the needs of a disk manager.

To source information about block devices, devices on the system are sourced from `/proc/partitions`, and then probed using bindings for `libblkid`, and supplemented with additional information from the kernel's `/sys/class/block` file system. LVM support is offered through the `lvmdbus1` DBus API. LUKS support is provided by bindings to `libcryptsetup`.

### Scanning

The scan system is used to fill the world with device entities, and their associated components describe these devices and their associations to other devices in the world. It's therefore possible to construct a complex graph of device associations that describe how each device relates to each other. All of which is achieved both efficiently and safely in Rust.

```rust
use ecs_disk_manager::*;

fn main() {
    let mut world = DiskManager::new().unwrap();

    world.scan().unwrap();
}
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

> Note that on Linux, libcryptsetup and libblkid are licensed under LGPL-2.0+. Therefore, do not statically link them if your application is not LGPL-2.0+ or GPL.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

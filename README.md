# ECS Disk Manager

This is an experiment to create a complete disk management library in Rust around an ECS architecture, exposing a simple API for programs to interact with. In addition to serving as a library, it will also be provided as a daemon with RPC capabilities to interact with software written in other languages, or that would prefer not to link a disk manager into their application.

## Goal

The goal of this project will be to replace all of the complex disk management logic in `distinst`. In order for `distinst` to do what it needs with disks, it contains a complete disk management solution embedded into it. However, this disk management solution is too complex, difficult to maintain, and only handles a limited number of disk configuration scenarios. In addition, it relies heavily on `libarted`, which is incredibly slow and cannot be used in a multi-threaded context.

A byproduct of having a standalone solution as a library would be the possibility of the development of new disk management tools written in Rust. GParted is showing its age and lacks support for Wayland, LVM, and LUKS. GNOME Disks supports Wayland, LVM, and LUKS but has limited disk management capabilities. New tools based on this could potentially be as useful on Linux as it could on Redox OS, as well.

## Architecture

The `DiskManager` structure, referred to as the "world", contains three inner crucial fields: `DiskEntities`, `DiskComponents`, and `DiskSystems`. When a `DiskSystem`'s `run()` method is executed in the world, it uniquely borrows itself, the `DiskEntities`, and `DiskComponents` fields. Therefore, each system has full control over the creation, removal, and modification of any entity and system in the world. It is able to do so safely, without reference counting, and full mutability.

> This implementation is based on [slotmap](https://crates.io/crates/slotmap).

### Seeding the World

To source information about block devices, devices on the system are sourced from `/proc/partitions`, and then probed using bindings for `libblkid`, and supplemented with additional information from the kernel's `/sys/class/block` file system. LVM support is offered through the `lvmdbus1` DBus API. LUKS support is provided by bindings to `libcryptsetup`. These are all part of the scanning system.

> Although currently Linux-specific, it is possible to add support for other operating systems.

### Entities and Components

There are currently two types of entities in the world that may be encountered: **devices** and **volume groups**. Devices are any addressable device in the system, such as loopbacks, disks, partitions, and logical volumes. Volume groups are not directly addressable to the system; but have LVM physical volumes associated with them; and when activated, provide logical volumes as addressable devices in the system.

Both of the `DiskEntities` and `DiskComponents` structures have inner fields to separate the two. Components specific to devices are found in the `components.devices` field, and components specific to volume groups are in `components.vgs`.

There is also an additional `QueuedChanges` field in `DiskComponents`, which is where all modifications to components are queued and eventually pulled and applied during system execution. This separates the theoretical state from the actual state, so that cancellations may be applied cleanly, or errors during system execution handled gracefully without requiring to probe the system again.

### The Systems

In addition to the scanning system, the following systems are planned to exist in the world, and are executed in the following order:

1. `RemoveSystem`: Removes devices
2. `ResizeSystem`: Resizes devices
3. `CreationSystem`: Creates devices
4. `ModificationSystem`: Formats devices with file systems, and changes partition labels

Among the benefits to the systems approach is that each system can safely read and apply actions to the world in parallel. For example, the `RemoveSystem` could open the partition tables for multiple devices at the same time, and write all changes in parallel. Likewise, the `ModificationSystem` could format every partition marked for formatting in parallel.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

> Note that on Linux, libcryptsetup and libblkid are licensed under LGPL-2.0+. Therefore, do not statically link them if your application is not LGPL-2.0+ or GPL.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

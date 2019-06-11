extern crate lvmdbus1;

use lvmdbus1::VolumeGroup;

fn main() {
    let interface = VolumeGroup::new().unwrap();

    for vg in interface.iter() {
        println!("{}", vg.name().unwrap());
        println!("{:?}", vg.pvs().unwrap());
    }

    // for id in 0.. {
    //     let vg = volume_group.connect(id);

    //     println!("{}", vg.get_name().unwrap());
    // }
}


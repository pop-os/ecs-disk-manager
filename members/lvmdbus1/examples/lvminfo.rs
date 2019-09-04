extern crate lvmdbus1;

use lvmdbus1::{LvConn, LvmConn, LvmPath, PvConn, VgConn};

fn main() {
    let interface = VgConn::new().unwrap();

    for vg in interface.iter() {
        println!("{}", vg.name().unwrap());
        println!("  EXTENT_SIZE:  {}", vg.extent_size_bytes().unwrap());
        println!("  EXTENTS:      {}", vg.extent_count().unwrap());
        println!("  EXTENTS_FREE: {}", vg.extent_free_count().unwrap());

        println!("  PVS:          {}", vg.pv_count().unwrap());
        for path in vg.pvs() {
            let pv = PvConn::new().unwrap();
            let pv = pv.connect_with_path(path);
            println!("    PV {}", pv.name().unwrap());
            println!("      UUID:     {}", pv.uuid().unwrap());
        }

        println!("  LVS:          {}", vg.lv_count().unwrap());
        for path in vg.lvs() {
            let lv = LvConn::new().unwrap();
            let lv = lv.connect_with_path(path);
            println!("    LV {}", lv.name().unwrap());
            println!("      UUID:     {}", lv.uuid().unwrap());
            println!("      PATH:     {:?}", lv.path().unwrap());
            println!("      BYTES:    {}", lv.size_bytes().unwrap());
        }
    }
}

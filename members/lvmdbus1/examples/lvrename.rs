extern crate lvmdbus1;

use lvmdbus1::{LvConn, LvmConn, LvmPath, PvConn, VgConn};
use std::{collections::HashMap, env, process::exit};

fn main() {
    let interface = VgConn::new().unwrap();
    let mut args = env::args().skip(1);

    if let (Some(group), Some(new_name)) = (args.next(), args.next()) {
        for vg in interface.iter() {
            if vg.name().unwrap() == group {
                if let Err(why) = vg.rename(&new_name, HashMap::new()) {}

                return;
            }
        }
    } else {
        eprintln!("requires two arguments: GROUP NEW_NAME");
        exit(1);
    }
}

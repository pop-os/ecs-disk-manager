#[macro_use]
extern crate prettytable;

use disk_prober::*;
use prettytable::{format::FormatBuilder, Cell, Row, Table};

fn main() {
    let mut table = Table::new();
    table.set_format(FormatBuilder::new().borders(' ').column_separator(' ').padding(0, 0).build());

    table.add_row(row![
        "DEVICE",
        "PATH",
        "VARIANT",
        "MAJ:MIN",
        "SSIZE",
        "ALIGN",
        "SECTORS",
        "START",
        "TYPE",
        "UUID",
        "PARTUUID",
        "PARTLABEL"
    ]);

    let prober = BlockProber::new().unwrap();
    for probed_res in prober.into_iter().filter_map(Result::transpose) {
        match probed_res {
            Ok(probed) => match probed.probe() {
                Ok(info) => {
                    table.add_row(Row::new(vec![
                        Cell::new(info.device),
                        Cell::new(info.path.to_string_lossy().as_ref()),
                        Cell::new(format!("{:?}", info.variant).as_str()),
                        Cell::new(format!("{}:{}", info.devno_major, info.devno_minor).as_str()),
                        Cell::new(info.logical_sector_size.to_string().as_str()),
                        Cell::new(info.alignment.to_string().as_str()),
                        Cell::new(info.sectors.to_string().as_str()),
                        Cell::new("0"),
                        Cell::new(info.fstype.as_ref().map_or("", AsRef::as_ref)),
                        Cell::new(info.uuid.as_ref().map_or("", AsRef::as_ref)),
                    ]));

                    for pinfo in &info.partitions {
                        table.add_row(Row::new(vec![
                            Cell::new(["├─", pinfo.device.as_ref()].concat().as_str()),
                            Cell::new(pinfo.path.to_string_lossy().as_ref()),
                            Cell::new(format!("Partition of {}", info.device).as_str()),
                            Cell::new(format!("{}:{}", info.devno_major, pinfo.no).as_str()),
                            Cell::new(info.logical_sector_size.to_string().as_str()),
                            Cell::new(info.alignment.to_string().as_str()),
                            Cell::new(pinfo.sectors.to_string().as_str()),
                            Cell::new(pinfo.offset.to_string().as_str()),
                            Cell::new(pinfo.fstype.as_ref().map_or("", AsRef::as_ref)),
                            Cell::new(pinfo.uuid.as_ref().map_or("", AsRef::as_ref)),
                            Cell::new(pinfo.partuuid.as_ref().map_or("", AsRef::as_ref)),
                            Cell::new(pinfo.partlabel.as_ref().map_or("", AsRef::as_ref)),
                        ]));
                    }
                }
                Err(why) => eprintln!("{:?}", why),
            },
            Err(why) => eprintln!("{:?}", why),
        }
    }

    table.printstd();
}

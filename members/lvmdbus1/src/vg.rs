use crate::{Error, LvmConn, LvmPath, Nodes};
use dbus::{
    arg::Dict,
    stdintf::org_freedesktop_dbus::{Introspectable, Properties},
    BusType, ConnPath, Connection, MessageItem, MessageItemArray, Signature,
};
use std::collections::HashMap;

pub struct VgConn {
    conn: Connection,
}

impl VgConn {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = VgPath<'a>> {
        let path = self.conn().with_path("com.redhat.lvmdbus1", Self::OBJECT, 1000);

        path.introspect()
            .map_err(|why| {
                eprintln!("{:?}", why);
                why
            })
            .ok()
            .into_iter()
            .map(|xml| serde_xml_rs::from_str::<Nodes>(xml.as_str()).unwrap())
            .flat_map(|nodes| nodes.nodes)
            .filter_map(|node| node.name.parse::<u32>().ok())
            .map(move |id| self.connect(id))
    }
}

impl<'a> LvmConn<'a> for VgConn {
    type Item = VgPath<'a>;

    const DEST: &'static str = "com.redhat.lvmdbus1";
    const OBJECT: &'static str = "/com/redhat/lvmdbus1/Vg";

    fn conn(&self) -> &Connection { &self.conn }
}

pub struct VgPath<'a> {
    conn:     ConnPath<'a, &'a Connection>,
    pub node: u32,
}

impl<'a> VgPath<'a> {
    pub fn activate(&self, control_flags: u64, options: HashMap<&str, &str>) -> Result<(), Error> {
        self.method("Activate", options, |m, tmo, options| {
            m.append_items(&[control_flags.into(), tmo.into(), options]);
        })
    }

    // TODO: fn allocation_policy_set

    pub fn change(&self, options: HashMap<&str, &str>) -> Result<(), Error> {
        self.method("Change", options, |m, tmo, options| {
            m.append_items(&[tmo.into(), options]);
        })
    }

    // TODO: fn create_cache_pool
    // TODO: fn create_thin_pool

    pub fn deactivate(
        &self,
        control_flags: u64,
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        self.method("Deactivate", options, |m, tmo, options| {
            m.append_items(&[control_flags.into(), tmo.into(), options]);
        })
    }

    pub fn extend(
        &self,
        pvs: &[dbus::Path<'static>],
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        self.method("Extend", options, |m, tmo, options| {
            m.append_items(&[pvs.into(), tmo.into(), options]);
        })
    }

    pub fn extent_count(&self) -> Result<u64, Error> { self.get("ExtentCount") }

    pub fn extent_size_bytes(&self) -> Result<u64, Error> { self.get("ExtentSizeBytes") }

    pub fn extent_free_count(&self) -> Result<u64, Error> { self.get("FreeCount") }

    pub fn lv_count(&self) -> Result<u64, Error> { self.get("LvCount") }

    pub fn lv_create(
        &self,
        name: &str,
        size_bytes: u64,
        pv_dests_and_ranges: impl IntoIterator<Item = (dbus::Path<'static>, u64, u64)>,
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        let dests_and_ranges = pv_dests_and_ranges_to_message_item(pv_dests_and_ranges);

        self.method("LvCreate", options, |m, tmo, options| {
            m.append_items(&[
                name.into(),
                size_bytes.into(),
                dests_and_ranges,
                tmo.into(),
                options,
            ]);
        })
    }

    pub fn lv_create_inner(
        &self,
        name: &str,
        size_bytes: u64,
        thin_pool: bool,
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        self.method("LvCreateInner", options, |m, tmo, options| {
            m.append_items(&[
                name.into(),
                size_bytes.into(),
                thin_pool.into(),
                tmo.into(),
                options,
            ]);
        })
    }

    // TODO: fn lv_create_mirror
    // TODO: fn lv_create_raid

    pub fn lvs(&self) -> impl Iterator<Item = dbus::Path> {
        self.conn
            .get::<Vec<dbus::Path>>(Self::PATH, "Lvs")
            .ok()
            .into_iter()
            .flat_map(|paths| paths.into_iter())
    }

    // TODO: fn max_lv_set
    // TODO: fn max_pv_set

    pub fn move_(
        &self,
        pv_source: dbus::Path<'static>,
        pv_source_range: (u64, u64),
        pv_dests_and_ranges: impl IntoIterator<Item = (dbus::Path<'static>, u64, u64)>,
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        let pv_source_range =
            MessageItem::Struct(vec![pv_source_range.0.into(), pv_source_range.1.into()]);
        let dests_and_ranges = pv_dests_and_ranges_to_message_item(pv_dests_and_ranges);

        self.method("Move", options, |m, tmo, options| {
            m.append_items(&[pv_source.into(), pv_source_range.into(), dests_and_ranges, options]);
        })
    }

    pub fn pv_count(&self) -> Result<u64, Error> { self.get("PvCount") }

    // TODO: fn pv_tags_add
    // TODO: fn pv_tags_delete

    pub fn pvs(&self) -> impl Iterator<Item = dbus::Path> {
        self.conn
            .get::<Vec<dbus::Path>>(Self::PATH, "Pvs")
            .ok()
            .into_iter()
            .flat_map(|paths| paths.into_iter())
    }

    pub fn reduce(
        &self,
        missing: bool,
        pvs: &[dbus::Path<'static>],
        options: HashMap<&str, &str>,
    ) -> Result<(), Error> {
        self.method("Reduce", options, |m, tmo, options| {
            m.append_items(&[missing.into(), pvs.into(), tmo.into(), options]);
        })
    }

    pub fn remove(&self, options: HashMap<&str, &str>) -> Result<(), Error> {
        self.method("Remove", options, |m, tmo, options| {
            m.append_items(&[tmo.into(), options]);
        })
    }

    pub fn rename(&self, name: &str, options: HashMap<&str, &str>) -> Result<(), Error> {
        self.method("Rename", options, |m, tmo, options| {
            m.append_items(&[name.into(), tmo.into(), options]);
        })
    }

    // TODO: fn tags_add
    // TODO: fn tags_delete
    // TODO: fn uuid_generate

    fn method<F: FnOnce(&mut dbus::Message, i32, MessageItem)>(
        &self,
        method: &'static str,
        options: HashMap<&str, &str>,
        func: F,
    ) -> Result<(), Error> {
        let tmo = self.conn.timeout;
        let options = dict_to_message_item(options);

        self.call_method(method, |m| func(m, tmo, options))?;
        Ok(())
    }
}

impl<'a> LvmPath<'a> for VgPath<'a> {
    const PATH: &'static str = "com.redhat.lvmdbus1.Vg";

    fn conn<'b>(&'b self) -> &'b ConnPath<'a, &'a Connection> { &self.conn }

    fn id(&self) -> u32 { self.node }

    fn from_path(conn: ConnPath<'a, &'a Connection>, node: u32) -> Self { Self { conn, node } }
}

fn dict_to_message_item<'a>(options: impl IntoIterator<Item = (&'a str, &'a str)>) -> MessageItem {
    MessageItem::from_dict::<(), _>(
        options.into_iter().map(|(k, v)| Ok((k.to_owned(), MessageItem::from(v)))),
    )
    .unwrap()
}

fn pv_dests_and_ranges_to_message_item(
    pv_dests_and_ranges: impl IntoIterator<Item = (dbus::Path<'static>, u64, u64)>,
) -> MessageItem {
    MessageItem::Array(
        MessageItemArray::new(
            pv_dests_and_ranges
                .into_iter()
                .map(|(p, s, e)| MessageItem::Struct(vec![p.into(), s.into(), e.into()]))
                .collect(),
            Signature::new("a(ott)").unwrap(),
        )
        .unwrap(),
    )
}

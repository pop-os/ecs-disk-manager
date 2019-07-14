
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate log;

mod dbus_helper;
mod methods;

use self::dbus_helper::DbusFactory;

use dbus::{
    self,
    tree::{Factory, Signal},
    BusType, Connection, Message, NameFlag
};

use ecs_disk_manager::*;
use ecs_disk_manager_dbus::*;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Error)]
pub enum InitError {
    #[error(display = "failed to create a private DBus connection")]
    PrivateConnection(#[error(cause)] dbus::Error),
    #[error(display = "failed to register dbus name")]
    RegisterName(#[error(cause)] dbus::Error),
    #[error(display = "failed to register object paths in dbus tree")]
    TreeRegister(#[error(cause)] dbus::Error),
}

pub struct Daemon {
    pub connection: Arc<Connection>,
    pub manager: DiskManager,
}

impl Daemon {
    pub fn new() -> Result<Self, InitError> {
        let connection = Arc::new(
            Connection::get_private(BusType::System)
                .map_err(InitError::PrivateConnection)?
        );

        connection
            .register_name(DBUS_NAME, NameFlag::ReplaceExisting as u32)
            .map_err(InitError::RegisterName)?;

        Ok(Daemon {
            connection,
            manager: DiskManager::default(),
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to init daemon")]
    Init(#[error(cause)] InitError),
}

fn main() {
    println!("{:?}", init());
}

fn init() -> Result<(), Error> {
    let factory = Factory::new_fn::<()>();
    let dbus_factory = DbusFactory::new(&factory);
    let daemon = Daemon::new().map_err(Error::Init)?;
    let mut daemon = Rc::new(RefCell::new(daemon));

    let root_interface = factory
        .interface(crate::DBUS_IFACE, ())
        .add_m(crate::methods::scan(daemon.clone(), &dbus_factory));

    let tree = factory.tree(()).add(
        factory
            .object_path(DBUS_PATH, ())
            .introspectable()
            .add(root_interface)
    );

    let connection = (*daemon.borrow()).connection.clone();
    tree.set_registered(&connection, true).unwrap();
    connection.add_handler(tree);

    loop {
        connection.incoming(1000).next();
    }
}

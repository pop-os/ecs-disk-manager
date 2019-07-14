use crate::{Daemon, dbus_helper::DbusFactory};
use dbus::{self, tree::{MTFn, Method}};
use ecs_disk_manager_dbus::methods;
use std::{cell::RefCell, rc::Rc};

pub fn scan(daemon: Rc<RefCell<Daemon>>, dbus_factory: &DbusFactory) -> Method<MTFn<()>, ()> {
    let method = dbus_factory.method(methods::SCAN, move |message| {
        let mut daemon = daemon.borrow_mut();
        daemon.manager.scan().map(|_| vec![])
    });

    method.consume()
}

pub fn entities(daemon: Rc<RefCell<Daemon>>, dbus_factory: &DbusFactory) -> Method<MTFn<()>, ()> {
    let method = dbus_factory.method(methods::SCAN, move |message| {
        let mut daemon = daemon.borrow_mut();
        daemon.manager
            .entities
            .iter()
            .collect::<Vec<(Entity, Flags)>>();
        Ok(())
    });

    method.consume()
}

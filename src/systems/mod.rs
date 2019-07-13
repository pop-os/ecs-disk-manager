pub mod create;
pub mod format;
pub mod label;
pub mod remove;
pub mod resize;
pub mod scan;

mod common;

pub(crate) use self::common::*;

pub use self::scan::scan;

use crate::{DiskManager, ManagerFlags};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "operations cancelled")]
    Cancelled,
    #[error(display = "failure in create system")]
    Create(#[error(cause)] create::Error),
    #[error(display = "failure in create system")]
    Format(#[error(cause)] format::Error),
    #[error(display = "failure in create system")]
    Label(#[error(cause)] label::Error),
    #[error(display = "failure in remove system")]
    Remove(#[error(cause)] remove::Error),
    // #[error(display = "failure in resize system")]
    // Resize(#[error(cause)] resize::Error),
}

impl From<create::Error> for Error {
    fn from(error: create::Error) -> Self {
        Error::Create(error)
    }
}

impl From<format::Error> for Error {
    fn from(error: format::Error) -> Self {
        Error::Format(error)
    }
}

impl From<label::Error> for Error {
    fn from(error: label::Error) -> Self {
        Error::Label(error)
    }
}

impl From<remove::Error> for Error {
    fn from(error: remove::Error) -> Self {
        Error::Remove(error)
    }
}

macro_rules! cancellation_check {
    ($cancel:ident) => {
        if $cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
    };
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    if world.flags.contains(ManagerFlags::REMOVE) {
        remove::run(world, cancel)?;
    }

    if world.flags.contains(ManagerFlags::RESIZE) {
        // cancellation_check!(cancel);
        // resize::run(world, cancel)?;
    }

    if world.flags.contains(ManagerFlags::CREATE) {
        cancellation_check!(cancel);
        create::run(world, cancel)?;
    }

    if world.flags.contains(ManagerFlags::FORMAT) {
        cancellation_check!(cancel);
        format::run(world, cancel)?;
    }

    if world.flags.contains(ManagerFlags::LABEL) {
        cancellation_check!(cancel);
        label::run(world, cancel)?;
    }

    Ok(())
}


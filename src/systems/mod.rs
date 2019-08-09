pub mod create;
pub mod modification;
pub mod remove;
pub mod resize;
pub mod scan;

mod common;

pub(crate) use self::common::*;

pub use self::scan::scan;

use self::{
    create::CreationSystem, modification::ModificationSystem, remove::RemoveSystem,
    resize::ResizeSystem,
};
use crate::{DiskComponents, DiskEntities, ManagerFlags};
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
    #[error(display = "failure in modification system")]
    Modification(#[error(cause)] modification::Error),
    #[error(display = "failure in remove system")]
    Remove(#[error(cause)] remove::Error),
    #[error(display = "failure in resize system")]
    Resize(#[error(cause)] resize::Error),
}

impl From<create::Error> for Error {
    fn from(error: create::Error) -> Self { Error::Create(error) }
}

impl From<modification::Error> for Error {
    fn from(error: modification::Error) -> Self { Error::Modification(error) }
}

impl From<remove::Error> for Error {
    fn from(error: remove::Error) -> Self { Error::Remove(error) }
}

impl From<resize::Error> for Error {
    fn from(error: resize::Error) -> Self { Error::Resize(error) }
}

macro_rules! cancellation_check {
    ($cancel:ident) => {
        if $cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
    };
}

#[derive(Debug, Default)]
pub(crate) struct DiskSystems {
    pub creation:     CreationSystem,
    pub modification: ModificationSystem,
    pub remove:       RemoveSystem,
    pub resize:       ResizeSystem,
}

pub(crate) fn run(
    entities: &mut DiskEntities,
    components: &mut DiskComponents,
    systems: &mut DiskSystems,
    flags: &ManagerFlags,
    cancel: &Arc<AtomicBool>,
) -> Result<(), Error> {
    if flags.contains(ManagerFlags::REMOVE) {
        systems.remove.run(entities, components, cancel)?;
    }

    if flags.contains(ManagerFlags::RESIZE) {
        cancellation_check!(cancel);
        systems.resize.run(entities, components, cancel)?
    }

    if flags.contains(ManagerFlags::CREATE) {
        cancellation_check!(cancel);
        systems.creation.run(entities, components, cancel)?
    }

    if flags.contains(ManagerFlags::FORMAT | ManagerFlags::LABEL) {
        cancellation_check!(cancel);
        systems.modification.run(entities, components, cancel)?;
    }

    Ok(())
}

pub trait System {
    type Err;

    /// Executes a disk system on the disk world.
    fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Self::Err>;
}

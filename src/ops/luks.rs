//! Commands for creating, activating, and deactivating LUKS devices.

// TODO: Use the cryptsetup bindings instead of the cryptsetup binary.

use crate::{DeviceEntity, DiskManager};
use disk_types::LuksPassphrase;
use secstr::SecStr;
use std::{
    io::{self, Write},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

impl DiskManager {
    /// Clears all remembered LUKS encryption passphrases.
    pub fn forget_encryption_keys(&mut self) {
        for key in self.components.devices.luks.values_mut() {
            *key = None;
        }
    }
}

#[derive(Debug, Error)]
#[error(display = "failed to execute cryptsetup command")]
pub struct Error(#[error(cause)] CommandError);

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(display = "command exited with failure status: {}", 0)]
    ExitStatus(ExitStatus),
    #[error(display = "failed to write to the stdin of the child process")]
    StdinWrite(#[error(cause)] io::Error),
    #[error(display = "failed to spawn child process")]
    Spawn(#[error(cause)] io::Error),
    #[error(display = "failed to wait on child process")]
    Wait(#[error(cause)] io::Error),
}

#[derive(Debug)]
pub struct LuksParams {
    pub key_size:    u16,
    pub kind:        Box<str>,
    pub target_name: Box<str>,
    pub passphrase:  Option<LuksPassphrase>,
}

pub fn format(device: &Path, luks_params: &LuksParams) -> Result<(), Error> {
    let key_size = format!("{}", luks_params.key_size);
    exec(
        Command::new("cryptsetup")
            .args(&["-s", &key_size, "luksFormat", "--type", &luks_params.kind])
            .arg(device),
        luks_params.passphrase.as_ref(),
    )
    .map_err(Error)
}

pub fn activate(
    device: &Path,
    device_map: &str,
    passphrase: Option<&LuksPassphrase>,
) -> Result<(), Error> {
    exec(Command::new("cryptsetup").arg("open").arg(device).arg(device_map), passphrase)
        .map_err(Error)
}

pub fn deactivate(device: &Path) -> Result<(), Error> {
    exec(Command::new("cryptsetup").arg("close").arg(device), None).map_err(Error)
}

fn exec(cmd: &mut Command, passphrase: Option<&LuksPassphrase>) -> Result<(), CommandError> {
    let mut child = cmd
        .stdin(if passphrase.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::null())
        .spawn()
        .map_err(CommandError::Spawn)?;

    if let Some(passphrase) = passphrase {
        let appended = append_newline(passphrase.as_ref());
        child
            .stdin
            .as_mut()
            .expect("stdin not obtained")
            .write_all(appended.unsecure())
            .map_err(CommandError::StdinWrite)?;
    }

    let status = child.wait().map_err(CommandError::Wait)?;

    if status.success() {
        Ok(())
    } else {
        Err(CommandError::ExitStatus(status))
    }
}

fn append_newline(input: &SecStr) -> SecStr {
    SecStr::new({
        let unsecured = input.unsecure();
        let mut updated = Vec::with_capacity(unsecured.len() + 1);
        updated.extend_from_slice(unsecured);
        updated.push(b'\n');
        updated
    })
}

//! Errors of talking to a device, with messages in the interface language.

use std::io;
use std::path::PathBuf;

use crate::flydigi::profile::ProfileError;

/// Why a device could not be opened or did not do what it was asked.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The hidraw node exists but this user may not open it, usually the udev rule is missing.
    #[error("{}", t!("No access to {}: {}. Install the udev rule from the README.", .path.display(), .source))]
    Access { path: PathBuf, source: io::Error },
    /// The node opened but nothing answered the first request: a receiver without a pad, or a hung device.
    #[error("{}", t!("{} does not answer, reconnect the gamepad", .0.display()))]
    Silent(PathBuf),
    /// A request went unanswered after its retries.
    #[error("{}", t!("The gamepad did not answer command 0x{}", format!("{:02x}", .0)))]
    NoReply(u8),
    /// The device was unplugged while being talked to.
    #[error("{}", t!("Gamepad disconnected"))]
    Disconnected,
    /// The profile could not be built or read.
    #[error(transparent)]
    Profile(#[from] ProfileError),
}

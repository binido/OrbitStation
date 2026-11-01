//! Static descriptions of device models. A model of a known protocol family is data, not code.

/// What kind of hardware a model is, it picks the sidebar icon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(dead_code, reason = "docks and receivers come with their first model description")]
pub enum Kind {
    Gamepad,
    Dock,
    Receiver,
}

/// Parts of a profile the model has, each turns on its editor page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Features {
    pub remap: bool,
    pub macros: bool,
    pub sticks: bool,
    pub triggers: bool,
    pub gyro: bool,
    pub vibration: bool,
    pub lighting: bool,
}

/// Profile blobs a model holds out of the box, read off a real device.
#[derive(Debug)]
pub struct Factory {
    pub map: &'static [u8],
    pub macros: &'static [u8],
    pub rgb: &'static [u8],
    /// Slot title before its number, `配置` gives `配置1`..`配置4` as Space Station writes them.
    pub title: &'static str,
}

/// Everything the app knows about one model.
#[derive(Debug)]
pub struct Model {
    /// Shown in the sidebar and on the device page, translated when it is not a product name.
    pub name: &'static str,
    /// Short ASCII name for file names of backups.
    pub slug: &'static str,
    pub kind: Kind,
    /// USB vendor and product ids the model shows up with.
    pub usb: &'static [(u16, u16)],
    /// HID interface number that carries the configuration protocol.
    pub config_interface: u8,
    /// Ids the device reports about itself, they tell models apart when the USB ids are shared.
    pub device_ids: &'static [u8],
    /// Profile slots on the device, 0 when it keeps none.
    pub profile_slots: u8,
    pub features: Features,
    /// Remappable buttons as (key id, name).
    pub keys: &'static [(u8, &'static str)],
    /// What a remapped button may send as (key id, name), the first entry means "unchanged".
    pub targets: &'static [(u8, &'static str)],
    /// Button names in bit order of the live input report, empty names are unused bits.
    pub buttons: &'static [&'static str],
    pub factory: Option<Factory>,
}

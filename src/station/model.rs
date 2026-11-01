//! Static descriptions of device models. A model of a known protocol family is data, not code.

/// What kind of hardware a model is, it picks the sidebar icon and the page wording.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Gamepad,
    Dock,
}

/// Parts of a profile the model has, each turns on its editor or a part of one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Features {
    pub remap: bool,
    pub macros: bool,
    pub sticks: bool,
    pub triggers: bool,
    pub gyro: bool,
    /// Grip motors.
    pub vibration: bool,
    /// A motor in each trigger, the Vader line has them and the Apex line has force triggers instead.
    pub trigger_motors: bool,
    pub lighting: bool,
}

impl Features {
    /// A model whose profile the app does not edit.
    pub const NONE: Features = Features {
        remap: false,
        macros: false,
        sticks: false,
        triggers: false,
        gyro: false,
        vibration: false,
        trigger_motors: false,
        lighting: false,
    };
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
    /// Ids the device reports about itself (a pad's device type, a dock's charger type); one model
    /// owns several, one per edition.
    pub device_ids: &'static [u8],
    /// Profile slots on the device, 0 when it keeps none.
    pub profile_slots: u8,
    /// Macros live in a store of their own (protocol 3.2), not inside the mapping blob.
    pub macro_store: bool,
    /// The device keeps toggles and choices of its own outside the profiles.
    pub settings: bool,
    /// The device streams its sticks, buttons and sensors for the test page.
    pub live_input: bool,
    pub features: Features,
    /// Remappable buttons as (key id, name).
    pub keys: &'static [(u8, &'static str)],
    /// What a remapped button may send as (key id, name), the first entry means "unchanged".
    pub targets: &'static [(u8, &'static str)],
    /// Button names in bit order of the live input report, empty names are unused bits.
    pub buttons: &'static [&'static str],
    pub factory: Option<Factory>,
}

impl Model {
    /// A model the app only names and reads the info of; the base for the others.
    pub const INFO_ONLY: Model = Model {
        name: "",
        slug: "",
        kind: Kind::Gamepad,
        device_ids: &[],
        profile_slots: 0,
        macro_store: false,
        settings: false,
        live_input: false,
        features: Features::NONE,
        keys: &[],
        targets: &[],
        buttons: &[],
        factory: None,
    };
}

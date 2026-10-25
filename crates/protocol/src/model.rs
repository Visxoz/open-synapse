use crate::flags::flags;

pub const VENDOR_ID: u16 = 0x1532;

/// What a mouse supports and how it expects to be spoken to.
#[derive(Debug)]
pub struct Model {
    pub pid: u16,
    pub name: &'static str,
    pub link: Link,
    /// The transaction ID the device answers to for most commands.
    pub tid: u8,
    pub wait: Wait,
    pub dpi: DpiRange,
    pub poll: Poll,
    pub power: Power,
    pub features: Features,
    pub lighting: Lighting,
}

impl Model {
    pub fn find(pid: u16) -> Option<&'static Model> {
        crate::models::MODELS.iter().find(|m| m.pid == pid)
    }

    pub fn all() -> &'static [Model] {
        crate::models::MODELS
    }

    /// Where Razer publishes a picture of the model from above.
    pub fn image_url(&self) -> Option<&'static str> {
        crate::images::IMAGES
            .iter()
            .find(|(pid, _)| *pid == self.pid)
            .map(|(_, url)| *url)
    }

    pub fn has_battery(&self) -> bool {
        self.power.contains(Power::BATTERY)
    }

    /// The transaction ID for a command whose expected one differs from
    /// `tid`; a read is matched by its write's ID.
    pub fn tid_for(&self, class: u8, id: u8) -> Option<u8> {
        let tids = crate::tids::TIDS;
        let (_, rules) = tids[tids.binary_search_by_key(&self.pid, |(pid, _)| *pid).ok()?];
        rules
            .iter()
            .find(|r| r.class == class && r.id == id & 0x7F)
            .map(|r| r.tid)
    }
}

/// A command a model expects under its own transaction ID.
#[derive(Debug, Clone, Copy)]
pub struct TidFor {
    pub class: u8,
    /// The write's command ID; the read's adds 0x80.
    pub id: u8,
    pub tid: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Wired,
    Wireless,
    Bluetooth,
}

/// How long the device takes to prepare a response; wireless receivers
/// relay each command to the mouse and back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    Quick,
    Receiver,
    Viper,
    Slow,
}

impl Wait {
    pub const fn micros(self) -> u64 {
        match self {
            Self::Quick => 600,
            Self::Receiver => 31_000,
            Self::Viper => 59_900,
            Self::Slow => 400_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpiRange {
    None,
    /// DPI as 16-bit values.
    Word {
        max: u16,
    },
    /// DPI scaled into a byte, on older mice.
    Byte {
        max: u16,
    },
}

impl DpiRange {
    pub const MIN: u16 = 100;

    pub const fn max(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Word { max } | Self::Byte { max } => max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    None,
    /// Up to 1000 Hz.
    Classic(&'static [u16]),
    /// HyperPolling, up to 8000 Hz.
    Hyper(&'static [u16]),
}

impl Poll {
    pub const fn rates(self) -> &'static [u16] {
        match self {
            Self::None => &[],
            Self::Classic(r) | Self::Hyper(r) => r,
        }
    }
}

flags! {
    Power: u8 { BATTERY = 0, CHARGING = 1, IDLE = 2, LOW_BATTERY = 3 }
}

flags! {
    Features: u16 {
        DPI_STAGES = 0,
        SCROLL_MODE = 1,
        SCROLL_ACCELERATION = 2,
        SMART_REEL = 3,
        /// One indicator LED on the HyperPolling dongle.
        DONGLE_LED = 4,
        /// Three indicator LEDs on the HyperPolling dongle.
        DONGLE_LEDS = 5,
        DPI_READ_VARSTORE = 6,
    }
}

flags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    Effects: u16 {
        STATIC = 0,
        BREATH_SINGLE = 1,
        BREATH_DUAL = 2,
        BREATH_RANDOM = 3,
        SPECTRUM = 4,
        WAVE = 5,
        REACTIVE = 6,
        BLINKING = 7,
        ON = 8,
        OFF = 9,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Led {
    /// The whole device, on mice whose lighting is one matrix.
    Matrix = 0x00,
    Scroll = 0x01,
    Logo = 0x04,
    Backlight = 0x05,
    Right = 0x10,
    Left = 0x11,
}

impl Led {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Matrix => "All",
            Self::Scroll => "Scroll wheel",
            Self::Logo => "Logo",
            Self::Backlight => "Backlight",
            Self::Right => "Right side",
            Self::Left => "Left side",
        }
    }
}

/// The command families Razer's mice use for lighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightProto {
    /// Class 0x0F effects, per LED.
    Extended,
    /// Per-LED state, colour and effect commands of the oldest mice.
    Classic,
    /// Class 0x03 effects addressed per LED.
    MouseExtended,
    /// One class 0x03 effect for the whole device.
    StandardMatrix,
    NagaPro,
    NagaV2Pro,
    NagaTrinity,
    BasiliskMobile,
}

#[derive(Debug, Clone, Copy)]
pub struct Zone {
    pub led: Led,
    pub effects: Effects,
    pub brightness: bool,
    /// What the zone lights, where the LED's own name would mislead.
    pub name: Option<&'static str>,
}

impl Zone {
    pub const fn new(led: Led, effects: Effects, brightness: bool) -> Self {
        Self {
            led,
            effects,
            brightness,
            name: None,
        }
    }

    pub const fn named(self, name: &'static str) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    pub fn label(&self) -> &'static str {
        self.name.unwrap_or(self.led.label())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Lighting {
    pub proto: LightProto,
    pub zones: &'static [Zone],
    /// The LEDs take any colour; otherwise they shine in one fixed colour.
    pub rgb: bool,
}

impl Lighting {
    pub const NONE: Self = Self {
        proto: LightProto::Extended,
        zones: &[],
        rgb: false,
    };

    pub fn zone(&self, led: Led) -> Option<&Zone> {
        self.zones.iter().find(|z| z.led == led)
    }
}

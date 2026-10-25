#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::model::Features;

pub const MAX_STAGES: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Dpi {
    pub x: u16,
    pub y: u16,
}

impl Dpi {
    pub const fn square(v: u16) -> Self {
        Self { x: v, y: v }
    }
}

/// The DPI steps the mouse's DPI button cycles through.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DpiStages {
    /// Index into `stages`.
    pub active: usize,
    pub stages: Vec<Dpi>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ScrollSetting {
    /// Free-spinning rather than tactile.
    FreeSpin,
    Acceleration,
    SmartReel,
}

impl ScrollSetting {
    pub const ALL: [Self; 3] = [Self::FreeSpin, Self::Acceleration, Self::SmartReel];

    /// The feature a model needs for this setting.
    pub const fn feature(self) -> Features {
        match self {
            Self::FreeSpin => Features::SCROLL_MODE,
            Self::Acceleration => Features::SCROLL_ACCELERATION,
            Self::SmartReel => Features::SMART_REEL,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::FreeSpin => "Free-spinning wheel",
            Self::Acceleration => "Scroll acceleration",
            Self::SmartReel => "Smart reel",
        }
    }

    pub(crate) const fn command(self) -> u8 {
        match self {
            Self::FreeSpin => 0x14,
            Self::Acceleration => 0x16,
            Self::SmartReel => 0x17,
        }
    }
}

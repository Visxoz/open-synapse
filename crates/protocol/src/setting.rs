#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::commands::{self, Batch};
use crate::lighting::{self, Effect};
use crate::model::{Led, Model};
use crate::types::{Dpi, DpiStages, ScrollSetting};

/// One value written to a mouse.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Setting {
    Dpi(Dpi),
    Stages(DpiStages),
    Poll(u16),
    Idle(u16),
    LowBattery(u8),
    Effect(Led, Effect),
    Brightness(Led, u8),
    DongleLed(u8),
    DongleLeds([u8; 3]),
    Scroll(ScrollSetting, bool),
}

/// Which value a setting writes, so a newer one can replace an older one
/// that has not been sent yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Dpi,
    Stages,
    Poll,
    Idle,
    LowBattery,
    Effect(Led),
    Brightness(Led),
    DongleLed,
    DongleLeds,
    Scroll(ScrollSetting),
}

impl Setting {
    pub fn batch(&self, m: &Model) -> Batch {
        match self {
            Self::Dpi(dpi) => commands::set_dpi(m, *dpi),
            Self::Stages(stages) => commands::set_stages(m, stages),
            Self::Poll(rate) => commands::set_poll(m, *rate),
            Self::Idle(seconds) => commands::set_idle(*seconds),
            Self::LowBattery(percent) => commands::set_low_battery(*percent),
            Self::Effect(led, effect) => lighting::set_effect(m, *led, effect),
            Self::Brightness(led, level) => lighting::set_brightness(m, *led, *level),
            Self::DongleLed(mode) => commands::set_dongle_led(*mode),
            Self::DongleLeds(modes) => commands::set_dongle_leds(*modes),
            Self::Scroll(setting, on) => commands::set_scroll(*setting, *on),
        }
    }

    pub const fn slot(&self) -> Slot {
        match self {
            Self::Dpi(_) => Slot::Dpi,
            Self::Stages(_) => Slot::Stages,
            Self::Poll(_) => Slot::Poll,
            Self::Idle(_) => Slot::Idle,
            Self::LowBattery(_) => Slot::LowBattery,
            Self::Effect(led, _) => Slot::Effect(*led),
            Self::Brightness(led, _) => Slot::Brightness(*led),
            Self::DongleLed(_) => Slot::DongleLed,
            Self::DongleLeds(_) => Slot::DongleLeds,
            Self::Scroll(s, _) => Slot::Scroll(*s),
        }
    }
}

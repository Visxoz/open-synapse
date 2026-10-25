//! Razer's mouse control protocol: 90-byte HID feature reports, the
//! commands built from them, and the table of mice that speak it.

#![forbid(unsafe_code)]

pub mod commands;
mod flags;
mod images;
pub mod lighting;
mod model;
mod models;
mod report;
mod setting;
mod types;

pub use commands::{Batch, Query};
pub use lighting::{Breath, Effect, EffectParams};
pub use model::*;
pub use report::{REPORT_LEN, Report, Status};
pub use setting::{Setting, Slot};
pub use types::*;

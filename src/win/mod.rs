//! Windows integration.

pub mod autostart;
pub mod hash;
pub mod http;
pub mod pointer;
pub mod programs;
pub mod single_instance;
pub mod window;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

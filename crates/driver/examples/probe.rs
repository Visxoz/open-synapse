//! Lists connected mice and everything they report: `cargo run --example probe`.

use std::time::Instant;

fn main() {
    for iface in razer_driver::candidates() {
        let start = Instant::now();
        let Some(device) = razer_driver::Device::open(&iface) else {
            continue;
        };
        let opened = start.elapsed();
        let state = device.read_state();
        println!(
            "{} (PID {:04X}) firmware {}.{:02} serial {:?}\n  {}\n  opened in {opened:?}, read in {:?}\n  {state:#?}",
            device.model.name,
            device.model.pid,
            device.firmware.0,
            device.firmware.1,
            device.serial,
            device.path,
            start.elapsed() - opened,
        );
    }
}

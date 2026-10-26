//! Lists connected mice and everything they report: `cargo run --example probe`.

use std::time::Instant;

fn main() {
    for iface in razer_driver::candidates() {
        let start = Instant::now();
        let device = match razer_driver::Device::open(&iface) {
            Ok(Some(device)) => device,
            Ok(None) => continue,
            Err(e) => {
                println!("{} (PID {:04X}) cannot be opened: {e}", iface.display_path(), iface.pid);
                continue;
            }
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

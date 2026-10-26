//! HID report descriptors, read just far enough to size feature reports,
//! which Linux and macOS leave to the caller.

/// The global items that size reports, kept per `Push`.
#[derive(Clone, Copy, Default)]
struct Globals {
    size: u32,
    count: u32,
    id: u8,
}

/// The longest feature report `descriptor` declares, in bytes with the
/// report ID, which is how Windows' `HidP_GetCaps` counts it; 0 if none.
pub fn feature_len(descriptor: &[u8]) -> usize {
    let mut globals = Globals::default();
    let mut pushed = Vec::new();
    // Bits per report ID.
    let mut bits = [0u32; 256];
    let mut rest = descriptor;
    while let Some((&prefix, tail)) = rest.split_first() {
        // A long item: its data length, its tag, then the data.
        if prefix == 0xFE {
            let len = tail.first().map_or(0, |&n| usize::from(n));
            rest = tail.get(len + 2..).unwrap_or_default();
            continue;
        }
        let len = [0, 1, 2, 4][usize::from(prefix & 3)];
        let Some(data) = tail.get(..len) else { break };
        rest = &tail[len..];
        let value = data.iter().rev().fold(0, |v, &b| v << 8 | u32::from(b));
        match prefix & 0xFC {
            0xB0 => {
                let id = &mut bits[usize::from(globals.id)];
                *id = id.saturating_add(globals.size.saturating_mul(globals.count));
            }
            0x74 => globals.size = value,
            0x84 => globals.id = value as u8,
            0x94 => globals.count = value,
            0xA4 => pushed.push(globals),
            0xB4 => globals = pushed.pop().unwrap_or(globals),
            _ => {}
        }
    }
    bits.iter()
        .max()
        .filter(|&&b| b > 0)
        .map_or(0, |&b| b.div_ceil(8) as usize + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_a_razer_control_report() {
        // A mouse's input report, then 90 bytes of vendor feature report.
        let descriptor = [
            0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29, 0x05, 0x15, 0x00,
            0x25, 0x01, 0x75, 0x01, 0x95, 0x05, 0x81, 0x02, 0x75, 0x03, 0x95, 0x01, 0x81, 0x01, 0xC0, 0xC0, 0x06, 0x00,
            0xFF, 0x09, 0x02, 0xA1, 0x01, 0x75, 0x08, 0x95, 0x5A, 0xB1, 0x01, 0xC0,
        ];
        assert_eq!(feature_len(&descriptor), 91);
    }

    #[test]
    fn counts_numbered_reports_apart() {
        let descriptor = [
            0x85, 0x01, 0x75, 0x08, 0x95, 0x10, 0xB1, 0x02, // 16 bytes as report 1
            0x85, 0x02, 0x95, 0x08, 0xB1, 0x02, 0xB1, 0x02, // 8 and 8 more as report 2
        ];
        assert_eq!(feature_len(&descriptor), 17);
    }

    #[test]
    fn restores_pushed_globals() {
        let descriptor = [
            0x75, 0x08, 0x95, 0x04, 0xA4, 0x95, 0x40, 0x81, 0x02, 0xB4, 0xB1, 0x02, // 4 bytes, not 64
        ];
        assert_eq!(feature_len(&descriptor), 5);
    }

    #[test]
    fn has_no_feature_reports_without_feature_items() {
        assert_eq!(feature_len(&[0x75, 0x08, 0x95, 0x08, 0x81, 0x02]), 0);
        assert_eq!(feature_len(&[0xB1]), 0, "truncated item");
        assert_eq!(feature_len(&[]), 0);
    }
}

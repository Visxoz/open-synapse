//! Commands a model expects under a transaction ID other than its usual
//! one, as OpenRazer's razermouse driver sends them (GPL-2.0-or-later,
//! commit 477f4d6). Where OpenRazer itself uses several IDs for one command,
//! none is listed and the device falls back to trying each.

use crate::model::TidFor;

/// Class, write command ID and transaction ID, per product ID.
pub(crate) static TIDS: &[(u16, &[TidFor])] = &[
    // Razer Naga
    (0x0015, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Naga Epic
    (
        0x001F,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Abyssus 1800
    (0x0020, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Mamba 2012 (Wired)
    (
        0x0024,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Mamba 2012 (Wireless)
    (
        0x0025,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Naga 2012
    (0x002E, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Imperator 2012
    (0x002F, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Ouroboros
    (0x0032, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Taipan
    (0x0034, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Naga Hex (Red)
    (0x0036, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer DeathAdder 2013
    (
        0x0037,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer DeathAdder 1800
    (0x0038, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Orochi 2013
    (0x0039, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Naga Epic Chroma (Wired)
    (
        0x003E,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Naga Epic Chroma (Wireless)
    (
        0x003F,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Naga Hex
    (0x0041, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Abyssus
    (0x0042, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer DeathAdder Chroma
    (
        0x0043,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Orochi (Wired)
    (0x0048, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer DeathAdder 2000
    (0x004F, &[TidFor::new(0x03, 0x00, 0x3F), TidFor::new(0x03, 0x02, 0x3F)]),
    // Razer Naga Chroma
    (0x0053, &[TidFor::new(0x03, 0x0D, 0x3F)]),
    // Razer DeathAdder 3500
    (
        0x0054,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Lancehead (Wired)
    (
        0x0059,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
        ],
    ),
    // Razer Lancehead (Wireless)
    (
        0x005A,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
        ],
    ),
    // Razer Abyssus V2
    (
        0x005B,
        &[
            TidFor::new(0x03, 0x00, 0x3F),
            TidFor::new(0x03, 0x01, 0x3F),
            TidFor::new(0x03, 0x02, 0x3F),
        ],
    ),
    // Razer Abyssus 2000
    (0x005E, &[TidFor::new(0x03, 0x00, 0x3F)]),
    // Razer Lancehead Tournament Edition
    (0x0060, &[TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer Atheris (Receiver)
    (
        0x0062,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
        ],
    ),
    // Razer Basilisk Essential
    (0x0065, &[TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer Naga Trinity
    (
        0x0067,
        &[
            TidFor::new(0x0F, 0x02, 0x1F),
            TidFor::new(0x0F, 0x03, 0x1F),
            TidFor::new(0x0F, 0x04, 0x3F),
        ],
    ),
    // Razer Abyssus Elite (D.Va Edition)
    (0x006A, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Abyssus Essential
    (0x006B, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Mamba Elite
    (0x006C, &[TidFor::new(0x00, 0x02, 0xFF), TidFor::new(0x00, 0x05, 0xFF)]),
    // Razer DeathAdder Essential
    (0x006E, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Lancehead Wireless (Receiver)
    (
        0x006F,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x00, 0x1F),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
            TidFor::new(0x07, 0x04, 0x1F),
        ],
    ),
    // Razer Lancehead Wireless (Wired)
    (
        0x0070,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x00, 0x1F),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
            TidFor::new(0x07, 0x04, 0x1F),
        ],
    ),
    // Razer DeathAdder Essential (White Edition)
    (0x0071, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Mamba Wireless (Receiver)
    (0x0072, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Mamba Wireless (Wired)
    (0x0073, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Pro Click (Receiver)
    (0x0077, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Viper
    (0x0078, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Viper Ultimate (Wired)
    (0x007A, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Viper Ultimate (Wireless)
    (0x007B, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer DeathAdder V2 Pro (Wired)
    (
        0x007C,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
        ],
    ),
    // Razer DeathAdder V2 Pro (Wireless)
    (
        0x007D,
        &[
            TidFor::new(0x04, 0x06, 0xFF),
            TidFor::new(0x07, 0x01, 0xFF),
            TidFor::new(0x07, 0x03, 0xFF),
        ],
    ),
    // Razer Pro Click (Wired)
    (0x0080, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Basilisk Ultimate
    (0x0086, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Basilisk Ultimate
    (0x0088, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Viper Mini
    (0x008A, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer DeathAdder V2 Mini
    (0x008C, &[TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer Naga Pro (Wired)
    (0x008F, &[TidFor::new(0x03, 0x0A, 0xFF)]),
    // Razer Viper 8KHz
    (
        0x0091,
        &[
            TidFor::new(0x00, 0x40, 0x1F),
            TidFor::new(0x0F, 0x02, 0x1F),
            TidFor::new(0x0F, 0x04, 0x1F),
        ],
    ),
    // Razer Orochi V2 (Receiver)
    (0x0094, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Orochi V2 (Bluetooth)
    (0x0095, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x03, 0xFF)]),
    // Razer Naga X
    (0x0096, &[TidFor::new(0x00, 0x02, 0x08), TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer DeathAdder Essential (2021)
    (0x0098, &[TidFor::new(0x0F, 0x02, 0x3F), TidFor::new(0x0F, 0x04, 0x3F)]),
    // Razer Basilisk V3
    (0x0099, &[TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer DeathAdder V2 X HyperSpeed
    (0x009C, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer DeathAdder V2 Lite
    (0x00A1, &[TidFor::new(0x04, 0x06, 0xFF)]),
    // Razer Cobra
    (0x00A3, &[TidFor::new(0x0F, 0x02, 0x1F), TidFor::new(0x0F, 0x04, 0x1F)]),
    // Razer Viper V2 Pro (Wired)
    (0x00A5, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Viper V2 Pro (Wireless)
    (0x00A6, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Basilisk V3 Pro (Wired)
    (0x00AA, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Basilisk V3 Pro (Wireless)
    (0x00AB, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Cobra Pro (Wired)
    (0x00AF, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Cobra Pro (Wireless)
    (0x00B0, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer HyperPolling Wireless Dongle
    (0x00B3, &[TidFor::new(0x07, 0x01, 0xFF), TidFor::new(0x07, 0x10, 0xFF)]),
    // Razer Viper V3 Pro (Wireless)
    (0x00C1, &[TidFor::new(0x07, 0x10, 0xFF)]),
    // Razer Basilisk V3 Pro 35K (Wired)
    (0x00CC, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Basilisk V3 Pro 35K (Wireless)
    (0x00CD, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Basilisk V3 Pro 35K Phantom Green Edition (Wired)
    (0x00D6, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Basilisk V3 Pro 35K Phantom Green Edition (Wireless)
    (0x00D7, &[TidFor::new(0x07, 0x01, 0xFF)]),
    // Razer Cobra HyperSpeed
    (0x00DA, &[TidFor::new(0x07, 0x01, 0xFF)]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;

    #[test]
    fn every_entry_is_a_known_model_and_differs_from_its_default() {
        for (pid, rules) in TIDS {
            let m = Model::find(*pid).expect("known model");
            assert!(rules.iter().all(|r| r.tid != m.tid && r.id & 0x80 == 0), "{}", m.name);
        }
    }

    #[test]
    fn reads_share_their_write_s_id() {
        let viper = Model::find(0x0078).unwrap();
        assert_eq!(viper.tid_for(0x0F, 0x02), Some(0x3F));
        assert_eq!(viper.tid_for(0x0F, 0x84), Some(0x3F));
        assert_eq!(viper.tid_for(0x00, 0x81), None);
    }
}

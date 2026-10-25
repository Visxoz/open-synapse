//! Commands a model expects under a transaction ID other than its usual
//! one, as OpenRazer's razermouse driver sends them (GPL-2.0-or-later,
//! commit 477f4d6). Where OpenRazer itself uses several IDs for one command,
//! none is listed and the device falls back to trying each.

use crate::model::TidFor;

const fn t(class: u8, id: u8, tid: u8) -> TidFor {
    TidFor { class, id, tid }
}

// Rule sets several models share, named for the commands they cover.
const LED_STATE: &[TidFor] = &[t(0x03, 0x00, 0x3F)];
const CLASSIC: &[TidFor] = &[t(0x03, 0x00, 0x3F), t(0x03, 0x01, 0x3F), t(0x03, 0x02, 0x3F)];
const EXTENDED: &[TidFor] = &[t(0x0F, 0x02, 0x3F), t(0x0F, 0x04, 0x3F)];
const STAGES: &[TidFor] = &[t(0x04, 0x06, 0xFF)];
const STAGES_POWER: &[TidFor] = &[t(0x04, 0x06, 0xFF), t(0x07, 0x01, 0xFF), t(0x07, 0x03, 0xFF)];
const POWER: &[TidFor] = &[t(0x07, 0x01, 0xFF), t(0x07, 0x03, 0xFF)];
const LOW_BATTERY: &[TidFor] = &[t(0x07, 0x01, 0xFF)];

/// Sorted by product ID, for `Model::tid_for`'s binary search.
pub(crate) static TIDS: &[(u16, &[TidFor])] = &[
    (0x0015, LED_STATE),                                                        // Razer Naga
    (0x001F, CLASSIC),                                                          // Razer Naga Epic
    (0x0020, LED_STATE),                                                        // Razer Abyssus 1800
    (0x0024, CLASSIC),                                                          // Razer Mamba 2012 (Wired)
    (0x0025, CLASSIC),                                                          // Razer Mamba 2012 (Wireless)
    (0x002E, LED_STATE),                                                        // Razer Naga 2012
    (0x002F, LED_STATE),                                                        // Razer Imperator 2012
    (0x0032, LED_STATE),                                                        // Razer Ouroboros
    (0x0034, LED_STATE),                                                        // Razer Taipan
    (0x0036, LED_STATE),                                                        // Razer Naga Hex (Red)
    (0x0037, CLASSIC),                                                          // Razer DeathAdder 2013
    (0x0038, LED_STATE),                                                        // Razer DeathAdder 1800
    (0x0039, LED_STATE),                                                        // Razer Orochi 2013
    (0x003E, CLASSIC),                                                          // Razer Naga Epic Chroma (Wired)
    (0x003F, CLASSIC),                                                          // Razer Naga Epic Chroma (Wireless)
    (0x0041, LED_STATE),                                                        // Razer Naga Hex
    (0x0042, LED_STATE),                                                        // Razer Abyssus
    (0x0043, CLASSIC),                                                          // Razer DeathAdder Chroma
    (0x0048, LED_STATE),                                                        // Razer Orochi (Wired)
    (0x004F, &[t(0x03, 0x00, 0x3F), t(0x03, 0x02, 0x3F)]),                      // Razer DeathAdder 2000
    (0x0053, &[t(0x03, 0x0D, 0x3F)]),                                           // Razer Naga Chroma
    (0x0054, CLASSIC),                                                          // Razer DeathAdder 3500
    (0x0059, STAGES_POWER),                                                     // Razer Lancehead (Wired)
    (0x005A, STAGES_POWER),                                                     // Razer Lancehead (Wireless)
    (0x005B, CLASSIC),                                                          // Razer Abyssus V2
    (0x005E, LED_STATE),                                                        // Razer Abyssus 2000
    (0x0060, STAGES),                                                           // Razer Lancehead Tournament Edition
    (0x0062, STAGES_POWER),                                                     // Razer Atheris (Receiver)
    (0x0065, STAGES),                                                           // Razer Basilisk Essential
    (0x0067, &[t(0x0F, 0x02, 0x1F), t(0x0F, 0x03, 0x1F), t(0x0F, 0x04, 0x3F)]), // Razer Naga Trinity
    (0x006A, EXTENDED),                                                         // Razer Abyssus Elite (D.Va Edition)
    (0x006B, EXTENDED),                                                         // Razer Abyssus Essential
    (0x006C, &[t(0x00, 0x02, 0xFF), t(0x00, 0x05, 0xFF)]),                      // Razer Mamba Elite
    (0x006E, EXTENDED),                                                         // Razer DeathAdder Essential
    (
        0x006F,
        &[
            t(0x04, 0x06, 0xFF),
            t(0x07, 0x00, 0x1F),
            t(0x07, 0x01, 0xFF),
            t(0x07, 0x03, 0xFF),
            t(0x07, 0x04, 0x1F),
        ],
    ), // Razer Lancehead Wireless (Receiver)
    (
        0x0070,
        &[
            t(0x04, 0x06, 0xFF),
            t(0x07, 0x00, 0x1F),
            t(0x07, 0x01, 0xFF),
            t(0x07, 0x03, 0xFF),
            t(0x07, 0x04, 0x1F),
        ],
    ), // Razer Lancehead Wireless (Wired)
    (0x0071, EXTENDED),               // Razer DeathAdder Essential (White Edition)
    (0x0072, POWER),                  // Razer Mamba Wireless (Receiver)
    (0x0073, POWER),                  // Razer Mamba Wireless (Wired)
    (0x0077, POWER),                  // Razer Pro Click (Receiver)
    (0x0078, EXTENDED),               // Razer Viper
    (0x007A, EXTENDED),               // Razer Viper Ultimate (Wired)
    (0x007B, EXTENDED),               // Razer Viper Ultimate (Wireless)
    (0x007C, STAGES_POWER),           // Razer DeathAdder V2 Pro (Wired)
    (0x007D, STAGES_POWER),           // Razer DeathAdder V2 Pro (Wireless)
    (0x0080, POWER),                  // Razer Pro Click (Wired)
    (0x0086, POWER),                  // Razer Basilisk Ultimate
    (0x0088, POWER),                  // Razer Basilisk Ultimate
    (0x008A, EXTENDED),               // Razer Viper Mini
    (0x008C, STAGES),                 // Razer DeathAdder V2 Mini
    (0x008F, &[t(0x03, 0x0A, 0xFF)]), // Razer Naga Pro (Wired)
    (0x0091, &[t(0x00, 0x40, 0x1F), t(0x0F, 0x02, 0x1F), t(0x0F, 0x04, 0x1F)]), // Razer Viper 8KHz
    (0x0094, POWER),                  // Razer Orochi V2 (Receiver)
    (0x0095, POWER),                  // Razer Orochi V2 (Bluetooth)
    (0x0096, &[t(0x00, 0x02, 0x08), t(0x04, 0x06, 0xFF)]), // Razer Naga X
    (0x0098, EXTENDED),               // Razer DeathAdder Essential (2021)
    (0x0099, STAGES),                 // Razer Basilisk V3
    (0x009C, LOW_BATTERY),            // Razer DeathAdder V2 X HyperSpeed
    (0x00A1, STAGES),                 // Razer DeathAdder V2 Lite
    (0x00A3, &[t(0x0F, 0x02, 0x1F), t(0x0F, 0x04, 0x1F)]), // Razer Cobra
    (0x00A5, LOW_BATTERY),            // Razer Viper V2 Pro (Wired)
    (0x00A6, LOW_BATTERY),            // Razer Viper V2 Pro (Wireless)
    (0x00AA, LOW_BATTERY),            // Razer Basilisk V3 Pro (Wired)
    (0x00AB, LOW_BATTERY),            // Razer Basilisk V3 Pro (Wireless)
    (0x00AF, LOW_BATTERY),            // Razer Cobra Pro (Wired)
    (0x00B0, LOW_BATTERY),            // Razer Cobra Pro (Wireless)
    (0x00B3, &[t(0x07, 0x01, 0xFF), t(0x07, 0x10, 0xFF)]), // Razer HyperPolling Wireless Dongle
    (0x00C1, &[t(0x07, 0x10, 0xFF)]), // Razer Viper V3 Pro (Wireless)
    (0x00CC, LOW_BATTERY),            // Razer Basilisk V3 Pro 35K (Wired)
    (0x00CD, LOW_BATTERY),            // Razer Basilisk V3 Pro 35K (Wireless)
    (0x00D6, LOW_BATTERY),            // Razer Basilisk V3 Pro 35K Phantom Green Edition (Wired)
    (0x00D7, LOW_BATTERY),            // Razer Basilisk V3 Pro 35K Phantom Green Edition (Wireless)
    (0x00DA, LOW_BATTERY),            // Razer Cobra HyperSpeed
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
    fn sorted_for_binary_search() {
        assert!(TIDS.windows(2).all(|p| p[0].0 < p[1].0));
    }

    #[test]
    fn reads_share_their_write_s_id() {
        let viper = Model::find(0x0078).unwrap();
        assert_eq!(viper.tid_for(0x0F, 0x02), Some(0x3F));
        assert_eq!(viper.tid_for(0x0F, 0x84), Some(0x3F));
        assert_eq!(viper.tid_for(0x00, 0x81), None);
    }
}

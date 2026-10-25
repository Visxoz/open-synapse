#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::commands::{Batch, Query};
use crate::model::{Effects, Led, LightProto, Model};
use crate::report::{Report, VARSTORE};
use crate::types::Rgb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Breath {
    Single(Rgb),
    Dual(Rgb, Rgb),
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Effect {
    Off,
    On,
    Static(Rgb),
    Blinking(Rgb),
    Breathing(Breath),
    Spectrum,
    /// 1 or 2, the two directions a wave can travel.
    Wave(u8),
    /// Lights up on clicks, fading at a speed of 1 (fast) to 4.
    Reactive {
        speed: u8,
        colour: Rgb,
    },
}

/// The adjustable parts of an effect, kept while switching between kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectParams {
    pub a: Rgb,
    pub b: Rgb,
    pub speed: u8,
    pub direction: u8,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            a: Rgb::new(0x44, 0xD6, 0x2C),
            b: Rgb::new(0x3D, 0x8B, 0xFF),
            speed: 2,
            direction: 1,
        }
    }
}

impl Effect {
    pub fn build(kind: Effects, p: &EffectParams) -> Option<Self> {
        Some(match kind {
            Effects::OFF => Self::Off,
            Effects::ON => Self::On,
            Effects::STATIC => Self::Static(p.a),
            Effects::BLINKING => Self::Blinking(p.a),
            Effects::BREATH_SINGLE => Self::Breathing(Breath::Single(p.a)),
            Effects::BREATH_DUAL => Self::Breathing(Breath::Dual(p.a, p.b)),
            Effects::BREATH_RANDOM => Self::Breathing(Breath::Random),
            Effects::SPECTRUM => Self::Spectrum,
            Effects::WAVE => Self::Wave(p.direction),
            Effects::REACTIVE => Self::Reactive {
                speed: p.speed,
                colour: p.a,
            },
            _ => return None,
        })
    }

    /// `p` with whatever this effect sets.
    pub fn params(&self, mut p: EffectParams) -> EffectParams {
        match *self {
            Self::Static(a) | Self::Blinking(a) | Self::Breathing(Breath::Single(a)) => p.a = a,
            Self::Breathing(Breath::Dual(a, b)) => (p.a, p.b) = (a, b),
            Self::Reactive { speed, colour } => (p.speed, p.a) = (speed, colour),
            Self::Wave(direction) => p.direction = direction,
            Self::Off | Self::On | Self::Spectrum | Self::Breathing(Breath::Random) => {}
        }
        p
    }

    pub const fn label(kind: Effects) -> &'static str {
        match kind {
            Effects::STATIC => "Static",
            Effects::BREATH_SINGLE => "Breathing",
            Effects::BREATH_DUAL => "Dual breathing",
            Effects::BREATH_RANDOM => "Random breathing",
            Effects::SPECTRUM => "Spectrum",
            Effects::WAVE => "Wave",
            Effects::REACTIVE => "Reactive",
            Effects::BLINKING => "Blinking",
            Effects::ON => "On",
            Effects::OFF => "Off",
            _ => "",
        }
    }

    /// The effect in white, which one-colour LEDs show at full strength.
    pub const fn in_white(self) -> Self {
        const W: Rgb = Rgb::new(0xFF, 0xFF, 0xFF);
        match self {
            Self::Static(_) => Self::Static(W),
            Self::Blinking(_) => Self::Blinking(W),
            Self::Breathing(Breath::Single(_)) => Self::Breathing(Breath::Single(W)),
            Self::Breathing(Breath::Dual(..)) => Self::Breathing(Breath::Dual(W, W)),
            Self::Reactive { speed, .. } => Self::Reactive { speed, colour: W },
            other => other,
        }
    }

    pub const fn kind(&self) -> Effects {
        match self {
            Self::Off => Effects::OFF,
            Self::On => Effects::ON,
            Self::Static(_) => Effects::STATIC,
            Self::Blinking(_) => Effects::BLINKING,
            Self::Breathing(Breath::Single(_)) => Effects::BREATH_SINGLE,
            Self::Breathing(Breath::Dual(..)) => Effects::BREATH_DUAL,
            Self::Breathing(Breath::Random) => Effects::BREATH_RANDOM,
            Self::Spectrum => Effects::SPECTRUM,
            Self::Wave(_) => Effects::WAVE,
            Self::Reactive { .. } => Effects::REACTIVE,
        }
    }
}

/// The reports that show `effect` on `led`, or none if the mouse lacks it.
pub fn set_effect(m: &Model, led: Led, effect: &Effect) -> Batch {
    let supported = m.lighting.zone(led).is_some_and(|z| z.effects.contains(effect.kind()));
    if !supported {
        return Batch::EMPTY;
    }
    let rgb = m.lighting.rgb;
    let effect = &if rgb { *effect } else { effect.in_white() };
    use LightProto as P;
    match (m.lighting.proto, led, effect) {
        (P::Classic, ..) | (P::StandardMatrix, Led::Scroll | Led::Logo, Effect::On | Effect::Off) => {
            classic(led, effect, rgb)
        }
        (P::MouseExtended, ..) => mouse_extended(led, effect),
        (P::StandardMatrix, ..) => standard_matrix(effect),
        (P::NagaPro | P::NagaV2Pro | P::NagaTrinity, Led::Matrix, Effect::Static(c)) => {
            Batch::of(&[CUSTOM_MODE, trinity_static(*c)])
        }
        (P::NagaPro, Led::Matrix, Effect::Off) => standard_matrix(effect),
        (P::NagaPro, Led::Matrix, _) | (P::NagaV2Pro, Led::Matrix, Effect::Reactive { .. }) => {
            mouse_extended(Led::Backlight, effect)
        }
        (P::NagaV2Pro, Led::Matrix, _) => trinity(effect),
        (P::BasiliskMobile, Led::Matrix, Effect::Static(c)) => Batch::of(&[CUSTOM_MODE, basilisk_mobile_static(*c)]),
        _ => extended(led, effect),
    }
}

/// The class and command that set `led`'s brightness, if it has one; the
/// command with 0x80 reads it back.
fn brightness_command(m: &Model, led: Led) -> Option<(u8, u8)> {
    m.lighting.zone(led)?.brightness.then_some(match m.lighting.proto {
        LightProto::Classic | LightProto::MouseExtended | LightProto::StandardMatrix => (0x03, 0x03),
        _ => (0x0F, 0x04),
    })
}

pub fn get_brightness(m: &Model, led: Led) -> Option<Query<u8>> {
    let (class, id) = brightness_command(m, led)?;
    Some(Query {
        request: Report::new(class, id | 0x80, 0x03, &[VARSTORE, led as u8]),
        parse: |r| r.args[2],
    })
}

pub fn set_brightness(m: &Model, led: Led, level: u8) -> Batch {
    match brightness_command(m, led) {
        Some((class, id)) => Report::new(class, id, 0x03, &[VARSTORE, led as u8, level]).into(),
        None => Batch::EMPTY,
    }
}

fn batch(class: u8, id: u8, size: u8, a: [u8; 16]) -> Batch {
    Report::new(class, id, size, &a).into()
}

fn args<const N: usize>(head: [u8; N], tail: &[u8]) -> [u8; 16] {
    let mut a = [0; 16];
    a[..N].copy_from_slice(&head);
    a[N..N + tail.len()].copy_from_slice(tail);
    a
}

fn dual(a: Rgb, b: Rgb) -> [u8; 6] {
    let [r1, g1, b1] = a.bytes();
    let [r2, g2, b2] = b.bytes();
    [r1, g1, b1, r2, g2, b2]
}

fn extended(led: Led, effect: &Effect) -> Batch {
    let l = led as u8;
    let r = |size, a| batch(0x0F, 0x02, size, a);
    match *effect {
        Effect::Off => r(0x06, args([VARSTORE, l, 0x00], &[])),
        Effect::Static(c) => r(0x09, args([VARSTORE, l, 0x01, 0, 0, 0x01], &c.bytes())),
        Effect::Breathing(Breath::Random) => r(0x06, args([VARSTORE, l, 0x02], &[])),
        Effect::Breathing(Breath::Single(c)) => r(0x09, args([VARSTORE, l, 0x02, 0x01, 0, 0x01], &c.bytes())),
        Effect::Breathing(Breath::Dual(a, b)) => r(0x0C, args([VARSTORE, l, 0x02, 0x02, 0, 0x02], &dual(a, b))),
        Effect::Spectrum => r(0x06, args([VARSTORE, l, 0x03], &[])),
        Effect::Wave(dir) => r(0x06, args([VARSTORE, l, 0x04, dir.clamp(1, 2), 0x28], &[])),
        Effect::Reactive { speed, colour } => r(
            0x09,
            args([VARSTORE, l, 0x05, 0, speed.clamp(1, 4), 0x01], &colour.bytes()),
        ),
        Effect::On | Effect::Blinking(_) => Batch::EMPTY,
    }
}

fn mouse_extended(led: Led, effect: &Effect) -> Batch {
    let l = led as u8;
    let r = |size, a| batch(0x03, 0x0D, size, a);
    match *effect {
        Effect::Off => r(0x03, args([VARSTORE, l, 0x00], &[])),
        Effect::Static(c) => r(0x06, args([VARSTORE, l, 0x06], &c.bytes())),
        Effect::Spectrum => r(0x03, args([VARSTORE, l, 0x04], &[])),
        Effect::Reactive { speed, colour } => r(0x07, args([VARSTORE, l, 0x02, speed.clamp(1, 4)], &colour.bytes())),
        Effect::Breathing(Breath::Random) => r(0x0A, args([VARSTORE, l, 0x03, 0x03], &[])),
        Effect::Breathing(Breath::Single(c)) => r(0x0A, args([VARSTORE, l, 0x03, 0x01], &c.bytes())),
        Effect::Breathing(Breath::Dual(a, b)) => r(0x0A, args([VARSTORE, l, 0x03, 0x02], &dual(a, b))),
        Effect::On | Effect::Blinking(_) | Effect::Wave(_) => Batch::EMPTY,
    }
}

/// One effect for the whole device, ignoring the LED.
fn standard_matrix(effect: &Effect) -> Batch {
    let r = |size, a| batch(0x03, 0x0A, size, a);
    match *effect {
        Effect::Off => r(0x01, args([0x00], &[])),
        Effect::Wave(dir) => r(0x02, args([0x01, dir.clamp(1, 2)], &[])),
        Effect::Reactive { speed, colour } => r(0x05, args([0x02, speed.clamp(1, 4)], &colour.bytes())),
        Effect::Breathing(Breath::Random) => r(0x08, args([0x03, 0x03], &[])),
        Effect::Breathing(Breath::Single(c)) => r(0x08, args([0x03, 0x01], &c.bytes())),
        Effect::Breathing(Breath::Dual(a, b)) => r(0x08, args([0x03, 0x02], &dual(a, b))),
        Effect::Spectrum => r(0x01, args([0x04], &[])),
        Effect::Static(c) => r(0x04, args([0x06], &c.bytes())),
        Effect::On | Effect::Blinking(_) => Batch::EMPTY,
    }
}

/// Turn the LED on, set its colour unless it has only one, then pick the
/// effect.
fn classic(led: Led, effect: &Effect, rgb: bool) -> Batch {
    let l = led as u8;
    let state = |on: bool| Report::new(0x03, 0x00, 0x03, &[VARSTORE, l, u8::from(on)]);
    let colour = |c: Rgb| Report::new(0x03, 0x01, 0x05, &args([VARSTORE, l], &c.bytes()));
    let mode = |id: u8| Report::new(0x03, 0x02, 0x03, &[VARSTORE, l, id]);
    let lit = |c: Rgb, id: u8| {
        if rgb {
            Batch::of(&[state(true), colour(c), mode(id)])
        } else {
            Batch::of(&[state(true), mode(id)])
        }
    };
    match *effect {
        Effect::Off => state(false).into(),
        Effect::On => state(true).into(),
        Effect::Static(c) => lit(c, 0x00),
        Effect::Blinking(c) => lit(c, 0x01),
        Effect::Breathing(Breath::Single(c)) => lit(c, 0x02),
        Effect::Spectrum => Batch::of(&[state(true), mode(0x04)]),
        _ => Batch::EMPTY,
    }
}

/// Switches the Naga Trinity family and the Basilisk Mobile to per-LED colours.
const CUSTOM_MODE: Report = Report {
    size: 0x06,
    class: 0x0F,
    id: 0x02,
    args: {
        let mut a = [0; crate::report::ARGS_LEN];
        a[2] = 0x08;
        a
    },
    ..Report::BLANK
};

fn trinity_static(c: Rgb) -> Report {
    let [r, g, b] = c.bytes();
    Report::new(0x0F, 0x03, 0x0E, &[0, 0, 0, 0, 0x02, r, g, b, r, g, b, r, g, b])
}

fn basilisk_mobile_static(c: Rgb) -> Report {
    let [r, g, b] = c.bytes();
    Report::new(0x0F, 0x03, 0x0B, &[0, 0, 0, 0, 0x01, r, g, b, r, g, b])
}

fn trinity(effect: &Effect) -> Batch {
    let r = |size, a| batch(0x0F, 0x02, size, a);
    match *effect {
        Effect::Off => r(0x06, args([0, 0, 0x00], &[])),
        Effect::Spectrum => r(0x06, args([0, 0, 0x03], &[])),
        Effect::Breathing(Breath::Random) => r(0x06, args([0, 0, 0x02], &[])),
        Effect::Breathing(Breath::Single(c)) => r(0x09, args([0, 0, 0x02, 0x01, 0, 0x01], &c.bytes())),
        Effect::Breathing(Breath::Dual(a, b)) => r(0x0C, args([0, 0, 0x02, 0x02, 0, 0x02], &dual(a, b))),
        _ => Batch::EMPTY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(pid: u16) -> &'static Model {
        Model::find(pid).unwrap()
    }

    #[test]
    fn extended_static_matches_openrazer() {
        let red = Rgb::new(0xFF, 0, 0);
        let batch = set_effect(model(0x00AA), Led::Logo, &Effect::Static(red));
        assert_eq!(batch.len(), 1);
        let r = batch[0];
        assert_eq!((r.class, r.id, r.size), (0x0F, 0x02, 0x09));
        assert_eq!(&r.args[..9], &[1, 0x04, 0x01, 0, 0, 1, 0xFF, 0, 0]);
    }

    #[test]
    fn classic_static_is_three_reports() {
        let batch = set_effect(model(0x0043), Led::Logo, &Effect::Static(Rgb::new(1, 2, 3)));
        let ids: Vec<_> = batch.iter().map(|r| (r.class, r.id)).collect();
        assert_eq!(ids, [(0x03, 0x00), (0x03, 0x01), (0x03, 0x02)]);
    }

    #[test]
    fn one_colour_leds_skip_the_colour_and_take_white() {
        // The DeathAdder 2000 breathes as OpenRazer does: on, then the effect.
        let batch = set_effect(
            model(0x004F),
            Led::Logo,
            &Effect::Breathing(Breath::Single(Rgb::new(1, 2, 3))),
        );
        let ids: Vec<_> = batch.iter().map(|r| (r.class, r.id, r.args[2])).collect();
        assert_eq!(ids, [(0x03, 0x00, 0x01), (0x03, 0x02, 0x02)]);
        // Class 0x0F effects still carry a colour, in white.
        let batch = set_effect(model(0x006E), Led::Logo, &Effect::Static(Rgb::new(1, 2, 3)));
        assert_eq!(&batch[0].args[6..9], &[0xFF; 3]);
    }

    #[test]
    fn naga_trinity_switches_mode_first() {
        let batch = set_effect(model(0x0067), Led::Matrix, &Effect::Static(Rgb::new(9, 9, 9)));
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].args[2], 0x08);
        assert_eq!((batch[1].class, batch[1].id), (0x0F, 0x03));
    }

    #[test]
    fn every_kind_builds_and_keeps_its_params() {
        let p = EffectParams {
            a: Rgb::new(1, 2, 3),
            b: Rgb::new(4, 5, 6),
            speed: 3,
            direction: 2,
        };
        for i in 0..10 {
            let kind = Effects::from_index(i);
            let effect = Effect::build(kind, &p).unwrap();
            assert_eq!(effect.kind(), kind);
            assert_eq!(
                Effect::build(kind, &effect.params(EffectParams::default())),
                Some(effect)
            );
            assert!(!Effect::label(kind).is_empty());
        }
    }

    #[test]
    fn brightness_commands_follow_the_family() {
        // Extended, class 0x0F.
        let get = get_brightness(model(0x00AA), Led::Logo).unwrap().request;
        assert_eq!((get.class, get.id, &get.args[..2]), (0x0F, 0x84, &[1, 0x04][..]));
        let set = set_brightness(model(0x00AA), Led::Logo, 7)[0];
        assert_eq!((set.class, set.id, &set.args[..3]), (0x0F, 0x04, &[1, 0x04, 7][..]));
        // Classic, class 0x03.
        let set = set_brightness(model(0x001F), Led::Scroll, 7)[0];
        assert_eq!((set.class, set.id), (0x03, 0x03));
        assert_eq!(get_brightness(model(0x001F), Led::Scroll).unwrap().request.id, 0x83);
        // A zone without brightness.
        assert!(get_brightness(model(0x0015), Led::Logo).is_none());
        assert!(set_brightness(model(0x0015), Led::Logo, 7).is_empty());
    }

    #[test]
    fn refuses_what_the_zone_lacks() {
        assert!(set_effect(model(0x00E6), Led::Logo, &Effect::Spectrum).is_empty());
        assert!(set_effect(model(0x0099), Led::Logo, &Effect::Off).is_empty());
    }
}

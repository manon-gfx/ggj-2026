use crate::{audio::notes::*, game::Key};
use interp::{InterpMode, interp};

struct MusicSettings;

impl MusicSettings {
    pub const TEMPO: f64 = 560. / 60.; // beats per second
}

pub const MELODY1: [f64; 32] = [
    A1, A1, REST, A1, A1, REST, A1, A1, REST, REST, C2, C2, REST, REST, REST, REST, F1, F1, REST,
    F1, F1, REST, F1, F1, REST, REST, G1, G1, REST, REST, REST, REST,
];

pub const MELODY2: [f64; 32] = [
    REST, REST, E4, REST, C4, REST, REST, D4, REST, REST, A4, REST, REST, REST, REST, REST, REST,
    REST, C4, REST, G4, REST, REST, A4, REST, REST, E4, REST, REST, REST, REST, REST,
];

pub fn signal(t: f64) -> f64 {
    let mut signal = 0.0;

    let n = (t * MusicSettings::TEMPO) as u32;
    let note1 = MELODY1[(n % MELODY1.len() as u32) as usize];
    if note1 != REST {
        signal += 0.5 * square_wave(t, note1);
    }
    let note2 = MELODY2[(n % MELODY2.len() as u32) as usize];
    if note2 != REST {
        signal += triangle_wave(t, note2);
    }

    signal
}

pub fn triangle_wave(t: f64, freq: f64) -> f64 {
    let saw = 2. * sawtooth_wave(t, freq);
    if saw > 1. {
        2. - saw
    } else if saw < -1. {
        -2. - saw
    } else {
        saw
    }
}

pub fn square_wave(t: f64, freq: f64) -> f64 {
    2. * (((freq * t * 2.) as i32) % 2) as f64 - 1.
}

pub fn sawtooth_wave(t: f64, freq: f64) -> f64 {
    2. * (freq * t) % 2. - 1.
}

pub fn custom_wave(t: f64, freq: f64, ttable: &[f64], ytable: &[f64]) -> f64 {
    let t_rel = (freq * t) % 1.;
    interp(&ttable, &ytable, t_rel, &InterpMode::default())
}

pub const TRIANGLETTABLE: [f64; 5] = [0.0, 0.25, 0.50, 0.75, 1.0];
pub const TRIANGLEYTABLE: [f64; 5] = [0.0, 1.0, 0.0, -1.0, 0.0];

pub const SINETTABLE: [f64; 21] = [
    0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70, 0.75,
    0.80, 0.85, 0.90, 0.95, 1.0,
];
pub const SINEYTABLE: [f64; 21] = [
    0.0000000000000000,
    0.3090169883750000,
    0.5877852522920000,
    0.8090169883750000,
    0.9510565162950000,
    1.0000000000000000,
    0.9510565162950000,
    0.8090169883750000,
    0.5877852522920000,
    0.3090169883750000,
    0.0000000000000000,
    -0.3090169883750000,
    -0.5877852522920000,
    -0.8090169883750000,
    -0.9510565162950000,
    -1.0000000000000000,
    -0.9510565162950000,
    -0.8090169883750000,
    -0.5877852522920000,
    -0.3090169883750000,
    0.0000000000000000,
];

pub fn sine_wave(t: f64, freq: f64) -> f64 {
    custom_wave(t, freq, &SINETTABLE, &SINEYTABLE)
}

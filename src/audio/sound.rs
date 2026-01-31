use crate::{audio::notes::*, game::Key};

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

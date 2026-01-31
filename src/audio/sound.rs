use crate::audio::notes::*;

struct MusicSettings;

impl MusicSettings {
    pub const TEMPO: f64 = 560. / 60.; // beats per second
}

pub const MELODY: [f64; 32] = [
    A1, A1, REST, A1, A1, REST, A1, A1, REST, REST, C2, C2, REST, REST, REST, REST, F1, F1, REST,
    F1, F1, REST, F1, F1, REST, REST, G1, G1, REST, REST, REST, REST,
];

pub fn audio(t: f64) -> f64 {
    let n = (t * MusicSettings::TEMPO) as u32;
    let note = MELODY[(n % MELODY.len() as u32) as usize];
    if note != REST {
        square_wave(t, note)
    } else {
        0.0
    }
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

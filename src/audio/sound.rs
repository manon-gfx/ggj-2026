use crate::audio::notes::*;
use interp::{InterpMode, interp};

struct MusicSettings;

impl MusicSettings {
    pub const TEMPO: f64 = 120. / 60.; // beats per second
    pub const BAR_LENGTH: usize = 4; // beats per bar
    pub const LOOP_LENGTH: usize = 8; // bars per loop
}

type WaveFn = fn(f64, f64) -> f64;

struct Track {
    pub wave: WaveFn,
    pub length: usize, // number of bars per track
    pub melody: &'static [f64],
}

const TRACK1: Track = Track {
    wave: triangle_wave,
    length: 8,
    melody: &[
        D4, D4, D4, A3, C4, C4, C4, A3, G3, G3, G3, F3, A3, A3, A3, REST, D4, D4, D4, A3, C4, C4,
        C4, A3, G3, G3, G3, F3, D3, D3, D3, REST,
    ],
};

const TRACK2: Track = Track {
    wave: square_wave,
    length: 8,
    melody: &[
        REST, REST, D2, D2,
        REST, D2, D2, REST,
        D2, D2, REST, REST,
        REST, REST, REST, REST,
        REST, REST, E2, E2,
        REST, E2, E2, REST,
        E2, E2, REST, REST,
        REST, REST, REST, REST,
        REST, REST, G2, G2,
        REST, G2, G2, REST,
        G2, G2, REST, REST,
        REST, REST, REST, REST,
        REST, REST, A2, A2,
        REST, A2, A2, REST,
        A2, A2, REST, REST,
        G2, G2, G2, REST,
        REST, REST, D2, D2,
        REST, D2, D2, REST,
        D2, D2, REST, REST,
        REST, REST, REST, REST,
        REST, REST, E2, E2,
        REST, E2, E2, REST,
        E2, E2, REST, REST,
        REST, REST, REST, REST,
        REST, REST, G2, G2,
        REST, G2, G2, REST,
        G2, G2, REST, REST,
        CS2, CS2, CS2, REST,
        D2, D2, REST, D2,
        D2, REST, D2, D2,
        D2, REST, REST, REST,
        REST, REST, REST, REST,
    ],
};

pub fn signal(t: f64) -> f64 {
    let mut signal = 0.0;

    let beat_in_game= (t * MusicSettings::TEMPO); // total beats since start of game
    let beat_in_loop = beat_in_game % (MusicSettings::BAR_LENGTH * MusicSettings::LOOP_LENGTH) as f64; // current beat in the loop

    let beat_in_track = beat_in_loop / (MusicSettings::LOOP_LENGTH / TRACK1.length) as f64;
    let idx_in_track = (beat_in_track * (TRACK1.melody.len() / (TRACK1.length * MusicSettings::BAR_LENGTH)) as f64) as usize % TRACK1.melody.len();
    let note = TRACK1.melody[idx_in_track];
    signal += 1.0 * (TRACK1.wave)(t, note);

    let beat_in_track = beat_in_loop / (MusicSettings::LOOP_LENGTH / TRACK1.length) as f64;
    let idx_in_track = (beat_in_track * (TRACK2.melody.len() / (TRACK2.length * MusicSettings::BAR_LENGTH)) as f64) as usize % TRACK2.melody.len();
    let note = TRACK2.melody[idx_in_track];
    signal += 0.5 * (TRACK2.wave)(t, note);

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

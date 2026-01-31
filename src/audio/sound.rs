use crate::audio::notes::*;
use interp::{InterpMode, interp};

struct MusicSettings;

impl MusicSettings {
    pub const TEMPO: f64 = 120. / 60.; // beats per second
    pub const BAR_LENGTH: usize = 4; // beats per bar
    pub const LOOP_LENGTH: usize = 8; // bars per loop
}

type WaveFn = fn(f64, f64) -> f64;

pub struct Track {
    pub wave: WaveFn,
    pub length: usize, // number of bars per track
    pub melody: &'static [f64],
    pub volume: f64,
}

pub struct Sound {
    pub wave: WaveFn,
    pub start: f64,    // start time
    pub duration: f64, // duration in seconds
    pub interval: f64, // interval in seconds
    pub melody: &'static [f64],
    pub volume: f64,
}

pub struct Music {
    pub tracks: Vec<Track>,
    pub track_mask: Vec<bool>,
}

impl Music {
    pub fn new() -> Self {
        let melody_track = Track {
            wave: triangle_wave,
            length: 8 * MusicSettings::BAR_LENGTH,
            melody: &[
                D4, D4, D4, A3, C4, C4, C4, A3, G3, G3, G3, F3, A3, A3, A3, REST, D4, D4, D4, A3,
                C4, C4, C4, A3, G3, G3, G3, F3, D3, D3, D3, REST,
            ],
            volume: 0.5,
        };

        let contramelody_track = Track {
            wave: sine_wave,
            length: 8 * MusicSettings::BAR_LENGTH,
            melody: &[
                A4, A4, A4, F4, G4, G4, G4, D4, F4, F4, F4, CS4, E4, E4, E4, REST, A4, A4, A4, F4,
                G4, G4, G4, D4, F4, F4, F4, G4, A4, A4, A4, REST,
            ],
            volume: 0.5,
        };

        let bass_track = Track {
            wave: square_wave,
            length: 8 * MusicSettings::BAR_LENGTH,
            melody: &[
                REST, REST, D2, D2, REST, D2, D2, REST, D2, D2, REST, REST, REST, REST, REST, REST,
                REST, REST, E2, E2, REST, E2, E2, REST, E2, E2, REST, REST, REST, REST, REST, REST,
                REST, REST, G2, G2, REST, G2, G2, REST, G2, G2, REST, REST, REST, REST, REST, REST,
                REST, REST, A2, A2, REST, A2, A2, REST, A2, A2, REST, REST, G2, G2, G2, REST, REST,
                REST, D2, D2, REST, D2, D2, REST, D2, D2, REST, REST, REST, REST, REST, REST, REST,
                REST, E2, E2, REST, E2, E2, REST, E2, E2, REST, REST, REST, REST, REST, REST, REST,
                REST, G2, G2, REST, G2, G2, REST, G2, G2, REST, REST, CS2, CS2, CS2, REST, D2, D2,
                REST, D2, D2, REST, D2, D2, D2, REST, REST, REST, REST, REST, REST, REST,
            ],
            volume: 0.25,
        };

        let accent_track = Track {
            wave: triangle_wave,
            length: 4 * MusicSettings::BAR_LENGTH,
            melody: &[
                REST, REST, D5, REST, D5, REST, C5, A4, REST, REST, REST, REST, REST, REST, REST,
                REST, REST, REST, A4, REST, A4, REST, C5, D5, REST, REST, REST, REST, REST, REST,
                REST, REST, REST, REST, D5, REST, D5, REST, F5, D5, REST, REST, REST, REST, REST,
                REST, REST, REST, REST, REST, C5, REST, C5, REST, B4, A4, REST, REST, REST, REST,
                REST, REST, REST, REST,
            ],
            volume: 0.5,
        };

        let snare_track = Track {
            wave: white_noise,
            length: 4 * MusicSettings::BAR_LENGTH,
            melody: &[
                REST, REST, REST, REST, C3, REST, REST, REST, REST, REST, REST, REST, C3, REST,
                REST, REST, REST, REST, REST, REST, C3, REST, REST, REST, REST, REST, REST, REST,
                C3, REST, REST, REST, REST, REST, REST, REST, C3, REST, REST, REST, REST, REST,
                REST, REST, C3, REST, REST, REST, REST, REST, REST, REST, C3, REST, REST, REST,
                REST, REST, C3, REST, C3, REST, REST, REST,
            ],
            volume: 0.5,
        };

        Self {
            tracks: vec![
                bass_track,
                melody_track,
                contramelody_track,
                accent_track,
                snare_track,
            ],
            track_mask: vec![false, false, false, false, false],
        }
    }
}

pub enum SoundTypes {
    FootstepSound,
    JumpSound,
    DeathSound,
}

pub struct SoundEffects {
    pub footstep: Sound,
    pub jump: Sound,
    pub death: Sound,
}

impl SoundEffects {
    pub fn new() -> Self {
        let footstep = Sound {
            wave: triangle_wave,
            start: 0.,
            duration: 0.15,
            interval: 0.3,
            melody: &[D2],
            volume: 0.2,
        };

        let jump = Sound {
            wave: sawtooth_wave,
            start: 0.,
            duration: 0.1,
            interval: 0.0,
            melody: &[C4, E4, G4, C5],
            volume: 0.5,
        };

        let death = Sound {
            wave: square_wave,
            start: 0.,
            duration: 2.,
            interval: 0.0,
            melody: &[
                D2, A1, REST, C2, G1, REST, A1, F1, REST, REST, D1, REST, D1, D1, D1, REST,
            ],
            volume: 0.5,
        };

        Self {
            footstep: footstep,
            jump: jump,
            death: death,
        }
    }
}

pub fn play_music(t: f64, music: &mut Music) -> f64 {
    let mut signal = 0.0;

    let beat_in_game = (t * MusicSettings::TEMPO); // total beats since start of game
    let beat_in_loop =
        beat_in_game % (MusicSettings::BAR_LENGTH * MusicSettings::LOOP_LENGTH) as f64; // current beat in the loop

    if beat_in_game > 32. {
        music.track_mask[4] = true;
    }

    for (track, play_track) in music.tracks.iter().zip(music.track_mask.iter()) {
        if *play_track {
            let beat_in_track = beat_in_loop % track.length as f64;
            let idx_in_track = (beat_in_track as f64 / track.length as f64
                * track.melody.len() as f64)
                .floor() as usize;
            let note = track.melody[idx_in_track];
            if note != REST {
                signal += track.volume * (track.wave)(t, note);
            }
        }
    }

    signal
}

pub fn play_sfx(t: f64, t0: f64, sound: &Sound) -> f64 {
    let dt = if sound.interval == 0.0 {
        t - t0
    } else {
        (t - t0) % sound.interval
    };

    if sound.duration > dt {
        let idx_in_melody = (dt / sound.duration * sound.melody.len() as f64) as usize;
        let note = sound.melody[idx_in_melody];
        sound.volume * (sound.wave)(t, note)
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

fn wang_hash(seed: u32) -> u32 {
    let seed = (seed ^ 61) ^ (seed >> 16);
    let seed = seed.overflowing_mul(9).0;
    let seed = seed ^ (seed >> 4);
    let seed = seed.overflowing_mul(0x27d4eb2d).0;
    let seed = seed ^ (seed >> 15);
    seed
}

pub fn white_noise(t: f64, freq: f64) -> f64 {
    let rand_u32 = wang_hash((t * 44_100.) as u32);
    let rand_0_1 = (rand_u32 as f64) / (u32::MAX as f64);
    1. - 2. * rand_0_1
}

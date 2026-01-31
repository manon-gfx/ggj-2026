use std::collections::HashMap;
use std::sync::{
    Arc,
    mpsc::{Receiver, Sender, channel},
};

pub mod notes;
pub mod sound;
use sound::{play_music, play_sfx};

use crate::audio::sound::SoundTypes;
use crate::audio::sound::{
    SoundEffects, sawtooth_wave, sine_wave, square_wave, triangle_wave, white_noise,
};
use crate::game::Key;

#[derive(Clone)]
struct AudioSettings {
    volume: f32,
    panning: f32,
    frequency: f32,
    note: u32,
}
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.5,
            panning: 0.5,
            frequency: 440.0,
            note: 69,
        }
    }
}

fn note_to_freq(note: u32) -> f64 {
    let note = note.max(20);
    // original -> 1.059_463_094_359_295_264_561_825_294_946_3
    //8.0 * f64::powi(1.059_463_094_359_295_3, note as i32)
    440.0 * f64::powf(2.0, (note as f64 - 69.0) / 12.0)
}

fn approx_exp(val: f64) -> f64 {
    let mut x = 1.0 + val / 4.0;
    x *= x;
    x *= x;
    x
}
fn approx_exp32(val: f32) -> f32 {
    let mut x = 1.0 + val / 4.0;
    x *= x;
    x *= x;
    x
}

pub(crate) struct Audio {
    _host: cpal::Host,     //raii
    _device: cpal::Device, //raii
    _stream: cpal::Stream, //raii

    config: cpal::SupportedStreamConfig,

    settings: AudioSettings,

    settings_sender: Sender<AudioSettings>,

    pub shit_recv: Receiver<Vec<f32>>,

    pub key_sender: Sender<(Key, bool)>,
    pub sfx_sender: Sender<(SoundTypes, bool)>,
}

impl Audio {
    pub(crate) fn new() -> Self {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No audio output device available.");
        let mut supported_configs_range = device.supported_output_configs().unwrap();

        let supported_config = supported_configs_range
            .find(|config| {
                if config.sample_format() == cpal::SampleFormat::F32
                    && config.channels() == 2
                    && config.min_sample_rate() >= 22050
                {
                    true
                } else {
                    false
                }
            })
            .expect("Failed to find a suitable audio format");
        let supported_config = supported_config.with_max_sample_rate();

        // let supported_config = supported_configs_range
        //     .next()
        //     .unwrap()
        //     .with_max_sample_rate();
        let sample_format = supported_config.sample_format();
        dbg!(sample_format);
        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let mut config: cpal::StreamConfig = supported_config.clone().into();
        config.buffer_size = cpal::BufferSize::Fixed(512); // ~11.6ms latency at 44.1kHz

        assert!(sample_format == cpal::SampleFormat::F32);

        let settings: AudioSettings = AudioSettings::default();
        let (settings_sender, settings_recv) = channel();

        let (shit_sender, shit_recv) = channel();
        let (key_sender, key_recv) = channel();
        let (sfx_sender, sfx_recv) = channel();

        struct StreamContext {
            settings: AudioSettings,
            settings_recv: Receiver<AudioSettings>,
        }

        let mut context = StreamContext {
            settings: settings.clone(),
            settings_recv,
        };

        let mut last_time = 0.0;
        let mut time = 0.0;
        let mut piano_notes: [bool; 17] = [false; 17];
        let mut max_value: f32 = 0.0;

        let mut music = sound::Music::new();
        music.track_mask[0] = true;

        let mut soundeffects = sound::SoundEffects::new();
        let mut start_jump_sound: bool = false;
        let mut start_death_sound: bool = false;
        let mut start_footstep_sound: bool = false;
        let mut stop_footstep_sound: bool = false;
        let mut play_jump_sound: bool = false;
        let mut play_death_sound: bool = false;
        let mut play_footstep_sound: bool = false;
        let mut t0_jump_sound: f64 = 0.0;
        let mut t0_death_sound: f64 = 0.0;
        let mut t0_footstep_sound: f64 = 0.0;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    while let Ok((key, down)) = key_recv.try_recv() {
                        match key {
                            Key::MusicC3 => piano_notes[0] = down,
                            Key::MusicCs3 => piano_notes[1] = down,
                            Key::S => piano_notes[2] = down,
                            Key::MusicDs3 => piano_notes[3] = down,
                            Key::MusicE3 => piano_notes[4] = down,
                            Key::MusicF3 => piano_notes[5] = down,
                            Key::MusicFs3 => piano_notes[6] = down,
                            Key::G => piano_notes[7] = down,
                            Key::MusicGs3 => piano_notes[8] = down,
                            Key::MusicA3 => piano_notes[9] = down,
                            Key::MusicAs3 => piano_notes[10] = down,
                            Key::MusicB3 => piano_notes[11] = down,
                            Key::MusicC4 => piano_notes[12] = down,
                            Key::MusicCs4 => piano_notes[13] = down,
                            Key::MusicD4 => piano_notes[14] = down,
                            Key::MusicDs4 => piano_notes[15] = down,
                            Key::MusicE4 => piano_notes[16] = down,
                            _ => {}
                        }
                    }

                    while let Ok((sfx_event, play)) = sfx_recv.try_recv() {
                        match sfx_event {
                            SoundTypes::FootstepSound => {
                                if play {
                                    start_footstep_sound = true;
                                } else {
                                    stop_footstep_sound = true;
                                }
                            }
                            SoundTypes::JumpSound => {
                                start_jump_sound = play;
                            }
                            SoundTypes::DeathSound => {
                                start_death_sound = play;
                            }
                            _ => {}
                        }
                    }

                    let sample_duration = 1.0 / sample_rate as f64;
                    let chunk_time = (data.len() / channels as usize) as f64 / sample_rate as f64;

                    if let Ok(settings) = context.settings_recv.try_recv() {
                        context.settings = settings;
                    }

                    let mut kaas = vec![0.0; data.len() / channels as usize];

                    let settings = &context.settings;
                    //write stream data here
                    let mut t = time;

                    for (wtf_i, frame) in data.chunks_exact_mut(channels as usize).enumerate() {
                        if ((t - last_time) - sample_duration).abs() > 0.0000000001 {
                            println!("Timing error! t: {}, last_time: {}, sample_duration: {}, diff: {}, err: {}",
                            t,
                            last_time, sample_duration, t - last_time, ((t - last_time) - sample_duration).abs());
                        }
                        last_time = t;

                        let mut value = play_music(t, &mut music);

                        if start_jump_sound {
                            start_jump_sound = false;
                            play_jump_sound = true;
                            t0_jump_sound = t;
                        }

                        if play_jump_sound {
                            value += 0.5 * play_sfx(t ,t0_jump_sound, &soundeffects.jump)
                        }

                        if start_death_sound {
                            start_death_sound = false;
                            play_death_sound = true;
                            t0_death_sound = t;
                        }

                        if play_death_sound {
                            value += 0.5 * play_sfx(t ,t0_death_sound, &soundeffects.death)
                        }

                        if start_footstep_sound {
                            start_footstep_sound = false;
                            play_footstep_sound = true;
                            t0_footstep_sound = t;
                        }

                        if stop_footstep_sound {
                            stop_footstep_sound = false;
                            play_footstep_sound = false;
                        }

                        if play_footstep_sound {
                            value += 0.5 * play_sfx(t ,t0_footstep_sound, &soundeffects.footstep)
                        }

                        for (i, note_played ) in piano_notes.iter().enumerate(){
                            if *note_played {
                               value += 0.5 * square_wave(t, 55. * 1.05946309436_f64.powi(i as i32 - 9));
                            }
                        }

                        // normalize output
                        let value = value as f32;
                        let value = value * settings.volume;

                        if value.abs() > max_value {
                            max_value = value.abs();
                            if max_value > 1.0 {
                                println!("WARNING: audio amplitude greater than 1");
                                println!("\tnormalizing amplitude from now on");
                            }
                        }

                        let value = if max_value > 1.0 {
                            value / max_value
                        } else {
                            value
                        };

                        // left and right channel
                        frame[0] = value; // * (1.0 - settings.panning).min(0.5) * 2.0;
                        frame[1] = value; // * (settings.panning).min(0.5) * 2.0;

                        // Output same audio for all other channels for now
                        for sample in frame[2..].iter_mut() {
                            *sample = value;
                        }

                        t += sample_duration;
                    }

                    time += chunk_time;
                    assert!((t - time).abs() < 0.00001);
                },
                move |err| {
                    //deal with errors I guess
                    panic!("err: {:?}", err);
                },
                None,
            )
            .unwrap();
        stream.play().unwrap();

        Self {
            _host: host,
            _device: device,
            _stream: stream,
            config: supported_config,

            settings,

            settings_sender,
            shit_recv,
            key_sender,
            sfx_sender,
        }
    }
}

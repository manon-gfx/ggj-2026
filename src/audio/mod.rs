use glam::*;
use std::sync::mpsc::{Sender, channel};

pub mod notes;
pub mod sound;
use sound::{play_music, play_sfx};

use crate::audio::sound::SoundTypes;
use crate::audio::sound::sawtooth_wave;
use crate::game::Key;

#[derive(Clone)]
struct AudioSettings {
    volume: f32,
}
impl Default for AudioSettings {
    fn default() -> Self {
        Self { volume: 0.5 }
    }
}

pub(crate) struct Audio {
    _host: cpal::Host,     //raii
    _device: cpal::Device, //raii
    _stream: cpal::Stream, //raii

    pub key_sender: Sender<(Key, bool)>,
    pub sfx_sender: Sender<(SoundTypes, bool)>,
    pub color_mask_sender: Sender<UVec3>,
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
                config.sample_format() == cpal::SampleFormat::F32 && config.channels() == 2
            })
            .expect("Failed to find a suitable audio format");
        let supported_config = if let Some(cfg) = supported_config.try_with_sample_rate(48000) {
            cfg
        } else if let Some(cfg) = supported_config.try_with_sample_rate(441000) {
            cfg
        } else if let Some(cfg) = supported_config.try_with_sample_rate(22050) {
            cfg
        } else {
            supported_config.with_max_sample_rate()
        };

        let sample_format = supported_config.sample_format();
        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let mut config: cpal::StreamConfig = supported_config.clone().into();
        config.buffer_size = cpal::BufferSize::Fixed(512); // ~11.6ms latency at 44.1kHz

        assert!(sample_format == cpal::SampleFormat::F32);

        let settings: AudioSettings = AudioSettings::default();
        let (key_sender, key_recv) = channel();
        let (sfx_sender, sfx_recv) = channel();
        let (color_mask_sender, color_mask_recv) = channel();

        struct StreamContext {
            settings: AudioSettings,
        }

        let context = StreamContext {
            settings: settings.clone(),
        };

        let mut last_time = 0.0;
        let mut time = 0.0;
        let mut piano_notes: [bool; 17] = [false; 17];
        let mut max_value: f32 = 0.0;

        let mut music = sound::Music::new();
        let mut t0_music = DVec4::ZERO; // red, green, blue, death
        let mut color_mask_music = UVec3::ZERO;

        let soundeffects = sound::SoundEffects::new();
        let mut start_footstep_sound: bool = false;
        let mut stop_footstep_sound: bool = false;
        let mut start_jump_sound: bool = false;
        let mut start_pickup_sound: bool = false;
        let mut start_death_sound: bool = false;
        let mut play_footstep_sound: bool = false;
        let mut play_jump_sound: bool = false;
        let mut play_pickup_sound: bool = false;
        let mut play_death_sound: bool = false;
        let mut t0_footstep_sound: f64 = 0.0;
        let mut t0_jump_sound: f64 = 0.0;
        let mut t0_pickup_sound: f64 = 0.0;
        let mut t0_death_sound: f64 = 0.0;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    while let Ok((key, down)) = key_recv.try_recv() {
                        match key {
                            Key::MusicC3 => piano_notes[0] = down,
                            Key::MusicCs3 => piano_notes[1] = down,
                            Key::MusicD3 => piano_notes[2] = down,
                            Key::MusicDs3 => piano_notes[3] = down,
                            Key::MusicE3 => piano_notes[4] = down,
                            Key::MusicF3 => piano_notes[5] = down,
                            Key::MusicFs3 => piano_notes[6] = down,
                            Key::MusicG3 => piano_notes[7] = down,
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
                            SoundTypes::PickupSound => {
                                start_pickup_sound = play;
                            }
                            SoundTypes::DeathSound => {
                                start_death_sound = play;
                            }
                        }
                    }

                    while let Ok(color_mask) = color_mask_recv.try_recv() {
                        color_mask_music = color_mask;

                        // first time each mask is picked up
                        for i in 0..3 {
                            if t0_music[i] == 0.0 && color_mask_music[i] > 0 {
                                t0_music[i] = time + 1.;
                            }
                        }
                    }

                    let sample_duration = 1.0 / sample_rate as f64;
                    let chunk_time = (data.len() / channels as usize) as f64 / sample_rate as f64;

                    let settings = &context.settings;
                    //write stream data here
                    let mut t = time;

                    for frame in data.chunks_exact_mut(channels as usize) {
                        if ((t - last_time) - sample_duration).abs() > 0.0000000001 {
                            println!("Timing error! t: {}, last_time: {}, sample_duration: {}, diff: {}, err: {}",
                            t,
                            last_time, sample_duration, t - last_time, ((t - last_time) - sample_duration).abs());
                        }
                        last_time = t;

                        let mut value = play_music(t, &t0_music, &color_mask_music, &mut music);

                        if start_footstep_sound {
                            start_footstep_sound = false;
                            play_footstep_sound = true;
                            t0_footstep_sound = t;
                        }

                        if stop_footstep_sound {
                            stop_footstep_sound = false;
                            play_footstep_sound = false;
                        }

                        if start_jump_sound {
                            start_jump_sound = false;
                            play_jump_sound = true;
                            t0_jump_sound = t;
                        }

                        if start_pickup_sound {
                            start_pickup_sound = false;
                            play_pickup_sound = true;
                            t0_pickup_sound = t;
                        }

                        if start_death_sound {
                            start_death_sound = false;
                            play_death_sound = true;
                            t0_death_sound = t;
                            t0_music[3] = t;
                        }

                        if play_footstep_sound {
                            value += play_sfx(t, t0_footstep_sound, &soundeffects.footstep)
                        }

                        if play_jump_sound {
                            value += play_sfx(t, t0_jump_sound, &soundeffects.jump)
                        }

                        if play_pickup_sound {
                            value += play_sfx(t, t0_pickup_sound, &soundeffects.pickup)
                        }

                        if play_death_sound {
                            value += play_sfx(t, t0_death_sound, &soundeffects.death)
                        }

                        for (i, note_played ) in piano_notes.iter().enumerate(){
                            if *note_played {
                               value += 0.5 * sawtooth_wave(t, 440. * 1.05946309436_f64.powi(i as i32 - 9));
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
                    println!("Failed to initialize the audio system: {:?}", err);
                },
                None,
            )
            .unwrap();
        stream.play().unwrap();

        Self {
            _host: host,
            _device: device,
            _stream: stream,

            key_sender,
            sfx_sender,
            color_mask_sender,
        }
    }
}

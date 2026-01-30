use std::collections::HashMap;
use std::sync::{
    Arc,
    mpsc::{Receiver, Sender, channel},
};

pub mod notes;
pub mod sound;
use sound::sine;

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
            volume: 0.1,
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

fn wang_hash(seed: u32) -> u32 {
    let seed = (seed ^ 61) ^ (seed >> 16);
    let seed = seed.overflowing_mul(9).0;
    let seed = seed ^ (seed >> 4);
    let seed = seed.overflowing_mul(0x27d4eb2d).0;
    let seed = seed ^ (seed >> 15);
    seed
}

pub(crate) struct Audio {
    _host: cpal::Host,     //raii
    _device: cpal::Device, //raii
    _stream: cpal::Stream, //raii

    config: cpal::SupportedStreamConfig,

    settings: AudioSettings,

    settings_sender: Sender<AudioSettings>,

    pub shit_recv: Receiver<Vec<f32>>,
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
                if config.sample_format() == cpal::SampleFormat::F32 && config.channels() == 2 {
                    true
                } else {
                    false
                }
            })
            .expect("Failed to find a suitable audio format");
        let supported_config = supported_config.with_sample_rate(44100);

        // let supported_config = supported_configs_range
        //     .next()
        //     .unwrap()
        //     .with_max_sample_rate();
        let sample_format = supported_config.sample_format();
        dbg!(sample_format);
        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();
        let config = supported_config.clone().into();

        assert!(sample_format == cpal::SampleFormat::F32);

        let settings: AudioSettings = AudioSettings::default();
        let (settings_sender, settings_recv) = channel();

        let (shit_sender, shit_recv) = channel();

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
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
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

                        // let value = t % 10000.0;
                        // noise
                        let value = wang_hash(t.to_bits() as u32) as f64 / u32::MAX as f64;

                        // let b = dbg!((440. * t) % 1.0);
                        // let value = (440. * t) % 1.0;
                        let value = sine(t);

                        let value = value as f32;
                        let value = value * settings.volume * 0.5;

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
        }
    }
}

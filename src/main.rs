mod midi_reader;

use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::iter::Iterator;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
enum PitchEnum {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}
impl PitchEnum {
    fn base_semitone(&self) -> u8 {
        match self {
            PitchEnum::C => 0,
            PitchEnum::D => 2,
            PitchEnum::E => 4,
            PitchEnum::F => 5,
            PitchEnum::G => 7,
            PitchEnum::A => 9,
            PitchEnum::B => 11,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Accidental {
    Sharp,
    Flat,
    Natural,
}

impl Accidental {
    fn semitone_modifier(&self) -> i8 {
        match self {
            Accidental::Sharp => 1,
            Accidental::Flat => -1,
            Accidental::Natural => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Note {
    pitch: PitchEnum,
    accident: Accidental,
    octave: u8,
}

impl Note {
    fn new(pitch: PitchEnum, accident: Accidental, octave: u8) -> Self {
        Self {
            pitch,
            accident,
            octave,
        }
    }

    fn midi_note_number(&self) -> u8 {
        let semitone = self.pitch.base_semitone() as i16 + self.accident.semitone_modifier() as i16;
        let midi = ((self.octave as i16 + 1) * 12) + semitone;
        midi.clamp(0, 127) as u8
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub time: f32,
    pub note: Note,
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .partial_cmp(&other.time)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for Event {}

struct Scheduler {
    queue: BinaryHeap<std::cmp::Reverse<Event>>,
    current_note: Option<Note>,
    current_note_start_time: Option<f32>,
}

impl Scheduler {
    pub fn new<T: IntoIterator<Item = Event>>(events: T) -> Self {
        Self {
            queue: BinaryHeap::from_iter(events.into_iter().map(std::cmp::Reverse)),
            current_note: None,
            current_note_start_time: None,
        }
    }

    pub fn get_state(&mut self, time: f32) -> Option<(Note, f32)> {
        while let Some(next_event) = self.queue.peek().map(|r| r.0) {
            if time >= next_event.time {
                let ev = self.queue.pop()?.0;
                self.current_note = Some(ev.note);
                self.current_note_start_time = Some(ev.time);
            } else {
                break;
            }
        }
        self.current_note
            .map(|n| (n, self.current_note_start_time.unwrap()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device found")?;

    let mut supported_configs = device.supported_output_configs()?;
    let config: StreamConfig = supported_configs
        .next()
        .ok_or("No supported output config")?
        .with_sample_rate(48000)
        .into();

    let sample_rate_hz = config.sample_rate;
    let channels = config.channels as usize;

    let mut sf2 = File::open("/home/vinicius/github/GuitarTuner/Yamaha_XG_Sound_Set.sf2")?;
    let sound_font = Arc::new(SoundFont::new(&mut sf2)?);
    let synth_settings = SynthesizerSettings::new(sample_rate_hz as i32);
    let mut synth = Synthesizer::new(&sound_font, &synth_settings)?;

    let events = vec![
        Event {
            time: 0.0,
            note: Note::new(PitchEnum::C, Accidental::Natural, 2),
        },
        Event {
            time: 1.0,
            note: Note::new(PitchEnum::D, Accidental::Natural, 2),
        },
        Event {
            time: 2.0,
            note: Note::new(PitchEnum::E, Accidental::Natural, 2),
        },
        Event {
            time: 3.0,
            note: Note::new(PitchEnum::F, Accidental::Natural, 2),
        },
        Event {
            time: 4.0,
            note: Note::new(PitchEnum::G, Accidental::Natural, 2),
        },
        Event {
            time: 5.0,
            note: Note::new(PitchEnum::A, Accidental::Natural, 2),
        },
        Event {
            time: 6.0,
            note: Note::new(PitchEnum::B, Accidental::Natural, 2),
        },
        Event {
            time: 7.0,
            note: Note::new(PitchEnum::C, Accidental::Natural, 3),
        },
    ];
    let mut scheduler = Scheduler::new(events);

    let mut time = 0.0f32;
    let dt = 1.0f32 / sample_rate_hz as f32;
    let mut current_midi: Option<u8> = None;

    let mut l = [0.0f32; 1];
    let mut r = [0.0f32; 1];

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                if let Some((note, _start)) = scheduler.get_state(time) {
                    let midi = note.midi_note_number();
                    if current_midi != Some(midi) {
                        if let Some(prev) = current_midi {
                            synth.note_off(0, prev as i32);
                        }
                        synth.note_on(0, midi as i32, 100);
                        current_midi = Some(midi);
                    }
                }

                synth.render(&mut l, &mut r);
                let sample_l = l[0];
                let sample_r = r[0];

                if channels >= 2 {
                    frame[0] = sample_l;
                    frame[1] = sample_r;
                    for sample in frame.iter_mut().skip(2) {
                        *sample = (sample_l + sample_r) * 0.5;
                    }
                } else {
                    frame[0] = (sample_l + sample_r) * 0.5;
                }

                time += dt;
            }
        },
        move |_err| {},
        None,
    )?;

    stream.play()?;
    loop {
        std::thread::park();
    }
}

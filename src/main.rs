use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use std::collections::BinaryHeap;
use std::iter::Iterator;
use std::{arch::x86_64::_MM_EXCEPT_OVERFLOW, cmp::Ordering};
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
        Note {
            pitch,
            accident,
            octave,
        }
    }
    fn frequency(&self) -> f32 {
        let semitone = self.pitch.base_semitone() as i16 + self.accident.semitone_modifier() as i16;
        let midi_index = ((self.octave as i16 + 1) * 12) + semitone;
        440.0 * f32::powf(2.0, (midi_index as f32 - 69.0) / 12.0)
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
            queue: BinaryHeap::from_iter(events.into_iter().map(|event| std::cmp::Reverse(event))),
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

//pub fn wave(time:f32,amplitude:f32,d)

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("Not output device found")?;
    let mut supported_configs_range = device.supported_output_configs()?;
    let config: StreamConfig = supported_configs_range
        .next()
        .ok_or("")?
        .with_max_sample_rate()
        .into();
    let mut time = 0.0f32;
    let pi = std::f32::consts::PI;
    let sample_rate = config.sample_rate as f32;
    let channels = config.channels as usize;

    let events = vec![
        Event {
            time: 0.0,
            note: Note::new(PitchEnum::C, Accidental::Natural, 4),
        },
        Event {
            time: 1.0,
            note: Note::new(PitchEnum::D, Accidental::Natural, 4),
        },
        Event {
            time: 2.0,
            note: Note::new(PitchEnum::E, Accidental::Natural, 4),
        },
        Event {
            time: 3.0,
            note: Note::new(PitchEnum::F, Accidental::Natural, 4),
        },
        Event {
            time: 4.0,
            note: Note::new(PitchEnum::G, Accidental::Natural, 4),
        },
        Event {
            time: 5.0,
            note: Note::new(PitchEnum::A, Accidental::Natural, 4),
        },
        Event {
            time: 6.0,
            note: Note::new(PitchEnum::B, Accidental::Natural, 4),
        },
        Event {
            time: 7.0,
            note: Note::new(PitchEnum::C, Accidental::Natural, 5),
        },
    ];
    let mut scheduler = Scheduler::new(events);
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let (note, start_time) = scheduler.get_state(time).unwrap();
                let rel_time = time - start_time;
                let attack = rel_time;
                let value = attack
                    * f32::sin(2.0 * pi * note.frequency() * rel_time)
                    * f32::exp(-(5.0 * rel_time));
                for sample in frame.iter_mut() {
                    *sample = value;
                }
                time += 1.0 / sample_rate;
            }
        },
        move |err| {},
        None,
    )?;

    stream.play()?;
    loop {
        std::thread::park();
    }
    return Ok(());
}

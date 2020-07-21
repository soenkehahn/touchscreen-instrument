pub mod audio_player;
pub mod generator;
pub mod hammond;
pub mod logger;
pub mod midi;
pub mod midi_controller;
pub mod midi_player;
pub mod wave_form;

use crate::areas::note_event_source::NoteEventSource;

const TAU: f32 = ::std::f32::consts::PI * 2.0;

pub trait Player {
    fn consume(&self, note_event_source: NoteEventSource);
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoteEvent {
    NoteOff { slot: usize },
    NoteOn { slot: usize, frequency: f32 },
}

impl NoteEvent {
    fn get_slot(&self) -> usize {
        match self {
            NoteEvent::NoteOff { slot } => *slot,
            NoteEvent::NoteOn { slot, .. } => *slot,
        }
    }
}

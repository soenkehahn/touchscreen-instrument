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

pub const POLYPHONY: usize = 20;

pub trait Player {
    fn consume(&self, note_event_source: NoteEventSource);
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoteEvent {
    NoteOff { slot: usize },
    NoteOn { slot: usize, frequency: f32 },
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn mk_voices(note_ons: Vec<(usize, NoteEvent)>) -> [NoteEvent; POLYPHONY] {
        let mut result = [NoteEvent::NoteOff { slot: 0 }; POLYPHONY];
        for (i, note) in note_ons {
            result[i] = note;
        }
        result
    }
}

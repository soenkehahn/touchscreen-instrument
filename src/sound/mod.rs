pub mod audio_player;
pub mod generator;
pub mod midi;
pub mod midi_player;

use areas::note_event_source::NoteEventSource;

pub trait Player {
    fn consume(&self, note_event_source: NoteEventSource);
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoteEvent {
    NoteOff,
    NoteOn(f32),
}

impl Default for NoteEvent {
    fn default() -> NoteEvent {
        NoteEvent::NoteOff
    }
}

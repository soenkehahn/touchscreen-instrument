pub mod audio_player;
pub mod generator;
pub mod midi_player;

use areas::NoteEvents;

pub trait Player {
    fn consume(&self, note_events: NoteEvents);
}

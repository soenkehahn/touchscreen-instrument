use super::Player;
use areas::NoteEvents;

pub struct MidiPlayer;

impl MidiPlayer {
    pub fn new() -> MidiPlayer {
        MidiPlayer
    }
}

impl Player for MidiPlayer {
    fn consume(&self, note_events: NoteEvents) {
        for event in note_events {
            println!("{:?}", event);
        }
    }
}

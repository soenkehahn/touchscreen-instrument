pub mod audio_player;
pub mod generator;
pub mod midi;
pub mod midi_player;

use areas::NoteEvents;
use evdev::Slots;

pub trait Player {
    fn consume(&self, note_events: NoteEvents);
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoteEvent {
    NoteOff,
    NoteOn(f32),
}

impl NoteEvent {
    pub fn get_first_note_on(slots: Slots<NoteEvent>) -> NoteEvent {
        for event in slots.into_iter() {
            match event {
                NoteEvent::NoteOn(_) => {
                    return *event;
                }
                _ => {}
            }
        }
        NoteEvent::NoteOff
    }
}

impl Default for NoteEvent {
    fn default() -> NoteEvent {
        NoteEvent::NoteOff
    }
}

#[cfg(test)]
mod test {
    use super::NoteEvent::*;
    use super::*;

    mod note_event {
        use super::*;

        mod get_first_note_on {
            use super::*;

            #[test]
            fn returns_the_first_slot_if_its_a_note_on() {
                let mut slots = [NoteOff; 10];
                slots[0] = NoteOn(200.0);
                assert_eq!(NoteEvent::get_first_note_on(slots), NoteOn(200.0));
            }

            #[test]
            fn skips_note_offs() {
                let mut slots = [NoteOff; 10];
                slots[1] = NoteOn(200.0);
                assert_eq!(NoteEvent::get_first_note_on(slots), NoteOn(200.0));
            }

            #[test]
            fn returns_note_off_if_all_slots_are_note_offs() {
                let slots = [NoteOff; 10];
                assert_eq!(NoteEvent::get_first_note_on(slots), NoteOff);
            }
        }
    }
}

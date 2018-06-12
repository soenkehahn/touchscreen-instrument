extern crate jack;

use super::Player;
use areas::{NoteEvent, NoteEvents};
use jack::*;
use sound::midi::frequency_to_midi;
use std::sync::mpsc::{channel, Receiver, Sender};
use {get_binary_name, ErrorString};

pub struct MidiPlayer {
    _active_client: AsyncClient<(), MidiProcessHandler>,
    sender: Sender<NoteEvent>,
}

impl MidiPlayer {
    pub fn new() -> Result<MidiPlayer, ErrorString> {
        let (sender, receiver) = channel();
        let (client, _status) =
            jack::Client::new(&get_binary_name()?, jack::ClientOptions::NO_START_SERVER)?;
        let port = client.register_port("output", MidiOut)?;
        let active_client = client.activate_async(
            (),
            MidiProcessHandler {
                port,
                receiver,
                midi_converter: MidiConverter::new(),
            },
        )?;
        Ok(MidiPlayer {
            _active_client: active_client,
            sender,
        })
    }
}

impl Player for MidiPlayer {
    fn consume(&self, note_events: NoteEvents) {
        for slots in note_events {
            match self.sender.send(NoteEvent::get_first_note_on(slots)) {
                Ok(()) => {}
                Err(e) => eprintln!("MidiPlayer.consume: error: {:?}", e),
            }
        }
    }
}

struct MidiProcessHandler {
    port: Port<MidiOut>,
    receiver: Receiver<NoteEvent>,
    midi_converter: MidiConverter,
}

impl ProcessHandler for MidiProcessHandler {
    fn process(&mut self, _client: &Client, scope: &ProcessScope) -> Control {
        let mut writer = self.port.writer(scope);
        for note_event in self.receiver.try_iter() {
            self.midi_converter.to_midi(note_event, |raw_midi| {
                let result = writer.write(&raw_midi);
                match result {
                    Ok(()) => {}
                    Err(e) => eprintln!("MidiProcessHandler.process: error: {:?}", e),
                }
            });
        }
        Control::Continue
    }
}

struct MidiConverter {
    active_note: Option<u8>,
}

impl MidiConverter {
    fn new() -> MidiConverter {
        MidiConverter { active_note: None }
    }

    fn to_midi<F>(&mut self, note_event: NoteEvent, mut callback: F)
    where
        F: FnMut(RawMidi),
    {
        #[inline]
        fn send_midi<F: FnMut(RawMidi)>(callback: &mut F, bytes: [u8; 3]) {
            callback(RawMidi {
                time: 0,
                bytes: &bytes,
            });
        };

        match (self.active_note, note_event) {
            (None, NoteEvent::NoteOn(frequency)) => {
                let midi_note = frequency_to_midi(frequency);
                send_midi(&mut callback, [0b10010000, midi_note, 127]);
                self.active_note = Some(midi_note);
            }
            (Some(midi_note), NoteEvent::NoteOff) => {
                send_midi(&mut callback, [0b10000000, midi_note, 0]);
                self.active_note = None;
            }
            (Some(old_midi_note), NoteEvent::NoteOn(frequency)) => {
                let new_midi_note = frequency_to_midi(frequency);
                if old_midi_note != new_midi_note {
                    send_midi(&mut callback, [0b10000000, old_midi_note, 0]);
                    send_midi(&mut callback, [0b10010000, new_midi_note, 127]);
                    self.active_note = Some(new_midi_note);
                }
            }
            (None, NoteEvent::NoteOff) => {}
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod midi_converter {
        use self::NoteEvent::*;
        use super::*;
        use sound::midi::midi_to_frequency;

        fn make_midi(bytes: &'static [u8]) -> RawMidi<'static> {
            RawMidi { time: 0, bytes }
        }

        fn expect_raw_midi(events: Vec<NoteEvent>, expecteds: Vec<RawMidi>) {
            let mut converter = MidiConverter::new();
            let mut result = vec![];
            for note_event in events.into_iter() {
                converter.to_midi(note_event, |raw_midi| {
                    result.push(format!("{:?}", raw_midi.bytes));
                });
            }
            let expected_as_strings: Vec<String> = expecteds
                .into_iter()
                .map(|x| format!("{:?}", x.bytes))
                .collect();
            assert_eq!(result, expected_as_strings);
        }

        #[test]
        fn converts_note_on_events() {
            expect_raw_midi(vec![NoteOn(440.0)], vec![make_midi(&[0b10010000, 69, 127])]);
        }

        #[test]
        fn converts_other_notes_correctly() {
            expect_raw_midi(
                vec![NoteOn(midi_to_frequency(60))],
                vec![make_midi(&[0b10010000, 60, 127])],
            );
        }

        #[test]
        fn converts_note_off_events_correctly() {
            expect_raw_midi(
                vec![NoteOn(midi_to_frequency(57)), NoteOff],
                vec![
                    make_midi(&[0b10010000, 57, 127]),
                    make_midi(&[0b10000000, 57, 0]),
                ],
            );
        }

        #[test]
        fn two_consecutive_note_off_events_trigger_only_one_note_off() {
            expect_raw_midi(
                vec![NoteOn(midi_to_frequency(57)), NoteOff, NoteOff],
                vec![
                    make_midi(&[0b10010000, 57, 127]),
                    make_midi(&[0b10000000, 57, 0]),
                ],
            );
        }

        #[test]
        fn two_consecutive_note_on_events_trigger_a_note_off_in_between() {
            expect_raw_midi(
                vec![NoteOn(midi_to_frequency(57)), NoteOn(midi_to_frequency(60))],
                vec![
                    make_midi(&[0b10010000, 57, 127]),
                    make_midi(&[0b10000000, 57, 0]),
                    make_midi(&[0b10010000, 60, 127]),
                ],
            );
        }

        #[test]
        fn two_consecutive_note_on_events_of_the_same_pitch_trigger_only_one_event() {
            expect_raw_midi(
                vec![NoteOn(midi_to_frequency(60)), NoteOn(midi_to_frequency(60))],
                vec![make_midi(&[0b10010000, 60, 127])],
            );
        }

        #[test]
        fn two_consecutive_note_on_events_leave_the_converter_in_a_valid_state() {
            expect_raw_midi(
                vec![
                    NoteOn(midi_to_frequency(57)),
                    NoteOn(midi_to_frequency(60)),
                    NoteOff,
                ],
                vec![
                    make_midi(&[0b10010000, 57, 127]),
                    make_midi(&[0b10000000, 57, 0]),
                    make_midi(&[0b10010000, 60, 127]),
                    make_midi(&[0b10000000, 60, 0]),
                ],
            );
        }
    }
}

#![allow(clippy::needless_range_loop)]

use super::Player;
use crate::areas::note_event_source::NoteEventSource;
use crate::sound::midi::frequency_to_midi;
use crate::sound::NoteEvent;
use crate::utils::Slots;
use crate::{get_binary_name, ErrorString};
use jack::*;
use skipchannel::*;

pub struct MidiPlayer {
    _active_client: AsyncClient<(), MidiProcessHandler>,
    sender: Sender<Slots<NoteEvent>>,
}

impl MidiPlayer {
    pub fn new() -> Result<MidiPlayer, ErrorString> {
        let (sender, receiver) = skipchannel();
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
    fn consume(&self, note_event_source: NoteEventSource) {
        for slots in note_event_source {
            self.sender.send(slots)
        }
    }
}

struct MidiProcessHandler {
    port: Port<MidiOut>,
    receiver: Receiver<Slots<NoteEvent>>,
    midi_converter: MidiConverter,
}

impl ProcessHandler for MidiProcessHandler {
    fn process(&mut self, _client: &Client, scope: &ProcessScope) -> Control {
        let mut writer = self.port.writer(scope);
        match self.receiver.recv() {
            None => {}
            Some(note_event) => self.midi_converter.connect(note_event, |raw_midi| {
                let result = writer.write(&raw_midi);
                match result {
                    Ok(()) => {}
                    Err(e) => eprintln!("MidiProcessHandler.process: error: {:?}", e),
                }
            }),
        }
        Control::Continue
    }
}

struct MidiConverter {
    active_notes: Slots<Option<u8>>,
}

impl MidiConverter {
    fn new() -> MidiConverter {
        MidiConverter {
            active_notes: [None; 10],
        }
    }

    fn connect<F>(&mut self, slots: Slots<NoteEvent>, mut callback: F)
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

        for i in 0..10 {
            match (self.active_notes[i], slots[i]) {
                (None, NoteEvent::NoteOn(frequency)) => {
                    let midi_note = frequency_to_midi(frequency);
                    send_midi(&mut callback, [0b1001_0000, midi_note, 127]);
                    self.active_notes[i] = Some(midi_note);
                }
                (Some(midi_note), NoteEvent::NoteOff) => {
                    send_midi(&mut callback, [0b1000_0000, midi_note, 0]);
                    self.active_notes[i] = None;
                }
                (Some(old_midi_note), NoteEvent::NoteOn(frequency)) => {
                    let new_midi_note = frequency_to_midi(frequency);
                    if old_midi_note != new_midi_note {
                        send_midi(&mut callback, [0b1000_0000, old_midi_note, 0]);
                        send_midi(&mut callback, [0b1001_0000, new_midi_note, 127]);
                        self.active_notes[i] = Some(new_midi_note);
                    }
                }
                (None, NoteEvent::NoteOff) => {}
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod midi_converter {
        use super::*;
        use crate::areas::note_event_source::test::from_single;
        use crate::sound::midi::midi_to_frequency;
        use NoteEvent::*;

        fn make_midi(bytes: &'static [u8]) -> RawMidi<'static> {
            RawMidi { time: 0, bytes }
        }

        fn expect_raw_midi(events: Vec<NoteEvent>, expecteds: Vec<RawMidi>) {
            let mut converter = MidiConverter::new();
            let mut result = vec![];
            for note_event in events.into_iter() {
                converter.connect(from_single(note_event), |raw_midi| {
                    result.push(format!("{:?}", raw_midi.bytes));
                });
            }
            let expected_as_strings: Vec<String> = expecteds
                .into_iter()
                .map(|x| format!("{:?}", x.bytes))
                .collect();
            assert_eq!(result, expected_as_strings);
        }

        fn expect_raw_midi_poly(events: Vec<Vec<NoteEvent>>, expecteds: Vec<RawMidi>) {
            let mut converter = MidiConverter::new();
            let mut result = vec![];
            for note_events in events.into_iter() {
                let mut slots = [NoteOff; 10];
                for (i, note_event) in note_events.into_iter().enumerate() {
                    slots[i] = note_event;
                }
                converter.connect(slots, |raw_midi| {
                    result.push(format!("{:?}", raw_midi.bytes));
                });
            }
            let expected_as_strings: Vec<String> = expecteds
                .into_iter()
                .map(|x| format!("{:?}", x.bytes))
                .collect();
            assert_eq!(result, expected_as_strings);
        }

        mod monophony {
            use super::*;

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

        mod polyphony {
            use super::*;

            #[test]
            fn allows_to_play_two_notes_simultaneously() {
                expect_raw_midi_poly(
                    vec![
                        vec![NoteOn(midi_to_frequency(60)), NoteOff],
                        vec![NoteOn(midi_to_frequency(60)), NoteOn(midi_to_frequency(62))],
                    ],
                    vec![
                        make_midi(&[0b10010000, 60, 127]),
                        make_midi(&[0b10010000, 62, 127]),
                    ],
                );
            }

            #[test]
            fn handles_overlapping_legato_melodies_correctly() {
                expect_raw_midi_poly(
                    vec![
                        vec![NoteOn(midi_to_frequency(60)), NoteOff],
                        vec![NoteOn(midi_to_frequency(60)), NoteOn(midi_to_frequency(62))],
                        vec![NoteOff, NoteOn(midi_to_frequency(62))],
                        vec![NoteOff, NoteOff],
                    ],
                    vec![
                        make_midi(&[0b10010000, 60, 127]),
                        make_midi(&[0b10010000, 62, 127]),
                        make_midi(&[0b10000000, 60, 0]),
                        make_midi(&[0b10000000, 62, 0]),
                    ],
                );
            }

            #[test]
            fn handles_note_offs_for_temporary_additional_notes_correctly() {
                expect_raw_midi_poly(
                    vec![
                        vec![NoteOn(midi_to_frequency(60)), NoteOff],
                        vec![NoteOn(midi_to_frequency(60)), NoteOn(midi_to_frequency(62))],
                        vec![NoteOn(midi_to_frequency(60)), NoteOff],
                        vec![NoteOff, NoteOff],
                    ],
                    vec![
                        make_midi(&[0b10010000, 60, 127]),
                        make_midi(&[0b10010000, 62, 127]),
                        make_midi(&[0b10000000, 62, 0]),
                        make_midi(&[0b10000000, 60, 0]),
                    ],
                );
            }

            #[test]
            fn does_not_rely_on_the_first_slot_being_used() {
                expect_raw_midi_poly(
                    vec![vec![NoteOff, NoteOn(midi_to_frequency(60))]],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }

            #[test]
            fn uses_the_last_slot() {
                let mut slots: Slots<NoteEvent> = [NoteOff; 10];
                slots[slots.len() - 1] = NoteOn(midi_to_frequency(60));
                expect_raw_midi_poly(
                    vec![slots.to_vec()],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }
        }
    }
}

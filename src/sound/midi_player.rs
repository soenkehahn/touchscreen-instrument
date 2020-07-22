#![allow(clippy::needless_range_loop)]

use super::Player;
use crate::areas::note_event_source::NoteEventSource;
use crate::sound::midi::frequency_to_midi;
use crate::sound::{NoteEvent, POLYPHONY};
use crate::{get_binary_name, ErrorString};
use jack::*;
use skipchannel::*;

pub struct MidiPlayer {
    _active_client: AsyncClient<(), MidiProcessHandler>,
    sender: Sender<[NoteEvent; POLYPHONY]>,
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
    receiver: Receiver<[NoteEvent; POLYPHONY]>,
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
    voices: [Option<u8>; POLYPHONY],
}

impl MidiConverter {
    fn new() -> MidiConverter {
        MidiConverter {
            voices: [None; POLYPHONY],
        }
    }

    fn connect<F>(&mut self, voice_events: [NoteEvent; POLYPHONY], mut callback: F)
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

        for (voice, event) in self.voices.iter_mut().zip(voice_events.iter()) {
            match (&voice, event) {
                (None, NoteEvent::NoteOn { frequency }) => {
                    let midi_note = frequency_to_midi(*frequency);
                    send_midi(&mut callback, [0b1001_0000, midi_note, 127]);
                    *voice = Some(midi_note);
                }
                (Some(midi_note), NoteEvent::NoteOff) => {
                    send_midi(&mut callback, [0b1000_0000, *midi_note, 0]);
                    *voice = None;
                }
                (Some(old_midi_note), NoteEvent::NoteOn { frequency }) => {
                    let new_midi_note = frequency_to_midi(*frequency);
                    if *old_midi_note != new_midi_note {
                        send_midi(&mut callback, [0b1000_0000, *old_midi_note, 0]);
                        send_midi(&mut callback, [0b1001_0000, new_midi_note, 127]);
                        *voice = Some(new_midi_note);
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
        use crate::sound::midi::midi_to_frequency;
        use crate::sound::test::mk_test_voices;
        use NoteEvent::*;

        fn make_midi(bytes: &'static [u8]) -> RawMidi<'static> {
            RawMidi { time: 0, bytes }
        }

        fn expect_raw_midi(chunks: Vec<Vec<NoteEvent>>, expecteds: Vec<RawMidi>) {
            expect_raw_midi_poly(
                chunks
                    .into_iter()
                    .map(|events| events.into_iter().map(|event| (0, event)).collect())
                    .collect(),
                expecteds,
            );
        }

        fn expect_raw_midi_poly(chunks: Vec<Vec<(usize, NoteEvent)>>, expecteds: Vec<RawMidi>) {
            let mut converter = MidiConverter::new();
            let mut result = vec![];
            for events in chunks {
                converter.connect(mk_test_voices(events), |raw_midi| {
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
                expect_raw_midi(
                    vec![vec![NoteOn { frequency: 440.0 }]],
                    vec![make_midi(&[0b10010000, 69, 127])],
                );
            }

            #[test]
            fn converts_other_notes_correctly() {
                expect_raw_midi(
                    vec![vec![NoteOn {
                        frequency: midi_to_frequency(60),
                    }]],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }

            #[test]
            fn converts_note_off_events_correctly() {
                expect_raw_midi(
                    vec![
                        vec![NoteOn {
                            frequency: midi_to_frequency(57),
                        }],
                        vec![],
                    ],
                    vec![
                        make_midi(&[0b10010000, 57, 127]),
                        make_midi(&[0b10000000, 57, 0]),
                    ],
                );
            }

            #[test]
            fn two_consecutive_note_off_events_trigger_only_one_note_off() {
                expect_raw_midi(
                    vec![
                        vec![NoteOn {
                            frequency: midi_to_frequency(57),
                        }],
                        vec![],
                        vec![],
                    ],
                    vec![
                        make_midi(&[0b10010000, 57, 127]),
                        make_midi(&[0b10000000, 57, 0]),
                    ],
                );
            }

            #[test]
            fn two_consecutive_note_on_events_trigger_a_note_off_in_between() {
                expect_raw_midi(
                    vec![
                        vec![NoteOn {
                            frequency: midi_to_frequency(57),
                        }],
                        vec![NoteOn {
                            frequency: midi_to_frequency(60),
                        }],
                    ],
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
                    vec![
                        vec![NoteOn {
                            frequency: midi_to_frequency(60),
                        }],
                        vec![NoteOn {
                            frequency: midi_to_frequency(60),
                        }],
                    ],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }

            #[test]
            fn two_consecutive_note_on_events_leave_the_converter_in_a_valid_state() {
                expect_raw_midi(
                    vec![
                        vec![NoteOn {
                            frequency: midi_to_frequency(57),
                        }],
                        vec![NoteOn {
                            frequency: midi_to_frequency(60),
                        }],
                        vec![],
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
                        vec![(
                            0,
                            NoteOn {
                                frequency: midi_to_frequency(60),
                            },
                        )],
                        vec![
                            (
                                0,
                                NoteOn {
                                    frequency: midi_to_frequency(60),
                                },
                            ),
                            (
                                1,
                                NoteOn {
                                    frequency: midi_to_frequency(62),
                                },
                            ),
                        ],
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
                        vec![(
                            0,
                            NoteOn {
                                frequency: midi_to_frequency(60),
                            },
                        )],
                        vec![
                            (
                                0,
                                NoteOn {
                                    frequency: midi_to_frequency(60),
                                },
                            ),
                            (
                                1,
                                NoteOn {
                                    frequency: midi_to_frequency(62),
                                },
                            ),
                        ],
                        vec![(
                            1,
                            NoteOn {
                                frequency: midi_to_frequency(62),
                            },
                        )],
                        vec![],
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
                        vec![(
                            0,
                            NoteOn {
                                frequency: midi_to_frequency(60),
                            },
                        )],
                        vec![
                            (
                                0,
                                NoteOn {
                                    frequency: midi_to_frequency(60),
                                },
                            ),
                            (
                                1,
                                NoteOn {
                                    frequency: midi_to_frequency(62),
                                },
                            ),
                        ],
                        vec![(
                            0,
                            NoteOn {
                                frequency: midi_to_frequency(60),
                            },
                        )],
                        vec![],
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
                    vec![vec![(
                        1,
                        NoteOn {
                            frequency: midi_to_frequency(60),
                        },
                    )]],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }

            #[test]
            fn uses_the_last_slot() {
                expect_raw_midi_poly(
                    vec![vec![(
                        POLYPHONY - 1,
                        NoteOn {
                            frequency: midi_to_frequency(60),
                        },
                    )]],
                    vec![make_midi(&[0b10010000, 60, 127])],
                );
            }
        }
    }
}

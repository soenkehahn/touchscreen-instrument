use crate::cli;
use crate::sound::wave_form::WaveForm;
use crate::sound::NoteEvent;
use crate::sound::{POLYPHONY, TAU};
use crate::utils::Slots;

pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

pub struct Generators {
    amplitude: f32,
    pub midi_controller_volume: f32,
    pub envelope: Envelope,
    pub wave_form: WaveForm,
    pub voices: Vec<VoiceState>,
}

pub const MIN_ATTACK: f32 = 0.005;
pub const MAX_ATTACK: f32 = 0.3;

pub const MIN_DECAY: f32 = 0.005;
pub const MAX_DECAY: f32 = 1.0;

pub const MIN_SUSTAIN: f32 = 0.0;
pub const MAX_SUSTAIN: f32 = 1.0;

pub const MIN_RELEASE: f32 = 0.005;
pub const MAX_RELEASE: f32 = 1.0;

impl Generators {
    pub fn new(cli_args: &cli::Args) -> Generators {
        let unit_slots: Slots<()> = [(); 10];
        let slots = unit_slots.len();
        Generators {
            amplitude: cli_args.volume / slots as f32,
            midi_controller_volume: 1.0,
            envelope: Envelope {
                attack: MIN_ATTACK,
                decay: MIN_DECAY,
                sustain: MAX_SUSTAIN,
                release: MIN_RELEASE,
            },
            wave_form: WaveForm::new(&cli_args.wave_form_config),
            voices: vec![VoiceState::default(); POLYPHONY],
        }
    }

    pub fn handle_note_events(&mut self, voice_events: [NoteEvent; POLYPHONY]) {
        for (voice, event) in self.voices.iter_mut().zip(voice_events.iter()) {
            match event {
                NoteEvent::NoteOff => voice.note_off(&self.envelope),
                NoteEvent::NoteOn(frequency) => voice.note_on(*frequency),
            }
        }
    }

    pub fn generate(&mut self, sample_rate: usize, buffer: &mut [f32]) {
        for voice in self.voices.iter_mut() {
            for sample in buffer.iter_mut() {
                voice.step(sample_rate, &self.envelope);
                match *voice {
                    VoiceState::Playing {
                        phase,
                        ref envelope_phase,
                        ..
                    } => {
                        *sample += self.wave_form.run(phase)
                            * self.amplitude
                            * self.midi_controller_volume
                            * envelope_phase.get_amplitude(&self.envelope);
                    }
                    VoiceState::Muted => {}
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvelopePhase {
    Attacking { amplitude: f32 },
    Decaying { amplitude: f32 },
    Sustaining,
    Releasing { amplitude: f32 },
}

impl EnvelopePhase {
    fn get_amplitude(&self, envelope: &Envelope) -> f32 {
        match self {
            EnvelopePhase::Attacking { amplitude } => *amplitude,
            EnvelopePhase::Decaying { amplitude } => *amplitude,
            EnvelopePhase::Sustaining => envelope.sustain,
            EnvelopePhase::Releasing { amplitude } => *amplitude,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VoiceState {
    Playing {
        frequency: f32,
        phase: f32,
        envelope_phase: EnvelopePhase,
    },
    Muted,
}

impl Default for VoiceState {
    fn default() -> VoiceState {
        VoiceState::Muted
    }
}

impl VoiceState {
    pub fn note_on(&mut self, new_frequency: f32) {
        match *self {
            VoiceState::Playing {
                ref mut frequency,
                ref mut envelope_phase,
                ..
            } => {
                *frequency = new_frequency;
                match envelope_phase {
                    EnvelopePhase::Attacking { .. } => {}
                    EnvelopePhase::Decaying { .. } => {}
                    EnvelopePhase::Sustaining => {}
                    EnvelopePhase::Releasing { .. } => {
                        *envelope_phase = EnvelopePhase::Attacking { amplitude: 0.0 };
                    }
                }
            }
            VoiceState::Muted => {
                *self = VoiceState::Playing {
                    frequency: new_frequency,
                    phase: 0.0,
                    envelope_phase: EnvelopePhase::Attacking { amplitude: 0.0 },
                };
            }
        };
    }

    pub fn note_off(&mut self, envelope: &Envelope) {
        match *self {
            VoiceState::Playing {
                ref mut envelope_phase,
                ..
            } => {
                *envelope_phase = EnvelopePhase::Releasing {
                    amplitude: envelope_phase.get_amplitude(&envelope),
                };
            }
            VoiceState::Muted => {}
        }
    }

    fn crank_phase(&mut self, sample_rate: usize) {
        match *self {
            VoiceState::Playing {
                frequency,
                ref mut phase,
                ..
            } => {
                *phase += frequency * TAU / sample_rate as f32;
                *phase %= TAU;
            }
            VoiceState::Muted => {}
        };
    }

    fn step_envelope(&mut self, sample_rate: usize, envelope: &Envelope) {
        match self {
            VoiceState::Playing {
                ref mut envelope_phase,
                ..
            } => {
                match envelope_phase {
                    EnvelopePhase::Attacking { ref mut amplitude } => {
                        *amplitude += 1.0 / (sample_rate as f32 * envelope.attack);
                        if *amplitude >= 1.0 {
                            *envelope_phase = EnvelopePhase::Decaying { amplitude: 1.0 };
                        }
                    }
                    EnvelopePhase::Decaying { ref mut amplitude } => {
                        *amplitude -=
                            (1.0 - envelope.sustain) / (sample_rate as f32 * envelope.decay);
                        if *amplitude <= envelope.sustain {
                            *envelope_phase = EnvelopePhase::Sustaining;
                        }
                    }
                    EnvelopePhase::Sustaining => {}
                    EnvelopePhase::Releasing {
                        ref mut amplitude, ..
                    } => {
                        let release_decrement =
                            envelope.sustain / (sample_rate as f32 * envelope.release);
                        let decay_decrement = if *amplitude > envelope.sustain {
                            (1.0 - envelope.sustain) / (sample_rate as f32 * envelope.decay)
                        } else {
                            0.0
                        };
                        *amplitude -= release_decrement + decay_decrement;
                        if *amplitude <= 0.0 {
                            *self = VoiceState::Muted;
                        }
                    }
                };
            }
            VoiceState::Muted => {}
        }
    }

    fn step(&mut self, sample_rate: usize, envelope: &Envelope) {
        self.crank_phase(sample_rate);
        self.step_envelope(sample_rate, envelope);
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::sound::mk_voices;

    const SAMPLE_RATE: usize = 44100;

    fn assert_close(a: f32, b: f32) {
        let epsilon = 0.004;
        if (a - b).abs() > epsilon {
            panic!(format!("assert_close: {} too far from {}", a, b));
        }
    }

    mod voice_state {
        use super::*;

        impl VoiceState {
            fn get_phase(&self) -> f32 {
                match self {
                    VoiceState::Playing { phase, .. } => *phase,
                    VoiceState::Muted => panic!("get_phase: Muted"),
                }
            }
        }

        mod step {
            use super::*;

            #[test]
            fn reaches_2_pi_after_1_second() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                let sample_rate = 100;
                for _ in 0..(sample_rate - 1) {
                    voice.step(
                        sample_rate,
                        &Envelope {
                            attack: 0.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                    );
                }
                assert_close(
                    voice.get_phase(),
                    TAU * (sample_rate - 1) as f32 / sample_rate as f32,
                );
            }

            #[test]
            fn increases_the_phase_for_one_sample() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                assert_eq!(voice.get_phase(), 0.0);
                voice.step(
                    SAMPLE_RATE,
                    &Envelope {
                        attack: 0.0,
                        decay: MIN_DECAY,
                        sustain: 1.0,
                        release: 0.0,
                    },
                );
                assert_eq!(voice.get_phase(), TAU / SAMPLE_RATE as f32);
            }

            #[test]
            fn wraps_around_at_2_pi() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                for _ in 0..SAMPLE_RATE {
                    voice.step(
                        SAMPLE_RATE,
                        &Envelope {
                            attack: 0.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                    );
                }
                assert_close(voice.get_phase(), 0.0);
            }
        }
    }

    pub mod generators {
        use super::*;

        pub fn sine_generators() -> Generators {
            Generators {
                amplitude: 1.0,
                midi_controller_volume: 1.0,
                envelope: Envelope {
                    attack: 0.0,
                    decay: MIN_DECAY,
                    sustain: 1.0,
                    release: 0.0,
                },
                wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE),
                voices: vec![VoiceState::default(); POLYPHONY],
            }
        }

        fn monophonic_sine_generators() -> Generators {
            Generators {
                amplitude: 1.0,
                midi_controller_volume: 1.0,
                envelope: Envelope {
                    attack: 0.0,
                    decay: MIN_DECAY,
                    sustain: 1.0,
                    release: 0.0,
                },
                wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE),
                voices: vec![VoiceState::default()],
            }
        }

        mod handle_note_events {
            use super::*;

            #[test]
            fn switches_on_the_voice_with_the_same_index_as_the_note_event() {
                for i in 0..POLYPHONY {
                    let mut generators = sine_generators();
                    let voices = {
                        let mut result = mk_voices(NoteEvent::NoteOff);
                        result[i] = NoteEvent::NoteOn(42.0);
                        result
                    };
                    generators.handle_note_events(voices);
                    let expected = {
                        let mut result = mk_voices(VoiceState::Muted);
                        result[i] = VoiceState::Playing {
                            frequency: 42.0,
                            phase: 0.0,
                            envelope_phase: EnvelopePhase::Attacking { amplitude: 0.0 },
                        };
                        result
                    };
                    assert_eq!(generators.voices, expected);
                }
            }

            #[test]
            fn switches_off_the_correct_voice() {
                for i in 0..POLYPHONY {
                    let mut generators = sine_generators();
                    let voices = {
                        let mut result = mk_voices(NoteEvent::NoteOff);
                        result[i] = NoteEvent::NoteOn(42.0);
                        result
                    };
                    generators.handle_note_events(voices);
                    generators.handle_note_events(mk_voices(NoteEvent::NoteOff));
                    generators.generate(SAMPLE_RATE, &mut [0.0]);
                    assert_eq!(generators.voices, mk_voices(VoiceState::Muted));
                }
            }
        }

        mod generate {
            use super::*;

            impl Generators {
                fn note_on(&mut self, i: usize, frequency: f32) {
                    self.voices[i].note_on(frequency);
                }

                fn note_off(&mut self, i: usize) {
                    self.voices[i].note_off(&self.envelope)
                }
            }

            #[test]
            fn new_creates_as_many_voices_as_configured() {
                let generators = Generators::new(&cli::test::args(vec![]));
                assert_eq!(generators.voices.len(), POLYPHONY);
            }

            #[test]
            fn starts_at_zero() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 1.0);
                let buffer = &mut [0.0; 10];
                generators.generate(SAMPLE_RATE, buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn generates_sine_waves() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 1.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn starts_with_phase_zero_after_pauses() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 1.0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.note_off(0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.note_on(0, 1.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 1.0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.note_on(0, 1.1);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
            }

            #[test]
            fn works_for_different_frequencies() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 300.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[8], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn allows_to_change_the_frequency_later() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 300.0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.note_on(0, 500.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(
                    buffer[0],
                    ((10.0 * 300.0 + 500.0) * TAU / SAMPLE_RATE as f32).sin()
                );
                assert_eq!(
                    buffer[1],
                    ((10.0 * 300.0 + 2.0 * 500.0) * TAU / SAMPLE_RATE as f32).sin()
                );
            }

            #[test]
            fn is_initially_muted() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        decay: MIN_DECAY,
                        sustain: 1.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|x| x.sin(), 10000),
                    voices: vec![VoiceState::default()],
                };
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn can_be_muted() {
                let mut generators = monophonic_sine_generators();
                generators.note_on(0, 1.0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.note_off(0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn allows_to_specify_the_wave_form() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        decay: MIN_DECAY,
                        sustain: 1.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|phase| phase * 5.0, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.note_on(0, 1.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_close(buffer[0], 5.0 * TAU / SAMPLE_RATE as f32);
                assert_close(buffer[1], 2.0 * 5.0 * TAU / SAMPLE_RATE as f32);
            }

            #[test]
            fn allows_to_scale_the_amplitude() {
                let mut generators = Generators {
                    amplitude: 0.25,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        decay: MIN_DECAY,
                        sustain: 1.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.note_on(0, 1.0);
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.1);
            }

            #[test]
            fn allows_to_adjust_the_controller_volume_later() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        decay: MIN_DECAY,
                        sustain: 1.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.note_on(0, 1.0);
                generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                generators.midi_controller_volume = 0.5;
                let mut buffer = [0.0; 10];
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.2);
            }

            mod envelope {
                use super::*;

                const SAMPLE_RATE: usize = 10;

                macro_rules! test_generators {
                    ($generators:expr, $expected:expr,) => {
                        test_generators!($generators, $expected)
                    };
                    ($generators:expr, $expected:expr) => {
                        let mut buffer = [0.0; 10];
                        $generators.generate(SAMPLE_RATE, &mut buffer);
                        println!("test_generators:");
                        let epsilon = 0.000001;
                        let mut close = true;
                        for (x, y) in buffer.iter().zip($expected.iter()) {
                            if (x - y).abs() > epsilon {
                                println!("{} != {}", x, y);
                                close = false;
                            } else {
                                println!("{} ~= {}", x, y);
                            }
                        }
                        assert!(close, "not close enough: {:?} and {:?}", buffer, $expected);
                    };
                }

                fn mk_generators(envelope: Envelope, wave_form: fn(f32) -> f32) -> Generators {
                    Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope,
                        wave_form: WaveForm::from_function(wave_form, 10000),
                        voices: vec![VoiceState::default()],
                    }
                }

                #[test]
                fn allows_to_specify_an_attack_time() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.5,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                        |_phase| 0.5,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [0.1, 0.2, 0.3, 0.4, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
                    );
                }

                #[test]
                fn does_not_reenter_an_attack_phase_for_subsequent_note_ons_when_playing() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.5,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                        |_phase| 0.5,
                    );
                    generators.note_on(0, 1.0);
                    generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                    generators.note_on(0, 1.0);
                    test_generators!(generators, [0.5; 10]);
                }

                #[test]
                fn does_not_restart_an_attack_phase_for_subsequent_note_ons_while_in_attack_phase()
                {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 2.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                    generators.note_on(0, 1.0);
                    let expected = {
                        let mut result = [0.0; 10];
                        for (i, cell) in result.iter_mut().enumerate() {
                            *cell = 0.5 + (i as f32 + 1.0) * 0.05;
                        }
                        result
                    };
                    test_generators!(generators, expected);
                }

                #[test]
                fn allows_to_specify_a_release_time() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.5,
                        },
                        |_phase| 0.5,
                    );
                    generators.note_on(0, 1.0);
                    generators.generate(SAMPLE_RATE, &mut [0.0; 10]);
                    generators.note_off(0);
                    test_generators!(
                        generators,
                        [0.4, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    );
                }

                #[test]
                fn allows_to_specify_a_decay_time() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.0,
                            decay: 0.5,
                            sustain: 0.0,
                            release: 0.0,
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [1.0, 0.8, 0.6, 0.4, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
                    );
                }

                #[test]
                fn allows_to_specify_a_sustain_level() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.0,
                            decay: 0.5,
                            sustain: 0.9,
                            release: 0.0,
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [1.0, 0.98, 0.96, 0.94, 0.92, 0.9, 0.9, 0.9, 0.9, 0.9],
                    );
                }

                #[test]
                fn full_adsr_test() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.5,
                            decay: 1.0,
                            sustain: 0.9,
                            release: 0.9,
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [0.2, 0.4, 0.6, 0.8, 1.0, 0.99, 0.98, 0.97, 0.96, 0.95],
                    );
                    test_generators!(
                        generators,
                        [0.94, 0.93, 0.92, 0.91, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9],
                    );
                    generators.note_off(0);
                    test_generators!(
                        generators,
                        [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0, 0.0],
                    );
                }

                #[test]
                fn stays_in_decaying_state_when_receiving_subsequent_note_on_event() {
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.0,
                            decay: 2.0,
                            sustain: 0.9,
                            release: 0.0,
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [1.0, 0.995, 0.99, 0.985, 0.98, 0.975, 0.97, 0.965, 0.96, 0.955],
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [0.95, 0.945, 0.94, 0.935, 0.93, 0.925, 0.92, 0.915, 0.91, 0.905],
                    );
                }

                #[test]
                fn combines_decay_and_release_decrements_on_note_off_events_when_in_decaying_state()
                {
                    let sustain = 0.5;
                    let decay_decrement = 0.03;
                    let release_decrement = 0.02;
                    let mut generators = mk_generators(
                        Envelope {
                            attack: 0.0,
                            decay: (1.0 - sustain) / (SAMPLE_RATE as f32 * decay_decrement),
                            sustain,
                            release: sustain / (SAMPLE_RATE as f32 * release_decrement),
                        },
                        |_phase| 1.0,
                    );
                    generators.note_on(0, 1.0);
                    test_generators!(
                        generators,
                        [1.0, 0.97, 0.94, 0.91, 0.88, 0.85, 0.82, 0.79, 0.76, 0.73],
                    );
                    generators.note_off(0);
                    test_generators!(
                        generators,
                        [0.68, 0.63, 0.58, 0.53, 0.48, 0.46, 0.44, 0.42, 0.4, 0.38],
                    );
                }
            }

            mod polyphony {
                use super::*;

                #[test]
                fn does_not_overwrite_the_buffer_when_muted() {
                    let mut generators = monophonic_sine_generators();
                    generators.note_off(0);
                    let mut buffer = [0.0; 10];
                    buffer[5] = 23.0;
                    generators.generate(SAMPLE_RATE, &mut buffer);
                    assert_eq!(buffer[5], 23.0);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer() {
                    let mut generators = Generators {
                        amplitude: 0.5,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 0.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.note_on(0, 440.0);
                    let mut buffer = [0.0; 10];
                    buffer[0] = 0.1;
                    generators.generate(10, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.5);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer_during_release() {
                    let sample_rate = 10;
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.0,
                            decay: MIN_DECAY,
                            sustain: 1.0,
                            release: 1.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    let mut buffer = [0.0; 10];
                    generators.note_on(0, 440.0);
                    generators.generate(sample_rate, &mut buffer);
                    generators.note_off(0);
                    buffer[0] = 0.1;
                    buffer[1] = 0.1;
                    generators.generate(sample_rate, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.9);
                    assert_close(buffer[1], 0.1 + 0.5 * 0.8);
                }
            }
        }
    }
}

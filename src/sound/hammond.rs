use sound::wave_form::WaveForm;

pub fn mk_hammond(harmonics: Vec<f32>) -> WaveForm {
    internal(harmonics).memoize(44100)
}

#[derive(Clone)]
struct Harmonic {
    harmonic: f32,
    volume: f32,
}

fn internal(harmonics: Vec<f32>) -> WaveForm {
    let with_harmonics: Vec<Harmonic> = harmonics
        .into_iter()
        .enumerate()
        .map(|(index, volume)| Harmonic {
            harmonic: (index + 1) as f32,
            volume,
        })
        .collect();
    WaveForm::new(move |phase| {
        let mut result = 0.0;
        for Harmonic { harmonic, volume } in with_harmonics.iter() {
            result += (phase * harmonic).sin() * volume;
        }
        result
    })
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_eq_wave<F>(a: WaveForm, b: F)
    where
        F: Fn(f32) -> f32,
    {
        let inputs = vec![0.0, 0.5, 1.0, 2.0, 2.5];
        let result: Vec<f32> = inputs.iter().map(|phase| a.run(*phase)).collect();
        let expected: Vec<f32> = inputs.iter().map(|phase| b(*phase)).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn first_harmonic_produces_sine_wave() {
        let wave_form = internal(vec![1.0]);
        assert_eq_wave(wave_form, |x| x.sin());
    }

    #[test]
    fn second_harmonic_is_an_octave() {
        let wave_form = internal(vec![0.0, 1.0]);
        assert_eq_wave(wave_form, |x| (x * 2.0).sin());
    }

    #[test]
    fn sine_waves_are_summed_up() {
        let wave_form = internal(vec![1.0, 1.0]);
        assert_eq_wave(wave_form, |x| x.sin() + (x * 2.0).sin());
    }

    #[test]
    fn harmonic_volumes_can_be_fractions() {
        let wave_form = internal(vec![0.5]);
        assert_eq_wave(wave_form, |x| x.sin() * 0.5);
    }

    #[test]
    fn supports_at_least_8_harmonics() {
        let harmonics = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let wave_form = internal(harmonics);
        assert_eq_wave(wave_form, |x| (x * 8.0).sin());
    }
}

use crate::sound::wave_form::WaveForm;

#[derive(Clone)]
struct Harmonic {
    harmonic: f32,
    volume: f32,
}

pub fn mk_hammond(harmonics: Vec<f32>, size: usize) -> WaveForm {
    let with_harmonics: Vec<Harmonic> = harmonics
        .into_iter()
        .enumerate()
        .map(|(index, volume)| Harmonic {
            harmonic: (index + 1) as f32,
            volume,
        })
        .collect();
    WaveForm::from_function(
        move |phase| {
            let mut result = 0.0;
            for Harmonic { harmonic, volume } in with_harmonics.iter() {
                result += (phase * harmonic).sin() * volume;
            }
            result
        },
        size,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_eq_wave_form<F>(a: WaveForm, b: F)
    where
        F: Fn(f32) -> f32,
    {
        let b_wave_form = WaveForm::from_function(b, a.table.len());
        assert_eq!(a, b_wave_form);
    }

    #[test]
    fn first_harmonic_produces_sine_wave() {
        let wave_form = mk_hammond(vec![1.0], 10000);
        assert_eq_wave_form(wave_form, |x| x.sin());
    }

    #[test]
    fn second_harmonic_is_an_octave() {
        let wave_form = mk_hammond(vec![0.0, 1.0], 10000);
        assert_eq_wave_form(wave_form, |x| (x * 2.0).sin());
    }

    #[test]
    fn sine_waves_are_summed_up() {
        let wave_form = mk_hammond(vec![1.0, 1.0], 10000);
        assert_eq_wave_form(wave_form, |x| x.sin() + (x * 2.0).sin());
    }

    #[test]
    fn harmonic_volumes_can_be_fractions() {
        let wave_form = mk_hammond(vec![0.5], 10000);
        assert_eq_wave_form(wave_form, |x| x.sin() * 0.5);
    }

    #[test]
    fn supports_at_least_8_harmonics() {
        let harmonics = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let wave_form = mk_hammond(harmonics, 10000);
        assert_eq_wave_form(wave_form, |x| (x * 8.0).sin());
    }
}

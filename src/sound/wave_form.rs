use crate::sound::hammond::mk_hammond;
use crate::sound::midi_controller::HarmonicsState;
use crate::sound::TAU;
use std::fmt::Debug;

#[derive(Clone, PartialEq)]
pub struct WaveForm {
    pub table: Vec<f32>,
}

impl Debug for WaveForm {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "WaveForm(<table>)")
    }
}

impl WaveForm {
    const TABLE_SIZE: usize = 44100;

    pub fn new(harmonics_state: &HarmonicsState) -> WaveForm {
        mk_hammond(&harmonics_state.harmonics, WaveForm::TABLE_SIZE)
    }

    pub fn from_function<F: Fn(f32) -> f32>(function: F, size: usize) -> WaveForm {
        let mut table: Vec<f32> = vec![0.0; size];
        for (i, cell) in table.iter_mut().enumerate() {
            let x = i as f32 * TAU / size as f32;
            *cell = function(x);
        }
        WaveForm { table }
    }

    pub fn run(&self, phase: f32) -> f32 {
        let size = self.table.len();
        let i = (((phase / TAU) * size as f32).round() as usize) % size;
        self.table[i]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allows_to_implement_rect_waves() {
        let wave_form = WaveForm::from_function(
            |phase: f32| if phase < TAU / 2.0 { -1.0 } else { 1.0 },
            10000,
        );
        assert_eq!(wave_form.run(0.0), -1.0);
    }

    #[test]
    fn allows_to_use_closures() {
        let closed_over = 42.0;
        let wave_form = WaveForm::from_function(move |phase: f32| phase + closed_over, 10000);
        assert_eq!(wave_form.run(0.0), 42.0);
    }

    #[test]
    fn implements_debug() {
        let wave_form = WaveForm::from_function(move |phase: f32| phase, 10000);
        assert_eq!(format!("{:?}", wave_form), "WaveForm(<table>)");
    }

    fn assert_eq_function(function: fn(f32) -> f32, inputs: Vec<f32>) {
        let expected: Vec<f32> = inputs.iter().map(|x| function(*x)).collect();
        let wave_form = WaveForm::from_function(function, 10000);
        let result: Vec<f32> = inputs.iter().map(|x| wave_form.run(*x)).collect();
        let mut success = true;
        for (a, b) in expected.iter().zip(&result) {
            if (a - b).abs() > 0.001 {
                success = false;
            }
        }
        if !success {
            panic!(format!("too far apart:\n\t{:?}\n\t{:?}", expected, result));
        }
    }

    #[test]
    fn behaves_like_the_input_function() {
        assert_eq_function(|x: f32| x * 2.0, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn behaves_like_the_input_function_for_fractions() {
        assert_eq_function(|x: f32| x * 2.0, vec![0.1, 0.2]);
    }

    #[test]
    fn does_not_crash_for_values_outside_the_range() {
        let wave_form = WaveForm::from_function(|x: f32| x * 2.0, 10000);
        for x in &[0.0, -0.1, -TAU, -(TAU + 0.1), TAU, TAU + 0.1, 100000.0] {
            wave_form.run(*x);
        }
    }

    #[test]
    fn does_discretize_the_input_function() {
        let function = |x: f32| x * 2.0;
        let wave_form = WaveForm::from_function(function, 10);
        assert_eq!(wave_form.run(0.11 * TAU), 0.1 * 2.0 * TAU, "rounds down");
        assert_eq!(wave_form.run(0.09 * TAU), 0.1 * 2.0 * TAU, "rounds up");
    }
}

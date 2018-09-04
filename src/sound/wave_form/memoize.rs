use sound::wave_form::WaveForm;
use sound::TAU;

impl WaveForm {
    pub fn memoize(self, size: usize) -> WaveForm {
        let mut table: Vec<f32> = vec![0.0; size];
        for i in 0..size {
            let x = i as f32 * TAU / size as f32;
            table[i] = self.run(x);
        }
        WaveForm::new(move |x: f32| {
            let i = (((x / TAU) * size as f32).round() as usize) % size;
            table[i]
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_eq_function(function: fn(f32) -> f32, inputs: Vec<f32>) {
        let memoized = WaveForm::new(function).memoize(10000);
        let result: Vec<f32> = inputs
            .clone()
            .into_iter()
            .map(|x| memoized.run(x))
            .collect();
        let expected: Vec<f32> = inputs.into_iter().map(function).collect();
        let mut success = true;
        for (a, b) in expected.clone().into_iter().zip(result.clone()) {
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
        let memoized = WaveForm::new(|x: f32| x * 2.0).memoize(10000);
        let _ = vec![0.0, -0.1, -TAU, -(TAU + 0.1), TAU, TAU + 0.1, 100000.0]
            .into_iter()
            .map(|x| memoized.run(x))
            .collect::<Vec<f32>>();
    }

    #[test]
    fn does_discretize_the_input_function() {
        let function = |x: f32| x * 2.0;
        let memoized = WaveForm::new(function).memoize(10);
        assert_eq!(memoized.run(0.11 * TAU), 0.1 * 2.0 * TAU, "rounds down");
        assert_eq!(memoized.run(0.09 * TAU), 0.1 * 2.0 * TAU, "rounds up");
    }
}

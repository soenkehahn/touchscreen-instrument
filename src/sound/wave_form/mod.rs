mod memoize;

use std::fmt::Debug;

#[derive(Clone)]
pub struct WaveForm {
    inner: Box<RunAndClone + Send>,
}

impl Debug for WaveForm {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "WaveForm(<function>)")
    }
}

impl WaveForm {
    pub fn new<F>(function: F) -> WaveForm
    where
        F: Fn(f32) -> f32 + 'static + Send + Clone,
    {
        WaveForm {
            inner: Box::new(function),
        }
    }

    pub fn run(&self, phase: f32) -> f32 {
        self.inner.run(phase)
    }
}

trait RunAndClone {
    fn run(&self, phase: f32) -> f32;

    fn my_clone(&self) -> Box<RunAndClone + Send>;
}

impl Clone for Box<RunAndClone + Send> {
    fn clone(&self) -> Box<RunAndClone + Send> {
        self.my_clone()
    }
}

impl<F> RunAndClone for F
where
    F: Fn(f32) -> f32 + 'static + Send + Clone,
{
    fn run(&self, phase: f32) -> f32 {
        self(phase)
    }

    fn my_clone(&self) -> Box<RunAndClone + Send> {
        Box::new((*self).clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn allows_to_implement_rect_waves() {
        let wave_form = WaveForm::new(|phase: f32| if phase < PI { -1.0 } else { 1.0 });
        assert_eq!(wave_form.run(0.0), -1.0);
    }

    #[test]
    fn allows_to_use_closures() {
        let foo = 42.0;
        let wave_form = WaveForm::new(move |phase: f32| phase + foo);
        assert_eq!(wave_form.run(0.0), 42.0);
    }

    #[test]
    fn implements_debug() {
        let wave_form = WaveForm::new(move |phase: f32| phase);
        assert_eq!(format!("{:?}", wave_form), "WaveForm(<function>)");
    }
}

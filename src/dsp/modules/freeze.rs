use num_complex::Complex;
use crate::params::{FxChannelTarget, StereoLink};
use super::{ModuleContext, ModuleType, SpectralModule};

pub struct FreezeModule;
impl FreezeModule { pub fn new() -> Self { Self } }
impl SpectralModule for FreezeModule {
    fn reset(&mut self, _: f32, _: usize) {}
    fn process(
        &mut self, _: usize, _: StereoLink, _: FxChannelTarget,
        _: &mut [Complex<f32>], _: Option<&[f32]>, _: &[&[f32]],
        suppression_out: &mut [f32], _: &ModuleContext,
    ) { suppression_out.fill(0.0); }
    fn module_type(&self) -> ModuleType { ModuleType::Freeze }
    fn num_curves(&self) -> usize { 4 }
}

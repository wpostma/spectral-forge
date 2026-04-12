use nih_plug::prelude::*;
use std::sync::Arc;

mod dsp;

#[derive(Params)]
struct SpectralForgeParams {}

pub struct SpectralForge;

impl Default for SpectralForge {
    fn default() -> Self { Self }
}

impl Plugin for SpectralForge {
    const NAME: &'static str = "Spectral Forge";
    const VENDOR: &'static str = "Kim";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        Arc::new(SpectralForgeParams {})
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }
}

impl ClapPlugin for SpectralForge {
    const CLAP_ID: &'static str = "com.spectral-forge.spectral-forge";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Spectral compressor");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
    ];
}

nih_export_clap!(SpectralForge);

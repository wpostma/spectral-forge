use num_complex::Complex;

/// Per-bin parameter values, physical units, pre-computed by pipeline.
pub struct BinParams<'a> {
    pub threshold_db: &'a [f32],  // dBFS per bin, e.g. -20.0
    pub ratio:        &'a [f32],  // ratio per bin, e.g. 4.0 = 4:1
    pub attack_ms:    &'a [f32],  // ms per bin, freq-scaled by pipeline
    pub release_ms:   &'a [f32],  // ms per bin, freq-scaled by pipeline
    pub knee_db:      &'a [f32],  // soft knee width in dB per bin
    pub makeup_db:    &'a [f32],  // makeup gain dB per bin
    pub mix:          &'a [f32],  // dry/wet per bin [0.0, 1.0]
}

pub trait SpectralEngine: Send {
    /// Called at initialize() and on sample rate / FFT size change.
    /// Pre-allocate all heap state here — never in process_bins().
    fn reset(&mut self, sample_rate: f32, fft_size: usize);

    /// Called once per STFT hop on the audio thread.
    /// Must not allocate, lock, or perform I/O.
    /// Write |gain_reduction_db| per bin into suppression_out for GUI stalactites.
    fn process_bins(
        &mut self,
        bins: &mut [Complex<f32>],
        sidechain: Option<&[f32]>,     // pre-smoothed sidechain magnitude, or None
        params: &BinParams,
        sample_rate: f32,
        suppression_out: &mut [f32],
    );

    /// Tail after silence. Override for engines with extended tails (e.g. Freeze).
    fn tail_length(&self, fft_size: usize) -> u32 {
        fft_size as u32
    }

    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineSelection {
    SpectralCompressor,
}

pub fn create_engine(sel: EngineSelection) -> Box<dyn SpectralEngine> {
    match sel {
        EngineSelection::SpectralCompressor => {
            Box::new(spectral_compressor::SpectralCompressorEngine::new())
        }
    }
}

pub mod spectral_compressor;

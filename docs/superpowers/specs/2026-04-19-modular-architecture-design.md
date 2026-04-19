# Modular Processing Architecture — Design Spec

**Date:** 2026-04-19  
**Status:** Approved — ready for implementation planning

## Goals

Replace the current fixed-stage DSP pipeline (single Dynamics slot + hardcoded EffectMode switching) with a fully modular architecture where:

- Every processing type lives in its own self-contained file implementing a common trait
- Up to 8 user-assignable slots + 1 fixed Master slot, routed via a 9×9 matrix
- Each slot has independent per-bin curve parameters
- Adding a new module type in future = one new file + one match arm in `create_module()`; nothing else changes
- 4 stereo CLAP sidechain inputs; each slot can be assigned to any of them
- The plugin UI adapts dynamically to whatever module is selected for editing

---

## 1. SpectralModule Trait

**File:** `src/dsp/modules/mod.rs`

This is the single interface all modules implement. Nothing outside a module file needs to know the module's internals.

```rust
pub trait SpectralModule: Send {
    /// Process one STFT hop in-place.
    ///
    /// - `channel`: 0 = left/mid, 1 = right/side
    /// - `stereo_link`: plugin-wide stereo mode
    /// - `target`: which channels this slot processes (All / Mid / Side)
    /// - `bins`: complex FFT frame, modified in-place
    /// - `sidechain`: smoothed magnitude envelope from the assigned aux input,
    ///   or None if no sidechain is connected
    /// - `curves`: `[0..num_curves()]` slices, each `ctx.num_bins` long,
    ///   linear gain multipliers from the per-slot curve nodes
    /// - `suppression_out`: filled with ≥0 dB values for the display;
    ///   must be fully written (not left uninitialised)
    /// - `ctx`: read-only shared context (sample rate, fft size, global scalars)
    fn process(
        &mut self,
        channel: usize,
        stereo_link: StereoLink,
        target: FxChannelTarget,
        bins: &mut [Complex<f32>],
        sidechain: Option<&[f32]>,
        curves: &[&[f32]],
        suppression_out: &mut [f32],
        ctx: &ModuleContext,
    );

    fn reset(&mut self, sample_rate: f32, fft_size: usize);

    /// Extra output tail beyond one FFT window. Default: 0.
    fn tail_length(&self) -> u32 { 0 }

    fn module_type(&self) -> ModuleType;

    /// How many of the 7 curve slots this module uses (0..=7).
    fn num_curves(&self) -> usize;
}
```

### ModuleContext

Passed read-only to every `process()` call. Contains the global scalar params that modules share.

```rust
pub struct ModuleContext<'a> {
    pub sample_rate: f32,
    pub fft_size:    usize,
    pub num_bins:    usize,
    pub attack_ms:          f32,
    pub release_ms:         f32,
    pub sensitivity:        f32,
    pub suppression_width:  f32,
    pub auto_makeup:        bool,
    pub delta_monitor:      bool,
    // extend as needed; modules access only what they use
    pub _params: std::marker::PhantomData<&'a ()>,
}
```

### ModuleType

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default,
         serde::Serialize, serde::Deserialize)]
pub enum ModuleType {
    #[default]
    Empty,
    Dynamics,
    Freeze,
    PhaseSmear,
    Gain,
    MidSide,
    Harmonic,
    Master,   // only valid at slot 8; never stored in user-assignable slots
}
```

### ModuleSpec

Statically known metadata per type. Used by the GUI (colors, button labels) and by `create_module()`.

```rust
pub struct ModuleSpec {
    pub display_name:  &'static str,
    pub color_lit:     Color32,   // active/selected
    pub color_dim:     Color32,   // present but not selected
    pub num_curves:    usize,
    pub curve_labels:  &'static [&'static str],
}

pub fn module_spec(ty: ModuleType) -> &'static ModuleSpec { /* match */ }
```

**Fixed colours and curve counts:**

| Type       | Color lit   | Curves | Labels |
|------------|-------------|--------|--------|
| Dynamics   | `#50c0c4`   | 6      | THRESHOLD, RATIO, ATTACK, RELEASE, KNEE, MIX |
| Freeze     | `#5080c8`   | 4      | LENGTH, THRESHOLD, PORTAMENTO, RESISTANCE |
| PhaseSmear | `#9060c8`   | 1      | AMOUNT |
| Gain       | `#c8a050`   | 1      | GAIN |
| MidSide    | `#c050a0`   | 5      | BALANCE, EXPANSION, DECORREL, TRANSIENT, PAN |
| Harmonic   | `#50c880`   | 0      | — |
| Master     | `#cccccc`   | 0      | — |

### create_module

```rust
pub fn create_module(
    ty: ModuleType,
    sample_rate: f32,
    fft_size: usize,
) -> Box<dyn SpectralModule>
```

The only place the concrete type names appear outside their own files. Every new module adds one arm here.

---

## 2. Module Files

**Directory:** `src/dsp/modules/`

Each file is fully self-contained. No module file may import from another module file.

### `dynamics.rs`
- Wraps the existing `SpectralEngine` (compressor) and contrast engine (spectral contrast mode)
- 6 curves: threshold, ratio, attack, release, knee, mix
- Channel gating logic moves here from `fx_matrix.rs`
- `auto_makeup` read from `ctx`
- The old `EffectMode::SpectralContrast` becomes an internal dynamics sub-mode, not a plugin-level param

### `freeze.rs`
- Owns all freeze state: `frozen_bins`, `freeze_target`, `freeze_port_t`, `freeze_hold_hops`, `freeze_accum`, `freeze_captured` (all `Vec<_>` pre-allocated at `MAX_NUM_BINS`)
- 4 curves: length, threshold, portamento, resistance
- DSP moved verbatim from `pipeline.rs` STFT closure; only the curve reads change
- `tail_length()` returns `FFT_SIZE as u32`

### `phase_smear.rs`
- Owns `rng_state: u64` (xorshift64, never zero)
- 1 curve: amount
- DSP moved from `pipeline.rs`

### `gain.rs`
- Stateless; no owned DSP state
- 1 curve: GAIN — maps linear gain multiplier to dB range **−24 to +18 dB** per bin
- `suppression_out` is zeroed (gain is display-only in the spectrum, not shown as suppression)
- Future: sidechain-aware auto-gain matching; `auto_makeup` eventually migrates here

### `mid_side.rs`
- Ported from `spectral2/src/dsp/pipeline.rs` lines 446–501
- 5 curves: balance, expansion, decorrelation, transient, pan
- Only active when `stereo_link == StereoLink::MidSide`; passes through otherwise
- Requires M/S encode/decode to be active at the pipeline level (same requirement as today)

### `harmonic.rs`
- Pass-through; 0 curves
- Reserved for future harmonic processing

### `master.rs`
- Transparent pass-through; 0 curves
- Slot 8 only; cannot be assigned or removed by the user
- `suppression_out` zeroed

---

## 3. Data Model

### 3a. Params (`src/params.rs`)

**Remove:**
- `curve_nodes`, `active_curve`
- `phase_curve_nodes`, `freeze_curve_nodes`, `freeze_active_curve`
- `fx_module_types`, `fx_module_names`, `fx_module_targets`
- `effect_mode: EnumParam<EffectMode>`
- `phase_rand_amount: FloatParam`
- `spectral_contrast_db: FloatParam`

**Add:**
```rust
/// Per-slot curve nodes. [slot 0..=8][curve 0..6][node 0..5].
/// Slot 8 = Master; its curves are unused but stored for uniform access.
#[persist = "slot_curve_nodes"]
pub slot_curve_nodes: Arc<Mutex<[[[CurveNode; NUM_NODES]; 7]; 9]>>,

/// Module type assigned to each slot.
/// Slot 8 is always ModuleType::Master; the value stored here is ignored.
#[persist = "slot_module_types"]
pub slot_module_types: Arc<Mutex<[ModuleType; 9]>>,

/// User-editable name for each slot. UTF-8, zero-padded to 32 bytes.
/// Display truncates to ~10 visible characters; full name shown on hover.
#[persist = "slot_names"]
pub slot_names: Arc<Mutex<[[u8; 32]; 9]>>,

/// Channel routing target per slot (All / Mid / Side).
#[persist = "slot_targets"]
pub slot_targets: Arc<Mutex<[FxChannelTarget; 9]>>,

/// Sidechain input assignment per slot. 0..=3 = aux input index.
/// 255 = self-detect (use main input when no sidechain is connected).
#[persist = "slot_sidechain"]
pub slot_sidechain: Arc<Mutex<[u8; 9]>>,

/// Which curve within the editing slot is selected (0..num_curves).
#[persist = "editing_curve"]
pub editing_curve: Arc<Mutex<u8>>,
```

**Rename/extend:**
- `editing_slot` stays (now 0..=8)
- `fx_route_matrix` extended: `Arc<Mutex<[[f32; 9]; 9]>>`

**Keep (global scalars, unaffected):**
- `input_gain`, `output_gain`, `mix`
- `attack_ms`, `release_ms`, `sc_gain`, `sc_attack_ms`, `sc_release_ms`
- `lookahead_ms`, `stereo_link`, `threshold_mode`, `sensitivity`
- `suppression_width`, `auto_makeup`, `delta_monitor`
- `graph_db_min`, `graph_db_max`, `peak_falloff_ms`
- All tilt/offset params (now apply to slot 0 Dynamics only; per-slot tilt is a future addition)

**Default state:**
- Slot 0: `ModuleType::Dynamics`, name `"Dynamics"`, target `All`, sidechain `255`
- Slots 1–7: `ModuleType::Empty`, names `"Slot 1"` … `"Slot 7"`
- Slot 8: `ModuleType::Master`, name `"Master"`, immutable
- All `slot_curve_nodes`: default neutral nodes per curve type

**Remove enums:**
- `EffectMode` — deleted
- `FxModuleType` — replaced by `ModuleType` in `modules/mod.rs`

### 3b. Bridge (`src/bridge.rs`)

Replace 7 flat curve channels with 9×7 = 63:

```rust
pub curve_tx: Vec<Vec<Arc<Mutex<TbInput<Vec<f32>>>>>>,  // [slot][curve]
pub curve_rx: Vec<Vec<TbOutput<Vec<f32>>>>,              // [slot][curve]
```

`SharedState::new(num_bins, sample_rate, fft_size)` — add `fft_size` parameter (Plan B already planned this).

Add per-aux sidechain activity:
```rust
pub sidechain_active: [Arc<AtomicBool>; 4],
```

Remove: `pending_engine: Arc<AtomicU8>` (dead code since Plan C).

### 3c. Pipeline (`src/dsp/pipeline.rs`)

**Slot curve cache:**

Replace `curve_cache: [Vec<f32>; 7]` with:
```rust
slot_curve_cache: Vec<Vec<Vec<f32>>>,  // [9][7][MAX_NUM_BINS], pre-allocated
```

Read loop (no allocation):
```rust
for s in 0..9 {
    for c in 0..7 {
        let src = shared.curve_rx[s][c].read();
        self.slot_curve_cache[s][c][..num_bins].copy_from_slice(src);
    }
}
```

**Sidechain:**

Process up to 4 aux inputs independently:
```rust
let mut sc_envelopes: [Option<Vec<f32>>; 4] = [None, None, None, None];
for i in 0..aux.inputs.len().min(4) {
    // run StftHelper + envelope follower for aux.inputs[i]
    // store smoothed magnitude envelope in sc_envelopes[i]
}
```

Per-slot sidechain selection:
```rust
let sc = match slot_sidechain[i] {
    255 => if sc_envelopes[0].as_ref().map(|e| e.iter().any(|&v| v > 1e-9)).unwrap_or(false) {
        sc_envelopes[0].as_deref()
    } else { None },
    idx => sc_envelopes[idx as usize].as_deref(),
};
```

**STFT closure** shrinks: no more `match effect_mode`. Just calls `fx_matrix.process_hop(...)` for each channel.

### 3d. CLAP layouts (`src/lib.rs`)

```rust
const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
    AudioIOLayout {
        main_input_channels:  NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[
            new_nonzero_u32(2),  // sidechain 1
            new_nonzero_u32(2),  // sidechain 2
            new_nonzero_u32(2),  // sidechain 3
            new_nonzero_u32(2),  // sidechain 4
        ],
        ..AudioIOLayout::const_default()
    },
    AudioIOLayout {
        main_input_channels:  NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    },
];
```

---

## 4. FxMatrix Changes (`src/dsp/fx_matrix.rs`)

```rust
pub const MAX_SLOTS: usize = 9;  // was 8

pub struct FxMatrix {
    pub slots: [Option<Box<dyn SpectralModule>>; MAX_SLOTS],
    pub send:  [[f32; MAX_SLOTS]; MAX_SLOTS],
    slot_out_cur:  Vec<Vec<Complex<f32>>>,  // [MAX_SLOTS][MAX_NUM_BINS]
    slot_out_prev: Vec<Vec<Complex<f32>>>,
    slot_supp:     Vec<Vec<f32>>,
}
```

`FxSlotKind` enum is deleted entirely; replaced by `Box<dyn SpectralModule>`.

**Slot 8 (Master):**
- Initialised in `FxMatrix::new()` as `Some(Box::new(MasterModule::new(...)))`
- `process_hop` treats slot 8 as the designated output: after processing, writes slot 8's output buffer back to `complex_buf`
- Cannot be assigned or removed via any public API

**process_hop signature** gains `slot_curves`:
```rust
pub fn process_hop(
    &mut self,
    channel: usize,
    stereo_link: StereoLink,
    complex_buf: &mut [Complex<f32>],
    sc_envelopes: &[Option<&[f32]>; 4],  // pre-computed, indexed by aux input
    slot_sidechain: &[u8; 9],
    slot_targets: &[FxChannelTarget; 9],
    slot_curves: &[&[&[f32]; 7]; 9],     // [slot][curve][bin]
    ctx: &ModuleContext,
    suppression_out: &mut [f32],
    num_bins: usize,
)
```

Dispatches to `slot.process(channel, stereo_link, target, bins, sc, curves, supp, ctx)`.

---

## 5. UI Changes

### Removed
- `EffectMode` button strip (Bypass / Freeze / PhaseRand / SpectralContrast) — gone entirely
- Fixed tabs DYNAMICS / EFFECTS / HARMONIC — gone; replaced by matrix selection
- Freeze-specific curve buttons in top bar — replaced by adaptive buttons
- Phase-specific controls — absorbed into PhaseSmear module slot editing

### Module assignment popup
- **Empty cell (diagonal):** left-click opens popup
- **Populated cell (diagonal):** left-click selects for editing; right-click opens popup with "Remove module" option
- Popup is an egui `Area` anchored near the clicked cell
- Lists all assignable types with colour swatch; "Remove module" at bottom (absent on Master)
- On assignment: `slot_module_types[i]` updated, `FxMatrix` slot replaced with `create_module(ty, ...)`, curve nodes reset to type defaults

### Adaptive curve selector buttons (top bar)
- Buttons derive from `module_spec(slot_module_types[editing_slot]).curve_labels`
- Active button uses `color_lit` for the editing slot's type; inactive buttons use `color_dim`
- If `num_curves == 0` (Master, Harmonic): no curve buttons shown; top bar shows only range controls

### Matrix cell display
- Name truncated to fit cell width (~10 characters); full name shown as egui hover tooltip
- Cell background: `color_lit` when selected, `color_dim` when populated-but-not-selected
- Master cell (row/col 8): always white/bright (`#cccccc`), label "OUT", no popup

### Graph header
- Format: `"Editing: {name} — {target}"` where name is the slot's user name
- Name is **inline-editable**: clicking the name activates a `TextEdit` widget, saved on Enter or focus loss, limited to 32 bytes UTF-8
- Disambiguation number appended automatically in the display name when multiple slots share a type ("Dynamics", "Dynamics 2", "Dynamics 3") — the stored name is separate and user-controlled

### Sidechain assignment
- Per-slot sidechain selector shown in the slot editing strip (below the curve area): small buttons "SC1" "SC2" "SC3" "SC4" "Self", selecting which aux input this slot uses
- "Self" = 255 = fall back to main input when aux is disconnected (current behaviour)

---

## 6. Implementation Plans

### Plan D1 — Foundation (target: same behaviour, new architecture)

| # | Task |
|---|------|
| 1 | `src/dsp/modules/mod.rs`: SpectralModule trait, ModuleType, ModuleSpec, ModuleContext, create_module stub |
| 2 | Module files: dynamics.rs (wraps existing engines), freeze.rs, phase_smear.rs, gain.rs, harmonic.rs, master.rs — DSP migrated from pipeline.rs |
| 3 | `src/params.rs`: add all per-slot fields; remove old fields; extend route matrix to 9×9 |
| 4 | `src/bridge.rs`: 9×7 curve channels; 4 sidechain_active atomics; remove pending_engine |
| 5 | `src/lib.rs`: 4 aux sidechain inputs in AUDIO_IO_LAYOUTS; wire fft_size through initialize |
| 6 | `src/dsp/pipeline.rs`: slot_curve_cache; 4 sidechain processing paths; shrink STFT closure |
| 7 | `src/dsp/fx_matrix.rs`: Box<dyn SpectralModule>, 9 slots, Master at slot 8, updated process_hop |

End state: plugin behaviour identical to today; Dynamics/Freeze/PhaseSmear all work; all tests pass.

### Plan D2 — UX + new modules

| # | Task |
|---|------|
| 1 | Module assignment popup (egui Area); right-click on populated cell |
| 2 | Adaptive curve selector buttons in top bar; remove fixed tabs and EffectMode strip |
| 3 | Matrix cell truncation + hover tooltip; inline name edit in graph header |
| 4 | Sidechain assignment strip (SC1–SC4/Self buttons per slot) |
| 5 | Gain module DSP + curve editor wiring |
| 6 | M/S module DSP (ported from spectral2) + curve editor wiring |

---

## 7. What Is Not In Scope

- Per-slot tilt/offset params (currently global; stays global until a specific need arises)
- Variable FFT per slot (Plan B covers plugin-wide variable FFT)
- Harmonic module DSP (placeholder only in D1/D2)
- More than 4 sidechain inputs
- Reordering slots (matrix position is fixed; user labels slots by name)

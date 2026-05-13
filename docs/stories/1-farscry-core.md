Story 1 - farscry-core

**Status:** Ready - @po fixes applied 2026-05-09
**Blocked by:** nothing
**Estimated hours:** 12h
**@architect sign-off:** required before @dev starts (architecture doc must be formally closed)

---

What to build

The foundational crate. Zero business logic. Only types, traits, and the pipeline orchestrator.

Crate: `crates/farscry-core/`

---

`src/types.rs` - complete type definitions

StateId - FIX 1 applied

```rust


pub struct StateId([u8; 8]);

impl StateId {
    pub fn from_bits(bits: u64) -> Self {
        Self(bits.to_be_bytes())
    }
    pub fn to_bits(&self) -> u64 {
        u64::from_be_bytes(self.0)
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "phash:{}", hex::encode(self.0))
    }
}
```

VaspOutput - FIX 2 applied

```rust


pub struct VaspOutput {
    pub vasp_version:       &'static str,       // always "1.0"
    pub schema_version:     u32,                // always 1
    pub state_id:           StateId,
    pub screen_type:        ScreenType,
    pub confidence:         Confidence,
    pub lang:               String,             // e.g. "eng", "por"
    pub delta_from:         Option<StateId>,    // null on first observation
    pub context_similarity: Option<f32>,        // null on single extract (not diff)
    pub context_changed:    Option<bool>,       // null on single extract (not diff)
    pub agent_context:      String,             // one-line summary
    pub ui_tree:            Vec<UiElement>,
    pub affordances:        Vec<Affordance>,
}

impl VaspOutput {

    pub fn new(
        state_id: StateId,
        screen_type: ScreenType,
        confidence: Confidence,
        lang: impl Into<String>,
        agent_context: impl Into<String>,
        ui_tree: Vec<UiElement>,
        affordances: Vec<Affordance>,
    ) -> Self {
        Self {
            vasp_version: "1.0",
            schema_version: 1,
            state_id,
            screen_type,
            confidence,
            lang: lang.into(),
            delta_from: None,
            context_similarity: None,
            context_changed: None,
            agent_context: agent_context.into(),
            ui_tree,
            affordances,
        }
    }
}
```

VaspDelta - FIX 3 applied

```rust


pub struct VaspDelta {
    pub vasp_version:       &'static str,   // always "1.0"
    pub diff_from:          StateId,
    pub diff_to:            StateId,
    pub context_similarity: f32,            // 0.0-1.0 token overlap
    pub context_changed:    bool,           // true when similarity < 0.20
    pub agent_context:      String,
    pub entries:            Vec<DeltaEntry>,
    pub tokens_saved:       Option<usize>,  // estimated tokens vs re-sending
}


pub enum DeltaEntry {
    Appeared(UiElement),
    Removed(UiElement),
    Changed { before: UiElement, after: UiElement },
    Unchanged(UiElement),
}
```

BatchResult - FIX 4 applied

```rust


pub struct BatchResult {
    pub path:   std::path::PathBuf,
    pub output: Result<VaspOutput, FarscryError>,
}
```

ScreenType

```rust

pub enum ScreenType {
    Error,
    Config,
    Terminal,
    Conversation,
    Ui,
    Unknown,
}
```

Confidence - PartialOrd/Ord required for threshold comparisons

```rust

pub enum Confidence {
    None   = 0,
    Low    = 1,
    Medium = 2,
    High   = 3,
}
```

UiElement (replaces TypedElement - cleaner name for the VASP output layer)

```rust

pub struct UiElement {
    pub text:         String,
    pub element_type: ElementType,
    pub cx:           f32,          // centroid x (pixels from left)
    pub cy:           f32,          // centroid y (pixels from top)
    pub w:            f32,
    pub h:            f32,
    pub enabled:      Option<bool>,
    pub value:        Option<String>,
}
```

ElementType

```rust

pub enum ElementType {
    Button,
    Input,
    Select,     // dropdown - required by Affordance::Select
    Label,
    Heading,
    Error,
    Badge,
    Unknown,
}
```

> **Note:** `Select` added to resolve @po issue [5-D] - `AffordanceAction::Select` requires a corresponding element type.

Affordance

```rust

pub struct Affordance {
    pub action:        AffordanceAction,
    pub label:         String,
    pub cx:            f32,
    pub cy:            f32,
    pub enabled:       bool,
    pub current_value: Option<String>,
}


pub enum AffordanceAction {
    Click,
    Type,
    Select,
}
```

OcrOutput (NOT HocrOutput)

```rust

pub struct OcrOutput {
    pub regions: Vec<TextRegion>,
    pub width:   u32,
    pub height:  u32,
}


pub struct TextRegion {
    pub text: String,
    pub cx:   f32,   // centroid x
    pub cy:   f32,   // centroid y
    pub w:    f32,
    pub h:    f32,
}
```

ClassifiedScreen (internal pipeline type)

```rust

pub struct ClassifiedScreen {
    pub ui_tree:     Vec<UiElement>,
    pub screen_type: ScreenType,
    pub state_id:    StateId,
    pub lang:        String,
    pub confidence:  Confidence,
}
```

---

`src/traits.rs` - pipeline traits (all Arc-compatible)

```rust
pub trait Preprocessor: Send + Sync + 'static {
    fn process(&self, image: image::DynamicImage) -> image::DynamicImage;
}

pub trait OcrEngine: Send + Sync + 'static {
    fn extract(&self, image: &image::DynamicImage) -> Result<OcrOutput, FarscryError>;
}

pub trait ElementClassifier: Send + Sync + 'static {
    fn classify(&self, ocr: &OcrOutput) -> Vec<UiElement>;
}

pub trait ScreenClassifier: Send + Sync + 'static {
    fn classify(&self, elements: &[UiElement]) -> ScreenType;
}

pub trait StateHasher: Send + Sync + 'static {
    fn hash(&self, image: &image::DynamicImage) -> StateId;
}

pub trait VaspFormatter: Send + Sync + 'static {
    fn format(&self, screen: &ClassifiedScreen) -> VaspOutput;
}

pub trait DiffEngine: Send + Sync + 'static {
    fn diff(&self, before: &VaspOutput, after: &VaspOutput) -> VaspDelta;
}
```

All trait objects used as `Arc<dyn Trait>` in Pipeline - **NOT Box**. Arc allows cheap clone for Rayon batch workers.

---

`src/pipeline.rs`

```rust
pub struct Pipeline {
    preprocessor:       std::sync::Arc<dyn Preprocessor>,
    ocr:                std::sync::Arc<dyn OcrEngine>,
    element_classifier: std::sync::Arc<dyn ElementClassifier>,
    screen_classifier:  std::sync::Arc<dyn ScreenClassifier>,
    state_hasher:       std::sync::Arc<dyn StateHasher>,
    formatter:          std::sync::Arc<dyn VaspFormatter>,
}

impl Pipeline {
    pub fn process(&self, image: image::DynamicImage) -> Result<VaspOutput, FarscryError> {
        let preprocessed  = self.preprocessor.process(image);
        let state_id      = self.state_hasher.hash(&preprocessed);
        let ocr           = self.ocr.extract(&preprocessed)?;
        let elements      = self.element_classifier.classify(&ocr);
        let screen_type   = self.screen_classifier.classify(&elements);
        let screen        = ClassifiedScreen {
            ui_tree: elements,
            screen_type,
            state_id,
            lang: "eng".into(),
            confidence: Confidence::High,
        };
        Ok(self.formatter.format(&screen))
    }


    pub fn process_batch(&self, paths: Vec<std::path::PathBuf>) -> Vec<BatchResult> {
        use rayon::prelude::*;
        paths.par_iter().map(|path| {
            let result = image::open(path)
                .map_err(|e| FarscryError::ImageLoad { path: path.clone(), source: e })
                .and_then(|img| self.process(img));
            BatchResult { path: path.clone(), output: result }
        }).collect()
    }
}
```

---

`src/hash.rs` - pHash implementation - FIX 6 applied

**Standard perceptual hash algorithm (unambiguous steps):**

```
1. Resize to 32x32 pixels using nearest-neighbor interpolation ONLY
   (nearest-neighbor: integer arithmetic, deterministic across all CPU architectures)

2. Convert to grayscale using ITU-R BT.601 luma:
   gray[x][y] = 0.299 x R + 0.587 x G + 0.114 x B
   Result: 32x32 = 1024 f32 values in range [0.0, 255.0]

3. Compute the 32x32 2D DCT-II (full image, NOT 8x8 block DCT):
   DCT[u][v] = (2/N) x Σ_x Σ_y gray[x][y]
                         x cos(π(2x+1)u / 64)
                         x cos(π(2y+1)v / 64)
   where N = 32
   (This is standard pHash, NOT JPEG block-DCT)

4. Extract the top-left 8x8 DCT coefficients: DCT[0..8][0..8]
   These are the 64 lowest-frequency components.

5. Exclude DCT[0][0] (the DC component - encodes mean luminance, too dominant).
   Working set: 63 values from DCT[0..8][0..8] excluding [0][0].

6. Compute mean of the 63 values:
   mean = sum(working_set) / 63.0

7. For each of 63 values: bit = 1 if value > mean, else bit = 0
   Pack as 63 bits, pad to 64 bits (1 padding zero at MSB or LSB - document choice).

8. Store as StateId([u8; 8]) - big-endian byte order.

9. Display: "phash:" + 16-char lowercase hex of the 8 bytes.

Why this is cross-platform deterministic:
  - Steps 1-2: integer/fixed-point arithmetic -> identical on all CPUs
  - Step 3: f32 DCT has sub-LSB FP variance between AVX2/AVX-512/NEON
  - Steps 4-7: bit = (value > mean) - a COMPARISON, not the floating point value itself
    -> Even if DCT values differ by 1e-7 between CPU architectures,
      the comparison result (above/below mean) is stable unless a value
      is exactly at the mean boundary (astronomically rare)
  - Step 8-9: binary -> deterministic
```

**Implementation note:** Use the `rustdct` crate (pure Rust, MIT, no SIMD dependencies that affect correctness) for the 2D DCT. Do not use FFTW bindings (C++ dependency) or ndarray-based DCT (potential BLAS variance).

```toml
Add to farscry-core Cargo.toml:
rustdct = "0.7"
hex = "0.4"
```

```rust
pub struct PHasher;

impl StateHasher for PHasher {
    fn hash(&self, image: &image::DynamicImage) -> StateId {
        phash_image(image)
    }
}

pub fn phash_image(image: &image::DynamicImage) -> StateId {

    let small = image.resize_exact(32, 32, image::imageops::FilterType::Nearest);

    let gray = small.to_luma8();

    let pixels: Vec<f32> = gray.pixels()
        .map(|p| p[0] as f32)
        .collect();
    let dct = compute_2d_dct(&pixels, 32);


    pack_phash_bits(&dct)
}
```

---

`src/error.rs` - FIX 5 applied

```rust
use std::path::PathBuf;


pub enum FarscryError {

    ImageLoad {
        path: PathBuf,

        source: image::ImageError,
    },


    ModelIntegrity {
        model: String,
        expected: String,
        actual: String,
    },


    ModelNotFound { path: PathBuf },


    OcrFailed(String),


    InvalidInput { message: String },


    LanguageNotInstalled(String),
}
```

---

`Cargo.toml` dependencies

```toml
[package]
name = "farscry-core"
version = "0.1.0"
edition = "2021"
publish = true

[dependencies]
image       = "0.25"
serde       = { version = "1", features = ["derive"] }
thiserror   = "1"
rayon       = "1"
rustdct     = "0.7"
hex         = "0.4"
```

---

Acceptance criteria

- [ ] `cargo test -p farscry-core` passes, zero warnings
- [ ] `cargo clippy -p farscry-core -- -D warnings` passes
- [ ] `StateId` is `[u8; 8]` - confirm via `assert_eq!(std::mem::size_of::<StateId>(), 8)`
- [ ] `StateId` Display outputs `phash:` prefix + 16-char lowercase hex
- [ ] pHash determinism: same image -> same `StateId` on 100 consecutive calls
- [ ] pHash perceptual stability: image shifted 1px -> same `StateId` (tolerance test)
- [ ] pHash sensitivity: image with new error banner -> different `StateId`
- [ ] pHash cross-platform: pre-computed fixture hash matches on x86_64 and aarch64 (checked via a golden-file test)
- [ ] `VaspOutput` has all 12 fields: vasp_version, schema_version, state_id, screen_type, confidence, lang, delta_from, context_similarity, context_changed, agent_context, ui_tree, affordances
- [ ] `VaspDelta` has all fields: vasp_version, diff_from, diff_to, context_similarity, context_changed, agent_context, entries, tokens_saved
- [ ] `DeltaEntry` enum: Appeared | Removed | Changed { before, after } | Unchanged
- [ ] `BatchResult` struct with `path: PathBuf` and `output: Result<VaspOutput, FarscryError>`
- [ ] `FarscryError::LanguageNotInstalled(String)` variant exists
- [ ] `Confidence` satisfies `High > Medium > Low > None` via `PartialOrd`
- [ ] `ElementType::Select` variant exists (required by `AffordanceAction::Select`)
- [ ] `Pipeline::process_batch` takes `Vec<PathBuf>`, NOT `Vec<DynamicImage>`
- [ ] `cargo tree -p farscry-core` shows no dependencies beyond: image, serde, thiserror, rayon, rustdct, hex
- [ ] `publish = true` in Cargo.toml

Dependencies

None.

---

@architect notes - open before @dev starts

1. **Architecture document:** `docs/architecture/farscry-ocr-backend-architecture.md` must be formally superseded by PRD v3 + this story. Add a sign-off record at the top of the architecture doc: `"Superseded by farscry-v0.1.0-prd.md - @architect, @rust-engineer, 2026-05-09"`.
2. **`farscry-preprocessor/` crate:** Appears in the architecture doc but NOT in PRD §7 (6-crate workspace). Decision: preprocessing logic lives in the `farscry-ocr-ort` and `farscry-ocr-coreml` crates respectively (dark mode + resize are OCR-engine-adjacent). No separate `farscry-preprocessor` crate in v0.1.0.
3. **`rustdct` crate audit:** `@security-engineer` must verify license (MIT) and supply chain before Story 1 merges.

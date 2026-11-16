//! LumaRecode shared core.
//!
//! Layering, from the outside in:
//!
//! - `domain` — the vocabulary (purpose, codec, sampling, precision, range).
//! - `source` — a normalized description of one input file.
//! - `policy` — source-aware output rules (what may be produced).
//! - `timeline` — frame-accurate selection in stream ticks.
//! - `presets` — the shipped measured preset tables.
//! - `capability` — what this machine can actually run.
//! - `plan` — the ordered, backend-neutral work to perform.
//!
//! Everything here is pure: no filesystem, no subprocess, no window. That is
//! what lets the Windows host and the macOS host share one rule set, and what
//! lets the whole rule surface be tested without a GPU.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod cache;
pub mod capability;
pub mod domain;
pub mod error;
pub mod execution;
pub mod metadata;
pub mod plan;
pub mod policy;
pub mod presets;
pub mod probe;
pub mod process;
pub mod providers;
pub mod render;
pub mod safety;
pub mod source;
pub mod timeline;

/// Version of this crate, independent of the ABI version.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version of the C ABI exported by `luma-ffi`.
pub const ABI_VERSION: u32 = 1;

pub use capability::{
    Backend, CapabilityStatus, Device, EncoderCapability, Requirement, Selection,
};
pub use domain::{
    AudioChoice, Codec, ColorRange, ContainerChoice, DepthChoice, DynamicHdrMode, EncodeOptions,
    LogProfile, OptionsDto, Output, Purpose, RangeChoice, ResolvedEncode, ResolvedFormat, Sampling,
    SamplingChoice, SourceKind, Transform, Variant,
};
pub use error::{CoreError, CoreResult};
pub use plan::{DecodePath, FrameTransport, Plan, PlanRequest, PlanStep, StepKind};
pub use policy::{FlacTrack, flac_tracks};
pub use presets::{AudioDefault, AudioPreset, PresetRegistry, VideoPreset};
pub use source::{AudioStream, DynamicHdrInfo, PictureFormat, SourceFormat};
pub use timeline::{FramePoint, FrameRange, FrameTimeline, FrameWindow};

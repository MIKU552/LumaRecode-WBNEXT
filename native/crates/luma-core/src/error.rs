//! Stable error identities shared by the native core and every host.
//!
//! Messages are never produced here: the core reports a stable code plus the
//! values that failed, and each front end renders localized text. This keeps
//! the CLI's machine-readable output and the GUI's localized strings in sync.

use std::error::Error;
use std::fmt;

/// A rule violation identified by code, optionally carrying the offending value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    code: &'static str,
    detail: Option<String>,
}

impl CoreError {
    /// Creates an error that only carries a code.
    pub const fn new(code: &'static str) -> Self {
        Self { code, detail: None }
    }

    /// Creates an error that also records the rejected value or field name.
    pub fn with_detail(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: Some(detail.into()),
        }
    }

    /// Stable identifier; hosts map this to localized text.
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// The rejected value, if the rule can name one.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Adds a detail value without changing the code.
    pub fn detail_or(mut self, detail: impl FnOnce() -> String) -> Self {
        if self.detail.is_none() {
            self.detail = Some(detail());
        }
        self
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "{} ({detail})", self.code),
            None => f.write_str(self.code),
        }
    }
}

impl Error for CoreError {}

/// Result alias used throughout the core.
pub type CoreResult<T> = Result<T, CoreError>;

/// Option and value rules. Codes are prefixed by the option they reject.
pub mod codes {
    // Option parsing and cross-option constraints.
    pub const OPTION_PURPOSE: &str = "options.purpose";
    pub const OPTION_CODEC: &str = "options.codec";
    pub const OPTION_OUTPUT: &str = "options.output";
    pub const OPTION_TRANSFORM: &str = "options.transform";
    pub const OPTION_PROFILE: &str = "options.profile";
    pub const OPTION_SAMPLING: &str = "options.sampling";
    pub const OPTION_VARIANT: &str = "options.variant";
    pub const OPTION_AUDIO: &str = "options.audio";
    pub const OPTION_CONTAINER: &str = "options.container";
    pub const OPTION_RANGE: &str = "options.range";
    pub const OPTION_DEPTH: &str = "options.depth";
    pub const OPTION_DYNAMIC_HDR: &str = "options.dynamic_hdr";
    pub const OPTION_SOURCE_KIND: &str = "options.source_kind";

    // Output policy.
    pub const POLICY_ALPHA_SAMPLING: &str = "policy.alpha_sampling";
    pub const POLICY_SAMPLING_CHOICE: &str = "policy.sampling_choice";
    pub const POLICY_SAMPLING_LIMIT: &str = "policy.sampling_limit";
    pub const POLICY_SAMPLING_SOFTWARE: &str = "policy.sampling_software";
    pub const POLICY_DEPTH_CHOICE: &str = "policy.depth_choice";
    pub const POLICY_DEPTH_LIMIT: &str = "policy.depth_limit";
    pub const POLICY_HDR_DEPTH: &str = "policy.hdr_depth";

    // Source description and audio eligibility.
    pub const SOURCE_TIME_BASE: &str = "source.time_base";
    pub const SOURCE_FRAME_INDEX: &str = "source.frame_index";
    pub const AUDIO_EMPTY: &str = "audio.empty";
    pub const AUDIO_NOT_LOSSLESS: &str = "audio.not_lossless";
    pub const AUDIO_UNKNOWN_DEPTH: &str = "audio.unknown_depth";
    pub const AUDIO_LAYOUT: &str = "audio.channel_layout";

    // Transform and dynamic HDR compatibility.
    pub const TRANSFORM_PROFILE_REQUIRED: &str = "transform.profile_required";
    pub const TRANSFORM_TARGET_MISMATCH: &str = "transform.target_mismatch";
    pub const TRANSFORM_REQUIRES_LOG: &str = "transform.requires_log";
    pub const TRANSFORM_REQUIRES_DEPTH: &str = "transform.requires_depth";
    pub const TRANSFORM_HDR_TO_SDR: &str = "transform.hdr_to_sdr";
    pub const TRANSFORM_UNDETERMINED: &str = "transform.undetermined";
    pub const TRANSFORM_SDR_TO_HDR: &str = "transform.sdr_to_hdr";
    pub const TRANSFORM_HDR_TO_SDR_REQUIRED: &str = "transform.hdr_to_sdr_required";
    pub const TRANSFORM_GROUP: &str = "transform.group";
    pub const RAW_INTERPRETATION: &str = "raw.interpretation";
    pub const RAW_DECODER_VERSION: &str = "raw.decoder_version";
    pub const RAW_DECODER_MISSING: &str = "raw.decoder_missing";
    pub const DVHDR_INTERPRETATION: &str = "dvhdr.interpretation";
    pub const DVHDR_LAYER: &str = "dvhdr.unsupported_layer";
    pub const DVHDR_COMPATIBILITY: &str = "dvhdr.compatibility_layer";
    pub const DVHDR_NEEDS_ARCHIVE: &str = "dvhdr.needs_software_archive";
    pub const DVHDR_HDR10PLUS_CODEC: &str = "dvhdr.hdr10plus_codec";
    pub const DVHDR_PROFILE5_CODEC: &str = "dvhdr.profile5_codec";
    pub const DVHDR_FORMAT: &str = "dvhdr.format";
    pub const DVHDR_DISPLAY_MATRIX: &str = "dvhdr.display_matrix";

    // Timeline.
    pub const TIMELINE_RANGE: &str = "timeline.range";

    // Presets, capability and planning.
    pub const PRESET_INVALID: &str = "preset.invalid";
    pub const PRESET_UNKNOWN: &str = "preset.unknown";
    pub const PRESET_VARIANT: &str = "preset.variant";
    pub const CAPABILITY_NO_PATH: &str = "capability.no_path";
    pub const PLAN_PREVIEW: &str = "plan.preview_size";
    pub const PLAN_DURATION: &str = "plan.duration";
    pub const PLAN_MOTION_SIZE: &str = "plan.motion_size";
    pub const CACHE_IDENTITY: &str = "cache.identity";
    pub const OUTPUT_SYMLINK: &str = "output.symlink";
    pub const OUTPUT_SAME_AS_INPUT: &str = "output.same_as_input";
    pub const OUTPUT_EXISTS: &str = "output.exists";

    // Native boundary.
    pub const FFI_NULL: &str = "ffi.null_argument";
    pub const FFI_UTF8: &str = "ffi.invalid_utf8";
    pub const FFI_JSON: &str = "ffi.invalid_json";
    pub const FFI_ENVELOPE: &str = "ffi.invalid_request";
    pub const FFI_OP: &str = "ffi.unknown_operation";
    pub const FFI_PANIC: &str = "ffi.panic";
    pub const FFI_STATE: &str = "ffi.state";
}

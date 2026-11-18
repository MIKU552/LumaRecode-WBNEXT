//! Backend-neutral task planning.
//!
//! The plan describes *what* must happen — decode, develop, tone map, convert,
//! encode, mux, verify — and where frames live between steps. Command-line
//! construction belongs to the backend that will run it, so FFmpeg flags never
//! become the platform-independent definition of a LumaRecode job.

use crate::capability::Backend;
use crate::domain::{EncodeOptions, Output, Purpose, ResolvedEncode, SourceKind, Transform};
use crate::error::{CoreError, CoreResult, codes};
use crate::policy;
use crate::source::SourceFormat;

/// Where decoded frames come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodePath {
    Hardware,
    Software,
}

impl DecodePath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hardware => "hardware",
            Self::Software => "software",
        }
    }
}

/// Where frames live while filters run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameTransport {
    /// Device memory; avoids download/upload but restricts filter support.
    GpuSurface,
    /// System memory; required by LUT, tone mapping and most software filters.
    SystemMemory,
}

impl FrameTransport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GpuSurface => "gpu_surface",
            Self::SystemMemory => "system_memory",
        }
    }
}

/// One ordered planning step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Decode,
    DevelopRaw,
    ToneMap,
    ApplyLut,
    ConvertRange,
    Scale,
    Encode,
    Mux,
    Verify,
}

impl StepKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Decode => "decode",
            Self::DevelopRaw => "develop_raw",
            Self::ToneMap => "tone_map",
            Self::ApplyLut => "apply_lut",
            Self::ConvertRange => "convert_range",
            Self::Scale => "scale",
            Self::Encode => "encode",
            Self::Mux => "mux",
            Self::Verify => "verify",
        }
    }
}

/// A step plus the memory it operates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanStep {
    pub kind: StepKind,
    pub transport: FrameTransport,
}

/// Inputs that change the shape of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRequest {
    pub options: EncodeOptions,
    pub source: SourceFormat,
    pub backend: Backend,
    /// A frame range was requested, so decode must be frame-accurate.
    pub frame_window: bool,
    pub motion_photo: bool,
    /// Zero for delivery; a positive preview viewport adds a scale step.
    pub preview_size: u32,
}

/// The resolved plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub resolved: ResolvedEncode,
    pub decode: DecodePath,
    pub transport: FrameTransport,
    pub steps: Vec<PlanStep>,
    /// Diagnostic tokens explaining non-obvious decisions.
    pub notes: Vec<&'static str>,
}

impl Plan {
    /// Steps in execution order, without transports.
    pub fn kinds(&self) -> Vec<StepKind> {
        self.steps.iter().map(|step| step.kind).collect()
    }
}

/// Builds a plan for one source, request and backend.
pub fn build(request: &PlanRequest) -> CoreResult<Plan> {
    let resolved = policy::validate(&request.options, &request.source)?;
    let options = &resolved.options;
    let source_range = policy::source_range(&request.source, None);
    // A transfer-function or range change must be rendered, which forces
    // system memory even on a hardware path.
    let color_change = match options.output {
        Output::Sdr => resolved.source_kind != SourceKind::Sdr,
        Output::Hdr => resolved.source_kind == SourceKind::Sdr,
    };
    let software_frames = options.transform != Transform::None
        || options.purpose == Purpose::Archive
        || options.cpu_decode
        || color_change
        || request.source.display_matrix
        || request.source.dynamic_hdr.is_present()
        || request.source.raw
        || request.frame_window
        || request.motion_photo
        || request.backend == Backend::Software;
    let decode = if request.backend.is_hardware() && !options.cpu_decode && !software_frames {
        DecodePath::Hardware
    } else {
        DecodePath::Software
    };
    let transport = if software_frames || decode == DecodePath::Software {
        FrameTransport::SystemMemory
    } else {
        FrameTransport::GpuSurface
    };
    let mut steps = vec![PlanStep {
        kind: StepKind::Decode,
        transport,
    }];
    if request.source.raw {
        steps.push(PlanStep {
            kind: StepKind::DevelopRaw,
            transport: FrameTransport::SystemMemory,
        });
    }
    if options.transform == Transform::HdrSdr {
        steps.push(PlanStep {
            kind: StepKind::ToneMap,
            transport: FrameTransport::SystemMemory,
        });
    }
    if matches!(options.transform, Transform::LogHdr | Transform::LogSdr) {
        steps.push(PlanStep {
            kind: StepKind::ApplyLut,
            transport: FrameTransport::SystemMemory,
        });
    }
    if resolved.format.color_range != source_range || options.transform != Transform::None {
        steps.push(PlanStep {
            kind: StepKind::ConvertRange,
            transport: FrameTransport::SystemMemory,
        });
    }
    if request.preview_size > 0 {
        steps.push(PlanStep {
            kind: StepKind::Scale,
            transport: FrameTransport::SystemMemory,
        });
    }
    steps.push(PlanStep {
        kind: StepKind::Encode,
        transport,
    });
    steps.push(PlanStep {
        kind: StepKind::Mux,
        transport: FrameTransport::SystemMemory,
    });
    steps.push(PlanStep {
        kind: StepKind::Verify,
        transport: FrameTransport::SystemMemory,
    });

    let mut notes = Vec::new();
    if software_frames && request.backend.is_hardware() {
        notes.push("software_frames_on_hardware_backend");
    }
    if decode == DecodePath::Software && request.backend.is_hardware() {
        notes.push(if options.cpu_decode {
            "software_decode_requested"
        } else {
            "software_decode_required"
        });
    }
    if request.frame_window {
        notes.push("frame_accurate_decode");
    }
    if request.motion_photo {
        notes.push("motion_photo_pairing");
    }
    if request.preview_size > 0 {
        notes.push("preview_viewport");
    }
    if resolved.source_kind == SourceKind::Raw {
        notes.push("raw_development");
    }
    if policy::preserve_dynamic_hdr(options, &request.source) {
        notes.push("dynamic_hdr_preserved");
    }

    Ok(Plan {
        resolved,
        decode,
        transport,
        steps,
        notes,
    })
}

/// Rejects a zero or non-finite duration before it reaches a backend.
pub fn validate_duration(seconds: Option<f64>) -> CoreResult<()> {
    if let Some(seconds) = seconds
        && !(seconds.is_finite() && seconds > 0.0)
    {
        return Err(CoreError::new(codes::PLAN_DURATION));
    }
    Ok(())
}

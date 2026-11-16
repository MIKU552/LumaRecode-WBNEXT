//! What each backend needs before it can be used.
//!
//! The capability rules answer "can this family encode that"; this table answers "what must
//! the host ask the machine". Keeping it data instead of scattered strings makes the
//! detection code a loop over requirements and lets the docs stay in step with the code.

use crate::capability::Backend;

/// One signal the host must collect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// `ffmpeg -encoders` lists an encoder of this family.
    EncoderList,
    /// A vendor adapter or device is present.
    AdapterList,
    /// The installed driver is new enough for the encoder generation.
    DriverVersion,
    /// A system-provided encoder (Media Foundation, VideoToolbox).
    SystemEncoder,
}

/// Detection recipe for one backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    pub backend: Backend,
    pub probe: Probe,
    /// Short explanation shown in diagnostics.
    pub note: &'static str,
}

/// Every backend, in the order diagnostics should list them.
pub const REQUIREMENTS: [Requirement; 6] = [
    Requirement {
        backend: Backend::Qsv,
        probe: Probe::EncoderList,
        note: "Intel graphics or Arc adapter, then the QSV encoders in ffmpeg",
    },
    Requirement {
        backend: Backend::Nvenc,
        probe: Probe::DriverVersion,
        note: "NVIDIA driver new enough for the encoder generation",
    },
    Requirement {
        backend: Backend::Amf,
        probe: Probe::AdapterList,
        note: "AMD adapter exposing the AMF encoders",
    },
    Requirement {
        backend: Backend::MediaFoundation,
        probe: Probe::SystemEncoder,
        note: "Windows system encoder; capability follows the driver",
    },
    Requirement {
        backend: Backend::VideoToolbox,
        probe: Probe::SystemEncoder,
        note: "Apple silicon or T2 system encoder",
    },
    Requirement {
        backend: Backend::Software,
        probe: Probe::EncoderList,
        note: "Bundled by the FFmpeg build (libx264, libx265, libsvtav1)",
    },
];

/// Requirement for one backend.
pub fn requirement(backend: Backend) -> Option<Requirement> {
    REQUIREMENTS
        .into_iter()
        .find(|requirement| requirement.backend == backend)
}

use crate::capability::backend_for_encoder;
use crate::domain::Codec;

/// How a vendor parameter is validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    /// Free integer within the encoder's own bounds.
    Integer,
    /// One of a fixed list of spellings.
    Choice(&'static [&'static str]),
    /// Presence-only switch.
    Flag,
}

/// One encoder-specific parameter a vendor accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameter {
    pub name: &'static str,
    pub kind: ParameterKind,
}

const PRESETS: &[&str] = &["fast", "medium", "slow", "slower"];
const NVENC_PRESETS: &[&str] = &["p1", "p2", "p3", "p4", "p5", "p6", "p7"];
const NVENC_TUNING: &[&str] = &["hq", "ll", "ull", "lossless"];
const NVENC_RATES: &[&str] = &["constqp", "vbr", "cbr"];
const AMF_QUALITY: &[&str] = &["speed", "balanced", "quality"];
const AMF_USAGE: &[&str] = &["transcoding", "lowlatency", "ultralowlatency", "webcam"];

const QSV_PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "preset",
        kind: ParameterKind::Choice(PRESETS),
    },
    Parameter {
        name: "async_depth",
        kind: ParameterKind::Integer,
    },
];
const NVENC_PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "preset",
        kind: ParameterKind::Choice(NVENC_PRESETS),
    },
    Parameter {
        name: "tune",
        kind: ParameterKind::Choice(NVENC_TUNING),
    },
    Parameter {
        name: "rc",
        kind: ParameterKind::Choice(NVENC_RATES),
    },
];
const AMF_PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "quality",
        kind: ParameterKind::Choice(AMF_QUALITY),
    },
    Parameter {
        name: "usage",
        kind: ParameterKind::Choice(AMF_USAGE),
    },
];
const VIDEOTOOLBOX_PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "realtime",
        kind: ParameterKind::Flag,
    },
    Parameter {
        name: "allow_sw",
        kind: ParameterKind::Flag,
    },
];
const SOFTWARE_PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "preset",
        kind: ParameterKind::Choice(PRESETS),
    },
    Parameter {
        name: "crf",
        kind: ParameterKind::Integer,
    },
];
const MEDIA_FOUNDATION_PARAMETERS: &[Parameter] = &[Parameter {
    name: "rate_control",
    kind: ParameterKind::Choice(&["cbr", "vbr"]),
}];

/// Parameters a backend accepts for one codec.
///
/// The table is deliberately small: it lists what this release writes, not every key the
/// encoder understands. AV1 has no separate entry yet because no vendor exposes one.
pub fn video_parameters(backend: Backend, codec: Codec) -> &'static [Parameter] {
    match (backend, codec) {
        (Backend::Qsv, _) => QSV_PARAMETERS,
        (Backend::Nvenc, _) => NVENC_PARAMETERS,
        (Backend::Amf, _) => AMF_PARAMETERS,
        (Backend::VideoToolbox, _) => VIDEOTOOLBOX_PARAMETERS,
        (Backend::MediaFoundation, _) => MEDIA_FOUNDATION_PARAMETERS,
        (Backend::Software, _) => SOFTWARE_PARAMETERS,
    }
}

/// How sure the host can be about one backend on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// An encoder is present and the requirement does not need device evidence.
    Available,
    /// No encoder is present.
    Unavailable,
    /// An encoder is present, but the family needs device or driver evidence the host
    /// has not collected yet. Never report this as available: a compiled-in encoder
    /// does not prove the hardware exists.
    Unverified,
}

impl Availability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Unverified => "unverified",
        }
    }
}

/// One backend's state on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub backend: Backend,
    pub availability: Availability,
    pub encoders: Vec<String>,
}

/// Maps the encoders FFmpeg reports to backend states.
pub fn detect(encoders: &[String]) -> Vec<BackendStatus> {
    let mut statuses = Vec::with_capacity(Backend::ALL.len());
    for backend in Backend::ALL {
        let mut matched: Vec<String> = encoders
            .iter()
            .filter(|encoder| backend_for_encoder(encoder) == Some(backend))
            .cloned()
            .collect();
        matched.sort();
        let availability = if matched.is_empty() {
            Availability::Unavailable
        } else {
            match requirement(backend).map(|requirement| requirement.probe) {
                Some(Probe::EncoderList) | Some(Probe::SystemEncoder) => Availability::Available,
                _ => Availability::Unverified,
            }
        };
        statuses.push(BackendStatus {
            backend,
            availability,
            encoders: matched,
        });
    }

    statuses
}

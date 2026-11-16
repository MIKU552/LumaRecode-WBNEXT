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

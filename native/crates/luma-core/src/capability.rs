//! Device capabilities and backend selection.
//!
//! Capabilities are described per device *and* per parameter combination. A
//! device that lists HEVC but cannot do 10-bit 4:2:0 for a given driver is not
//! a HEVC 10-bit path, so the description keeps those limits explicit instead
//! of reducing them to a boolean.

use crate::domain::{Codec, Output, Purpose, Sampling};
use crate::error::{CoreError, CoreResult, codes};

/// Execution backend family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Backend {
    Qsv,
    Amf,
    Nvenc,
    MediaFoundation,
    VideoToolbox,
    Software,
}

impl Backend {
    pub const ALL: [Self; 6] = [
        Self::Qsv,
        Self::Amf,
        Self::Nvenc,
        Self::MediaFoundation,
        Self::VideoToolbox,
        Self::Software,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Qsv => "qsv",
            Self::Amf => "amf",
            Self::Nvenc => "nvenc",
            Self::MediaFoundation => "media_foundation",
            Self::VideoToolbox => "videotoolbox",
            Self::Software => "software",
        }
    }

    /// `false` only for the CPU encoders (x264/x265/SVT-AV1).
    pub const fn is_hardware(self) -> bool {
        !matches!(self, Self::Software)
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|backend| backend.as_str() == value)
    }
}

/// Detection outcome. `Unsupported` is a durable fact about the device;
/// `TemporarilyUnavailable` is a session condition and must never be cached as
/// a permanent negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityStatus {
    Probing,
    Available,
    Unsupported,
    KnownIssue,
    TemporarilyUnavailable,
}

impl CapabilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Probing => "probing",
            Self::Available => "available",
            Self::Unsupported => "unsupported",
            Self::KnownIssue => "known_issue",
            Self::TemporarilyUnavailable => "temporarily_unavailable",
        }
    }

    /// Only `Available` participates in automatic selection.
    pub const fn selectable(self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Probing,
            Self::Available,
            Self::Unsupported,
            Self::KnownIssue,
            Self::TemporarilyUnavailable,
        ]
        .into_iter()
        .find(|status| status.as_str() == value)
    }
}

/// How much evidence backs a capability entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verification {
    /// Declared by the vendor interface only.
    Declared,
    /// A probe query answered.
    Probed,
    /// A bounded initialization task produced a valid output.
    ShortTask,
}

impl Verification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Probed => "probed",
            Self::ShortTask => "short_task",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "declared" => Some(Self::Declared),
            "probed" => Some(Self::Probed),
            "short_task" => Some(Self::ShortTask),
            _ => None,
        }
    }
}

/// One encoder family on one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderCapability {
    pub codec: Codec,
    /// Highest bit depth accepted by this encoder on this device.
    pub max_depth: u8,
    pub samplings: Vec<Sampling>,
    /// Accepts HDR transfer functions and static metadata.
    pub hdr: bool,
    /// Can carry dynamic HDR (Dolby Vision RPU / HDR10+ metadata).
    pub dynamic_hdr: bool,
    pub verification: Verification,
}

impl EncoderCapability {
    fn supports(&self, request: &Requirement) -> bool {
        self.codec == request.codec
            && self.max_depth >= request.depth
            && self.samplings.contains(&request.sampling)
            && (!request.output_is_hdr() || self.hdr)
            && (!request.dynamic_hdr || self.dynamic_hdr)
    }
}

/// A GPU or CPU encoder set with its own identity and driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub backend: Backend,
    pub status: CapabilityStatus,
    pub driver: Option<String>,
    pub encoders: Vec<EncoderCapability>,
}

/// What the user asked the output to be, in capability terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub purpose: Purpose,
    pub codec: Codec,
    pub output: Output,
    pub depth: u8,
    pub sampling: Sampling,
    pub dynamic_hdr: bool,
}

impl Requirement {
    fn output_is_hdr(&self) -> bool {
        self.output == Output::Hdr
    }
}

/// Why a candidate ranked the way it did. Tokens are diagnostic and stable.
pub type Reason = &'static str;

/// The chosen execution path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub device_id: String,
    pub device_name: String,
    pub backend: Backend,
    pub score: i32,
    pub reasons: Vec<Reason>,
    /// `true` when no hardware candidate matched and software was substituted.
    pub fallback: bool,
}

fn score(
    device: &Device,
    encoder: &EncoderCapability,
    request: &Requirement,
) -> (i32, Vec<Reason>) {
    let mut total = 0;
    let mut reasons = Vec::new();
    match (request.purpose, device.backend.is_hardware()) {
        (Purpose::Share, true) => {
            total += 40;
            reasons.push("hardware_encode_for_share");
        }
        (Purpose::Archive, false) => {
            total += 40;
            reasons.push("software_encode_for_archive");
        }
        (Purpose::Share, false) => {
            total += 10;
            reasons.push("software_fallback_cost");
        }
        (Purpose::Archive, true) => {
            // Hardware archive is allowed but must not outrank a software path
            // that preserves precision and metadata.
            total += 20;
            reasons.push("hardware_archive_cost");
        }
    }
    if encoder.max_depth == request.depth {
        total += 5;
        reasons.push("exact_depth");
    } else {
        total += 2;
        reasons.push("depth_headroom");
    }
    if encoder.samplings.len() == 1 {
        total += 1;
        reasons.push("single_sampling");
    }
    if encoder.verification == Verification::ShortTask {
        total += 5;
        reasons.push("short_task_verified");
    } else if encoder.verification == Verification::Probed {
        total += 2;
        reasons.push("probe_verified");
    }
    if request.dynamic_hdr {
        total += 4;
        reasons.push("dynamic_hdr_capable");
    }
    (total, reasons)
}

/// Picks the best available path for a request.
///
/// Ranking is a documented cost model, not a vendor order: share prefers the
/// hardware encoder that avoids a full-frame download, archive prefers the
/// software encoder that keeps precision, and verified capabilities break ties.
pub fn select(devices: &[Device], request: &Requirement) -> CoreResult<Selection> {
    let mut candidates: Vec<(i32, &Device, Vec<Reason>)> = Vec::new();
    for device in devices {
        if !device.status.selectable() {
            continue;
        }
        for encoder in &device.encoders {
            if encoder.supports(request) {
                let (score, reasons) = score(device, encoder, request);
                candidates.push((score, device, reasons));
            }
        }
    }
    // Deterministic tie-break: higher score, then device id.
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    if let Some((best_score, device, best_reasons)) = candidates.first() {
        let fallback = !device.backend.is_hardware();
        let mut reasons = best_reasons.clone();
        if fallback {
            reasons.push("software_only_path");
        }
        return Ok(Selection {
            device_id: device.id.clone(),
            device_name: device.name.clone(),
            backend: device.backend,
            score: *best_score,
            reasons,
            fallback,
        });
    }
    let detail = format!(
        "{}:{}:{}:{}",
        request.codec.as_str(),
        request.output.as_str(),
        request.depth,
        request.sampling.label()
    );
    Err(CoreError::with_detail(codes::CAPABILITY_NO_PATH, detail))
}

/// Aggregated limits used to hide options the machine cannot honour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySummary {
    pub backends: Vec<Backend>,
    pub max_depth: u8,
    pub samplings: Vec<Sampling>,
    pub hdr: bool,
    pub dynamic_hdr: bool,
}

/// Summarizes what any available device can do for one codec.
pub fn summarize(devices: &[Device], codec: Codec) -> CapabilitySummary {
    let mut backends = Vec::new();
    let mut samplings: Vec<Sampling> = Vec::new();
    let mut max_depth = 0;
    let mut hdr = false;
    let mut dynamic_hdr = false;
    for device in devices.iter().filter(|device| device.status.selectable()) {
        for encoder in device
            .encoders
            .iter()
            .filter(|encoder| encoder.codec == codec)
        {
            if !backends.contains(&device.backend) {
                backends.push(device.backend);
            }
            max_depth = max_depth.max(encoder.max_depth);
            for sampling in &encoder.samplings {
                if !samplings.contains(sampling) {
                    samplings.push(*sampling);
                }
            }
            hdr |= encoder.hdr;
            dynamic_hdr |= encoder.dynamic_hdr;
        }
    }
    backends.sort();
    samplings.sort_by_key(|sampling| sampling.label());
    CapabilitySummary {
        backends,
        max_depth,
        samplings,
        hdr,
        dynamic_hdr,
    }
}

/// Wire form of one device.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DeviceDto {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub driver: Option<String>,
    #[serde(default)]
    pub encoders: Vec<EncoderDto>,
}

/// Wire form of one encoder capability.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct EncoderDto {
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub max_depth: Option<u8>,
    #[serde(default)]
    pub samplings: Vec<String>,
    #[serde(default)]
    pub hdr: bool,
    #[serde(default)]
    pub dynamic_hdr: bool,
    #[serde(default)]
    pub verification: Option<String>,
}

impl DeviceDto {
    /// Converts a probe snapshot, skipping entries with unknown backends.
    pub fn into_device(self) -> Option<Device> {
        let backend = Backend::parse(self.backend.as_deref()?)?;
        let status = self
            .status
            .as_deref()
            .and_then(CapabilityStatus::parse)
            .unwrap_or(CapabilityStatus::Available);
        let encoders = self
            .encoders
            .into_iter()
            .filter_map(EncoderDto::into_capability)
            .collect();
        Some(Device {
            id: self.id.unwrap_or_else(|| backend.as_str().to_owned()),
            name: self.name.unwrap_or_else(|| backend.as_str().to_owned()),
            backend,
            status,
            driver: self.driver,
            encoders,
        })
    }
}

impl EncoderDto {
    fn into_capability(self) -> Option<EncoderCapability> {
        let codec = Codec::parse(self.codec.as_deref()?).ok()?;
        let samplings = self
            .samplings
            .iter()
            .filter_map(|label| Sampling::from_label(label))
            .collect::<Vec<_>>();
        if samplings.is_empty() {
            return None;
        }
        Some(EncoderCapability {
            codec,
            max_depth: self.max_depth.unwrap_or(8),
            samplings,
            hdr: self.hdr,
            dynamic_hdr: self.dynamic_hdr,
            verification: self
                .verification
                .as_deref()
                .and_then(Verification::parse)
                .unwrap_or(Verification::Declared),
        })
    }
}

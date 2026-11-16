//! Backend selection tests: deterministic ranking, fallback and option hiding.

use luma_core::capability::{
    Backend, CapabilityStatus, Device, EncoderCapability, Requirement, Verification, select,
    summarize,
};
use luma_core::domain::{Codec, Output, Purpose, Sampling};

fn encoder(
    codec: Codec,
    max_depth: u8,
    samplings: &[Sampling],
    hdr: bool,
    dynamic_hdr: bool,
    verification: Verification,
) -> EncoderCapability {
    EncoderCapability {
        codec,
        max_depth,
        samplings: samplings.to_vec(),
        hdr,
        dynamic_hdr,
        verification,
    }
}

fn device(
    id: &str,
    backend: Backend,
    status: CapabilityStatus,
    encoders: Vec<EncoderCapability>,
) -> Device {
    Device {
        id: id.to_owned(),
        name: id.to_owned(),
        backend,
        status,
        driver: None,
        encoders,
    }
}

fn hevc_only(
    max_depth: u8,
    hdr: bool,
    dynamic_hdr: bool,
    verification: Verification,
) -> Vec<EncoderCapability> {
    vec![encoder(
        Codec::Hevc,
        max_depth,
        &[Sampling::Yuv420],
        hdr,
        dynamic_hdr,
        verification,
    )]
}

fn requirement(purpose: Purpose, codec: Codec, output: Output, depth: u8) -> Requirement {
    Requirement {
        purpose,
        codec,
        output,
        depth,
        sampling: Sampling::Yuv420,
        dynamic_hdr: false,
    }
}

#[test]
fn share_prefers_hardware_and_archive_prefers_software() {
    let devices = vec![
        device(
            "intel-igpu",
            Backend::Qsv,
            CapabilityStatus::Available,
            hevc_only(10, true, false, Verification::ShortTask),
        ),
        device(
            "cpu",
            Backend::Software,
            CapabilityStatus::Available,
            hevc_only(12, true, false, Verification::Probed),
        ),
    ];

    let selection = select(
        &devices,
        &requirement(Purpose::Share, Codec::Hevc, Output::Sdr, 8),
    )
    .expect("share path");
    assert_eq!(selection.backend, Backend::Qsv);
    assert!(!selection.fallback);
    assert!(selection.reasons.contains(&"hardware_encode_for_share"));

    let selection = select(
        &devices,
        &requirement(Purpose::Archive, Codec::Hevc, Output::Sdr, 10),
    )
    .expect("archive path");
    assert_eq!(selection.backend, Backend::Software);
    assert!(selection.reasons.contains(&"software_encode_for_archive"));
}

#[test]
fn unavailable_devices_are_never_selected() {
    let devices = vec![
        device(
            "busy-gpu",
            Backend::Nvenc,
            CapabilityStatus::TemporarilyUnavailable,
            hevc_only(10, true, true, Verification::ShortTask),
        ),
        device(
            "broken-gpu",
            Backend::Amf,
            CapabilityStatus::KnownIssue,
            hevc_only(10, true, true, Verification::ShortTask),
        ),
        device(
            "cpu",
            Backend::Software,
            CapabilityStatus::Available,
            hevc_only(10, true, false, Verification::Probed),
        ),
    ];
    let selection = select(
        &devices,
        &requirement(Purpose::Share, Codec::Hevc, Output::Sdr, 8),
    )
    .expect("software remains");
    assert_eq!(selection.backend, Backend::Software);
    assert!(selection.fallback);
    assert!(selection.reasons.contains(&"software_only_path"));
}

#[test]
fn depth_and_transfer_requirements_filter_candidates() {
    let devices = vec![device(
        "intel-igpu",
        Backend::Qsv,
        CapabilityStatus::Available,
        hevc_only(10, false, false, Verification::Probed),
    )];
    // 12-bit is beyond the encoder, and HDR is beyond this entry.
    assert_eq!(
        select(
            &devices,
            &requirement(Purpose::Archive, Codec::Hevc, Output::Sdr, 12)
        )
        .expect_err("depth limit")
        .code(),
        "capability.no_path"
    );
    assert_eq!(
        select(
            &devices,
            &requirement(Purpose::Archive, Codec::Hevc, Output::Hdr, 10)
        )
        .expect_err("no HDR")
        .code(),
        "capability.no_path"
    );
}

#[test]
fn dynamic_hdr_requires_an_explicit_capability() {
    let devices = vec![device(
        "cpu",
        Backend::Software,
        CapabilityStatus::Available,
        hevc_only(10, true, false, Verification::ShortTask),
    )];
    let mut request = requirement(Purpose::Archive, Codec::Hevc, Output::Hdr, 10);
    assert!(select(&devices, &request).is_ok());
    request.dynamic_hdr = true;
    assert_eq!(
        select(&devices, &request)
            .expect_err("metadata path")
            .code(),
        "capability.no_path"
    );
}

#[test]
fn ties_break_on_device_id_so_results_are_reproducible() {
    let devices = vec![
        device(
            "b-gpu",
            Backend::Nvenc,
            CapabilityStatus::Available,
            hevc_only(10, true, false, Verification::ShortTask),
        ),
        device(
            "a-gpu",
            Backend::Qsv,
            CapabilityStatus::Available,
            hevc_only(10, true, false, Verification::ShortTask),
        ),
    ];
    let selection = select(
        &devices,
        &requirement(Purpose::Share, Codec::Hevc, Output::Sdr, 10),
    )
    .expect("share path");
    assert_eq!(selection.device_id, "a-gpu");
}

#[test]
fn summary_reports_what_the_machine_can_offer() {
    let devices = vec![
        device(
            "intel-igpu",
            Backend::Qsv,
            CapabilityStatus::Available,
            vec![encoder(
                Codec::Hevc,
                10,
                &[Sampling::Yuv420],
                true,
                false,
                Verification::ShortTask,
            )],
        ),
        device(
            "cpu",
            Backend::Software,
            CapabilityStatus::Available,
            vec![encoder(
                Codec::Hevc,
                12,
                &[Sampling::Yuv420, Sampling::Yuv444],
                true,
                true,
                Verification::Probed,
            )],
        ),
        device(
            "offline-gpu",
            Backend::Amf,
            CapabilityStatus::Unsupported,
            vec![encoder(
                Codec::Av1,
                10,
                &[Sampling::Yuv420],
                true,
                true,
                Verification::Declared,
            )],
        ),
    ];
    let summary = summarize(&devices, Codec::Hevc);
    assert_eq!(summary.backends, vec![Backend::Qsv, Backend::Software]);
    assert_eq!(summary.max_depth, 12);
    assert_eq!(summary.samplings, vec![Sampling::Yuv420, Sampling::Yuv444]);
    assert!(summary.hdr);
    assert!(summary.dynamic_hdr);

    // AV1 exists only on the unsupported device, so nothing is offered.
    let av1 = summarize(&devices, Codec::Av1);
    assert!(av1.backends.is_empty());
    assert_eq!(av1.max_depth, 0);
}

#[test]
fn wire_snapshots_ignore_unknown_backends() {
    let snapshot = r#"[
        {"id":"igpu","backend":"qsv","status":"available","encoders":[
            {"codec":"hevc","max_depth":10,"samplings":["420"],"hdr":true,"verification":"short_task"}
        ]},
        {"id":"future","backend":"mystery","encoders":[]}
    ]"#;
    let devices: Vec<luma_core::capability::DeviceDto> =
        serde_json::from_str(snapshot).expect("snapshot parses");
    let devices: Vec<Device> = devices
        .into_iter()
        .filter_map(|entry| entry.into_device())
        .collect();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].backend, Backend::Qsv);
    assert_eq!(devices[0].encoders[0].max_depth, 10);
    assert_eq!(devices[0].encoders[0].verification, Verification::ShortTask);
}

#[test]
fn encoder_names_map_to_their_backend() {
    use luma_core::capability::{Backend, backend_for_encoder};

    assert_eq!(backend_for_encoder("hevc_qsv"), Some(Backend::Qsv));
    assert_eq!(backend_for_encoder("hevc_amf"), Some(Backend::Amf));
    assert_eq!(backend_for_encoder("av1_nvenc"), Some(Backend::Nvenc));
    assert_eq!(
        backend_for_encoder("hevc_mf"),
        Some(Backend::MediaFoundation)
    );
    assert_eq!(
        backend_for_encoder("h264_videotoolbox"),
        Some(Backend::VideoToolbox)
    );
    assert_eq!(backend_for_encoder("libx265"), Some(Backend::Software));
    assert_eq!(backend_for_encoder("libsvtav1"), Some(Backend::Software));
    assert_eq!(backend_for_encoder("mystery_encoder"), None);
}

#[test]
fn backends_refuse_what_their_family_does_not_support() {
    use luma_core::capability::{supports, Backend};
    use luma_core::Codec;

    assert!(supports(Backend::Software, Codec::Hevc, 10, "422"));
    assert!(supports(Backend::Software, Codec::Av1, 10, "420"));
    assert!(!supports(Backend::Software, Codec::Av1, 10, "422"));

    assert!(supports(Backend::Qsv, Codec::Hevc, 10, "420"));
    assert!(!supports(Backend::Qsv, Codec::Hevc, 12, "420"));
    assert!(!supports(Backend::Qsv, Codec::Hevc, 10, "422"));

    assert!(!supports(Backend::Amf, Codec::Av1, 10, "420"));
    assert!(!supports(Backend::MediaFoundation, Codec::Hevc, 10, "420"));
    assert!(supports(Backend::VideoToolbox, Codec::Hevc, 10, "420"));
    assert!(!supports(Backend::Nvenc, Codec::Avc, 8, "444"));
}

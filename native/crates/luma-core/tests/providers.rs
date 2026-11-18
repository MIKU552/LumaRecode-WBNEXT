use luma_core::capability::Backend;
use luma_core::providers::{REQUIREMENTS, requirement};

#[test]
fn every_backend_has_exactly_one_requirement() {
    for backend in Backend::ALL {
        let matches = REQUIREMENTS
            .iter()
            .filter(|requirement| requirement.backend == backend)
            .count();
        assert_eq!(matches, 1, "{backend:?}");
        assert!(requirement(backend).is_some());
    }
}

#[test]
fn requirements_explain_what_to_check() {
    for requirement in REQUIREMENTS {
        assert!(
            !requirement.note.trim().is_empty(),
            "{:?}",
            requirement.backend
        );
    }
}

#[test]
fn software_is_detected_through_the_encoder_list() {
    let software = requirement(Backend::Software).expect("software requirement");
    assert_eq!(software.probe, luma_core::providers::Probe::EncoderList);
}

#[test]
fn every_backend_publishes_parameters() {
    use luma_core::capability::Backend;
    use luma_core::domain::Codec;
    use luma_core::providers::video_parameters;

    for backend in Backend::ALL {
        let parameters = video_parameters(backend, Codec::Hevc);
        assert!(!parameters.is_empty(), "{backend:?}");
        for parameter in parameters {
            assert!(!parameter.name.is_empty());
        }
    }
}

#[test]
fn vendor_specific_choices_are_present() {
    use luma_core::capability::Backend;
    use luma_core::domain::Codec;
    use luma_core::providers::{ParameterKind, video_parameters};

    let amf = video_parameters(Backend::Amf, Codec::Hevc);
    assert!(amf.iter().any(|parameter| parameter.name == "quality"));
    assert!(matches!(
        amf[0].kind,
        ParameterKind::Choice(choices) if choices.contains(&"quality")
    ));

    let nvenc = video_parameters(Backend::Nvenc, Codec::Hevc);
    assert!(nvenc.iter().any(|parameter| parameter.name == "rc"));
}

#[test]
fn detection_separates_present_encoders_from_verified_hardware() {
    use luma_core::capability::Backend;
    use luma_core::providers::{Availability, detect};

    let encoders = vec![
        "hevc_qsv".to_owned(),
        "libx265".to_owned(),
        "hevc_mf".to_owned(),
    ];
    let statuses = detect(&encoders);

    let qsv = statuses
        .iter()
        .find(|status| status.backend == Backend::Qsv)
        .expect("qsv");
    assert_eq!(qsv.availability, Availability::Unverified);
    assert_eq!(qsv.encoders, vec!["hevc_qsv".to_owned()]);

    let software = statuses
        .iter()
        .find(|status| status.backend == Backend::Software)
        .expect("software");
    assert_eq!(software.availability, Availability::Available);

    let nvenc = statuses
        .iter()
        .find(|status| status.backend == Backend::Nvenc)
        .expect("nvenc");
    assert_eq!(nvenc.availability, Availability::Unavailable);
}

#[test]
fn an_empty_encoder_list_leaves_every_backend_unavailable() {
    use luma_core::providers::{Availability, detect};

    assert!(
        detect(&[])
            .iter()
            .all(|status| status.availability == Availability::Unavailable)
    );
}

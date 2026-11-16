use luma_core::capability::Backend;
use luma_core::providers::{requirement, REQUIREMENTS};

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
        assert!(!requirement.note.trim().is_empty(), "{:?}", requirement.backend);
    }
}

#[test]
fn software_is_detected_through_the_encoder_list() {
    let software = requirement(Backend::Software).expect("software requirement");
    assert_eq!(software.probe, luma_core::providers::Probe::EncoderList);
}

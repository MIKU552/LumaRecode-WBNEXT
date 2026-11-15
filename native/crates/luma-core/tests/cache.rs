use luma_core::cache::{
    BuildIdentity, Decision, Entry, Policy, kept_bytes, plan, reclaimable_bytes,
};

fn entry(key: &str, age_days: u32, bytes: u64, in_use: bool) -> Entry {
    Entry {
        key: key.to_owned(),
        age_days,
        bytes,
        in_use,
    }
}

fn decisions(plan: &[(String, Decision)]) -> Vec<Decision> {
    plan.iter().map(|(_, decision)| *decision).collect()
}

#[test]
fn key_is_short_lowercase_and_ignores_the_display_version() {
    let identity = BuildIdentity {
        source_commit: "0E9B340A1B2C3D4E5F60718293A4B5C6D7E8F901".to_owned(),
        toolchain: "Rust 1.100.0 / .NET 11.0.100".to_owned(),
        payload_digest: "AABBCCDDEEFF00112233445566778899".to_owned(),
    };

    let key = identity.key();
    assert!(key.starts_with("0e9b340a1b2c-"), "{key}");
    assert!(key.contains("rust"), "{key}");
    assert!(key.ends_with("-aabbccdd"), "{key}");
    assert!(
        key.chars().all(|character| character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '-'),
        "{key}"
    );
    assert!(identity.validate().is_ok());
}

#[test]
fn identity_rejects_missing_parts() {
    let empty = BuildIdentity {
        source_commit: " ".to_owned(),
        toolchain: "rust".to_owned(),
        payload_digest: "abcd".to_owned(),
    };

    assert_eq!(empty.validate().unwrap_err().code(), "cache.identity");
    assert_eq!(empty.validate().unwrap_err().detail(), Some("commit"));
}

#[test]
fn current_build_survives_age_count_and_budget() {
    let entries = [
        entry("current", 400, 900_000, false),
        entry("old", 60, 100_000, false),
        entry("recent", 2, 100_000, false),
    ];
    let policy = Policy {
        keep_recent: 1,
        max_age_days: 30,
        max_bytes: 0,
    };

    let plan = plan(&entries, "current", &policy);
    assert_eq!(decisions(&plan)[0], Decision::KeepCurrent);
    assert_eq!(decisions(&plan)[1], Decision::RemoveTooOld);
    assert_eq!(decisions(&plan)[2], Decision::KeepRecent);
}

#[test]
fn in_use_build_is_never_removed() {
    let entries = [
        entry("current", 0, 10, false),
        entry("running", 90, 10, true),
        entry("stale", 90, 10, false),
    ];

    let plan = plan(
        &entries,
        "current",
        &Policy {
            keep_recent: 0,
            max_age_days: 30,
            max_bytes: 1,
        },
    );
    assert_eq!(
        decisions(&plan),
        vec![
            Decision::KeepCurrent,
            Decision::KeepInUse,
            Decision::RemoveTooOld
        ]
    );
}

#[test]
fn only_the_newest_reclaimable_builds_survive_the_count_pass() {
    let entries = [
        entry("current", 0, 1, false),
        entry("a", 10, 1, false),
        entry("b", 5, 1, false),
        entry("c", 20, 1, false),
    ];
    let policy = Policy {
        keep_recent: 2,
        max_age_days: 0,
        max_bytes: 0,
    };

    let plan = plan(&entries, "current", &policy);
    assert_eq!(
        decisions(&plan),
        vec![
            Decision::KeepCurrent,
            Decision::KeepRecent,
            Decision::KeepRecent,
            Decision::RemoveOverCount,
        ]
    );
}

#[test]
fn budget_drops_the_oldest_reclaimable_build_first() {
    let entries = [
        entry("current", 0, 500, false),
        entry("a", 1, 400, false),
        entry("b", 9, 400, false),
    ];
    let policy = Policy {
        keep_recent: 5,
        max_age_days: 0,
        max_bytes: 1_000,
    };

    let plan = plan(&entries, "current", &policy);
    assert_eq!(
        decisions(&plan),
        vec![
            Decision::KeepCurrent,
            Decision::KeepRecent,
            Decision::RemoveOverBudget
        ]
    );
    assert_eq!(kept_bytes(&entries, &plan), 900);
    assert_eq!(reclaimable_bytes(&entries, &plan), 400);
}

#[test]
fn an_empty_cache_plans_nothing() {
    let entries: Vec<Entry> = Vec::new();
    assert!(plan(&entries, "current", &Policy::default()).is_empty());
}

#[test]
fn a_blank_policy_keeps_everything_recent() {
    let entries = [entry("a", 3, 10, false), entry("b", 4, 10, false)];
    let policy = Policy {
        keep_recent: 10,
        max_age_days: 0,
        max_bytes: 0,
    };

    let plan = plan(&entries, "current", &policy);
    assert!(
        decisions(&plan)
            .iter()
            .all(|decision| matches!(decision, Decision::KeepRecent))
    );
}

#[test]
fn owner_root_keeps_the_program_segment() {
    assert_eq!(
        luma_core::cache::owner_root("C:/Users/u/AppData/Local/"),
        "C:/Users/u/AppData/Local/luma-recode"
    );
    assert_eq!(
        luma_core::cache::owner_root("/Users/u/Library/Caches"),
        "/Users/u/Library/Caches/luma-recode"
    );
}

#[test]
fn entry_path_appends_the_build_key() {
    assert_eq!(
        luma_core::cache::entry_path("C:/cache/luma-recode", "abc-123"),
        "C:/cache/luma-recode/abc-123"
    );
}

#[test]
fn containment_refuses_escapes_and_siblings() {
    assert!(luma_core::cache::within(
        "C:/cache/luma-recode",
        "C:/cache/luma-recode/a"
    ));
    assert!(luma_core::cache::within(
        "C:/cache/luma-recode",
        "C:/cache/luma-recode"
    ));
    assert!(!luma_core::cache::within(
        "C:/cache/luma-recode",
        "C:/cache/luma-recode/../other"
    ));
    assert!(!luma_core::cache::within(
        "C:/cache/luma-recode",
        "C:/cache/luma-recode2/a"
    ));
    assert!(!luma_core::cache::within("", "C:/cache/a"));
}

#[test]
fn scope_rejects_system_roots_and_shared_extraction_directories() {
    assert!(!luma_core::cache::scope_allowed(""));
    assert!(!luma_core::cache::scope_allowed("/"));
    assert!(!luma_core::cache::scope_allowed("C:/"));
    assert!(!luma_core::cache::scope_allowed("C:/Windows/Temp"));
    assert!(!luma_core::cache::scope_allowed("/var/tmp"));
    assert!(!luma_core::cache::scope_allowed(
        "C:/Users/u/AppData/Local/Temp/.net"
    ));
}

#[test]
fn scope_accepts_the_programs_own_cache() {
    assert!(luma_core::cache::scope_allowed(
        "C:/Users/u/AppData/Local/luma-recode"
    ));
    assert!(luma_core::cache::scope_allowed(
        "/Users/u/Library/Caches/luma-recode"
    ));
}

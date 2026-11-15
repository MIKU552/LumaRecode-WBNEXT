use luma_core::safety::{check, Action, PathFacts};

fn facts(path: &str, exists: bool) -> PathFacts {
    PathFacts {
        path: path.to_owned(),
        exists,
        symlink: false,
        case_insensitive: false,
    }
}

#[test]
fn a_new_path_is_created() {
    let action = check(&facts("out.mkv", false), &[facts("in.mov", true)], false).unwrap();
    assert_eq!(action, Action::Create);
    assert!(!action.replace());
}

#[test]
fn an_existing_path_needs_confirmation() {
    let error = check(&facts("out.mkv", true), &[facts("in.mov", true)], false).unwrap_err();
    assert_eq!(error.code(), "output.exists");

    let action = check(&facts("out.mkv", true), &[facts("in.mov", true)], true).unwrap();
    assert_eq!(action, Action::Replace);
    assert!(action.replace());
}

#[test]
fn writing_over_an_input_is_rejected_even_with_confirmation() {
    let error = check(&facts("clip.mp4", true), &[facts("./clip.mp4", true)], true).unwrap_err();
    assert_eq!(error.code(), "output.same_as_input");
    assert_eq!(error.detail(), Some("./clip.mp4"));
}

#[test]
fn separators_and_dot_segments_do_not_hide_the_same_file() {
    let output = PathFacts::new("media\\clip.mp4");
    let input = PathFacts::new("media/./clip.mp4");
    assert_eq!(output.normalized(), input.normalized());
}

#[test]
fn parent_segments_are_resolved() {
    assert_eq!(PathFacts::new("a/b/../c.mkv").normalized(), "a/c.mkv");
    assert_eq!(PathFacts::new("/a/./b//c.mkv").normalized(), "/a/b/c.mkv");
}

#[test]
fn case_insensitive_volumes_compare_folded_names() {
    let output = PathFacts {
        path: "OUT.MKV".to_owned(),
        exists: false,
        symlink: false,
        case_insensitive: true,
    };
    let input = PathFacts {
        path: "out.mkv".to_owned(),
        exists: true,
        symlink: false,
        case_insensitive: true,
    };

    assert_eq!(
        check(&output, &[input], true).unwrap_err().code(),
        "output.same_as_input"
    );
}

#[test]
fn a_symbolic_link_destination_is_refused() {
    let output = PathFacts {
        path: "out.mkv".to_owned(),
        exists: true,
        symlink: true,
        case_insensitive: false,
    };

    assert_eq!(check(&output, &[], true).unwrap_err().code(), "output.symlink");
}

#[test]
fn other_inputs_may_share_a_name() {
    let action = check(
        &facts("out.mkv", false),
        &[facts("a/clip.mov", true), facts("b/clip.mov", true)],
        false,
    )
    .unwrap();

    assert_eq!(action, Action::Create);
}

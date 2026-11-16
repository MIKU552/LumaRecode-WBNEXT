use luma_core::process::{quote, Command};

#[test]
fn plain_paths_render_without_quotes() {
    let command = Command::new("ffmpeg").arg("-i").arg("clip.mp4").arg("-y");
    assert_eq!(command.render(), "ffmpeg -i clip.mp4 -y");
}

#[test]
fn arguments_with_spaces_are_quoted() {
    let command = Command::new("/opt/my tools/ffmpeg").arg("-i").arg("my clip.mov");
    assert_eq!(
        command.render(),
        "\"/opt/my tools/ffmpeg\" -i \"my clip.mov\""
    );
}

#[test]
fn embedded_quotes_are_escaped_and_empty_arguments_kept() {
    assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    assert_eq!(quote(""), "\"\"");
    assert_eq!(quote("filter=a=b"), "filter=a=b");
}

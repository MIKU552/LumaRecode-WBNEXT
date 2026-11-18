#!/usr/bin/env bash
# Builds the shared core, the Swift package, the .app bundle and a DMG.
#
# Everything the release needs happens here, so Xcode and the shell cannot
# disagree about where the Rust library comes from. Run with --stage-only to
# prepare Libraries/ before building inside Xcode.
set -euo pipefail

configuration="release"
make_dmg=1
stage_only=0
run_tests=0
identity="-"
with_ffmpeg=0

usage() {
    cat <<'EOF'
usage: build-macos.sh [options]

  --configuration <debug|release>  Swift build configuration (default: release)
  --skip-dmg                       Build the .app without packaging a DMG
  --stage-only                     Only build and stage the Rust library
  --test                           Run the Swift package tests after staging
  --identity <identity>            codesign identity (default: ad-hoc "-")
  --with-ffmpeg                    Embed the full FFmpeg (keg-only; dynamic, so it only
                                   runs where its libraries are)
  -h, --help                       Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --configuration) configuration="${2:?missing value}"; shift 2 ;;
        --skip-dmg) make_dmg=0; shift ;;
        --stage-only) stage_only=1; shift ;;
        --test) run_tests=1; shift ;;
        --identity) identity="${2:?missing value}"; shift 2 ;;
        --with-ffmpeg) with_ffmpeg=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

case "$configuration" in
    debug|release) ;;
    *) echo "--configuration must be debug or release" >&2; exit 2 ;;
esac

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
app_dir="$repo/app-macos"
architecture="$(uname -m)"
if [[ "$architecture" != "arm64" ]]; then
    echo "The first release targets Apple Silicon; refusing to build on $architecture." >&2
    exit 1
fi

echo "==> Building shared core"
cargo build --manifest-path "$repo/native/Cargo.toml" -p luma-ffi --release

mkdir -p "$app_dir/Libraries"
cp "$repo/native/target/release/liblumacore.a" "$app_dir/Libraries/liblumacore.a"

if [[ $stage_only -eq 1 ]]; then
    echo "==> Staged $app_dir/Libraries/liblumacore.a"
    exit 0
fi

# SwiftPM's link step does not read LIBRARY_PATH reliably, so pass the staged
# directory to the linker explicitly as well.
export LIBRARY_PATH="$app_dir/Libraries"
linker=(-Xlinker "-L$app_dir/Libraries")

if [[ $run_tests -eq 1 ]]; then
    echo "==> Testing LumaKit"
    swift test --package-path "$app_dir" "${linker[@]}"
fi

echo "==> Building LumaRecode ($configuration)"
swift build --package-path "$app_dir" -c "$configuration" "${linker[@]}"
bin_path="$(swift build --package-path "$app_dir" -c "$configuration" --show-bin-path)"

echo "==> Assembling app bundle"
app="$app_dir/dist/LumaRecode.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$app_dir/Resources/Info.plist" "$app/Contents/Info.plist"
cp "$bin_path/LumaRecode" "$app/Contents/MacOS/LumaRecode"
chmod +x "$app/Contents/MacOS/LumaRecode"
for bundle in "$bin_path"/*.bundle; do
    [[ -e "$bundle" ]] || continue
    cp -R "$bundle" "$app/Contents/Resources/"
done

if [[ $with_ffmpeg -eq 1 ]]; then
    echo "==> Embedding FFmpeg"
    full="$(brew --prefix ffmpeg-full 2>/dev/null || true)"
    if [[ -z "$full" || ! -x "$full/bin/ffmpeg" ]]; then
        echo "ffmpeg-full is not installed; brew install ffmpeg-full" >&2
        exit 1
    fi
    mkdir -p "$app/Contents/Resources/bin" "$app/Contents/Resources/licenses"
    # Homebrew's bin entries are symlinks into the Cellar; copy what they point at.
    cp -L "$full/bin/ffmpeg" "$full/bin/ffprobe" "$app/Contents/Resources/bin/"
    for binary in ffmpeg ffprobe; do
        size="$(stat -f%z "$app/Contents/Resources/bin/$binary")"
        if [[ $size -lt 1000000 ]]; then
            echo "embedded $binary is only $size bytes; the copy took a link, not the file" >&2
            exit 1
        fi
    done
    # The bundle ships GPL binaries, so their licence travels with them.
    for licence in "$full"/COPYING* "$full"/LICENSE*; do
        [[ -f "$licence" ]] && cp "$licence" "$app/Contents/Resources/licenses/"
    done
fi

echo "==> Signing with $identity"
codesign --force --sign "$identity" --timestamp=none "$app"
codesign --verify --strict --verbose=2 "$app"

version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")"

if [[ $make_dmg -eq 1 ]]; then
    echo "==> Packaging DMG"
    suffix=""
    [[ $with_ffmpeg -eq 1 ]] && suffix="-full"
    dmg="$app_dir/dist/LumaRecode-$version-arm64$suffix.dmg"
    rm -f "$dmg"
    hdiutil create -volname "LumaRecode" -srcfolder "$app" -ov -format UDZO "$dmg" >/dev/null
    echo "DMG: $dmg"
fi

echo "App: $app"
if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$app/Contents/MacOS/LumaRecode"
fi

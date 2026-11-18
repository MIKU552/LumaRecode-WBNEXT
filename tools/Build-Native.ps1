<#
.SYNOPSIS
Builds the Rust shared core and stages the native library per architecture.
.DESCRIPTION
Produces lumacore.dll for x64 or ARM64 under artifacts/native/<arch>. The
managed projects pick that directory up through LumaNativeLibraryDirectory, so
Visual Studio and build.ps1 never need a manual copy step.

Native selects the host architecture. Arm64 cross-compiles and requires the
Visual Studio ARM64 C++ toolchain. Tests only run when the target matches the
host, because a cross-compiled library cannot execute locally.
#>
param(
    [ValidateSet('Debug','Release')][string]$Configuration = 'Release',
    [ValidateSet('Native','X64','Arm64')][string]$Target = 'Native',
    [switch]$Test,
    [switch]$NoStage
)
$ErrorActionPreference = 'Stop'
$taskRepo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location -LiteralPath $taskRepo
try {
    $taskRoot = $taskRepo.TrimEnd([IO.Path]::DirectorySeparatorChar)
    $taskHostArch = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $taskArch = switch ($Target) {
        'X64' { 'x64' }
        'Arm64' { 'arm64' }
        default { if ($taskHostArch -eq 'Arm64') { 'arm64' } else { 'x64' } }
    }
    $taskTriple = if ($taskArch -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
    $taskRid = if ($taskArch -eq 'arm64') { 'win-arm64' } else { 'win-x64' }
    $taskHostTriple = if ($taskHostArch -eq 'Arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'cargo was not found. Install the Rust toolchain recorded in native/rust-toolchain.toml.'
    }
    $taskProfile = if ($Configuration -eq 'Debug') { 'debug' } else { 'release' }
    $taskCargoArgs = @('build','--manifest-path','native/Cargo.toml','-p','luma-ffi','--target',$taskTriple)
    if ($Configuration -eq 'Release') { $taskCargoArgs += '--release' }
    $taskRust = (& cargo --version) -join ''
    $taskRustc = (& rustc --version) -join ''
    Write-Output "Core: $taskRust / $taskRustc"
    Write-Output "Target: $Target -> $taskTriple ($taskRid), $Configuration"
    $taskInstalled = (& rustup target list --installed) -contains $taskTriple
    if (-not $taskInstalled) { throw "Rust target $taskTriple is not installed. Run: rustup target add $taskTriple" }
    if ($Test) {
        if ($taskTriple -ne $taskHostTriple) {
            Write-Warning "Skipping tests: $taskTriple cannot run on this host."
        }
        else {
            & cargo test --manifest-path native/Cargo.toml --workspace
            if ($LASTEXITCODE -ne 0) { throw 'Core tests failed.' }
        }
    }
    & cargo @taskCargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed.' }
    $taskTarget = Join-Path $taskRoot "native/target/$taskTriple/$taskProfile"
    $taskDll = Join-Path $taskTarget 'lumacore.dll'
    if (-not (Test-Path -LiteralPath $taskDll)) { throw "Missing build output: $taskDll" }
    if ($NoStage) {
        Write-Output "Built (unstaged): $taskDll"
        return
    }
    $taskStage = Join-Path $taskRoot "artifacts/native/$taskArch"
    New-Item -ItemType Directory -Path $taskStage -Force | Out-Null
    foreach ($taskName in @('lumacore.dll','lumacore.pdb','lumacore.lib')) {
        $taskFile = Join-Path $taskTarget $taskName
        if (Test-Path -LiteralPath $taskFile) {
            Copy-Item -LiteralPath $taskFile -Destination (Join-Path $taskStage $taskName) -Force
        }
    }
    Copy-Item -LiteralPath (Join-Path $taskRoot 'native/include/luma_core.h') -Destination (Join-Path $taskStage 'luma_core.h') -Force
    $taskHash = (Get-FileHash -LiteralPath (Join-Path $taskStage 'lumacore.dll') -Algorithm SHA256).Hash
    Write-Output "Staged: $taskStage/lumacore.dll"
    Write-Output "SHA256: $taskHash"
} finally { Pop-Location }

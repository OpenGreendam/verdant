$ErrorActionPreference = 'Stop'
$root = (New-Item -ItemType Directory -Path "$env:GITHUB_WORKSPACE/evidence").FullName
$source = (Resolve-Path "$env:GITHUB_WORKSPACE/source").Path
$ExpectedCommit = $env:RELEASE_COMMIT
$identity = @{version='0.1.0'; tree=$env:RELEASE_TREE}
$pins = Get-Content -Raw "$PSScriptRoot/approved.json" | ConvertFrom-Json -AsHashtable
$pin = $pins.policy
$selection = $null
$env:SOURCE_DATE_EPOCH = $env:RELEASE_EPOCH
$env:CARGO_TARGET_DIR = "$env:RUNNER_TEMP/target"
function Hash([string]$Path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Run($Program, [string[]]$Arguments) {
    & $Program @Arguments 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "$Program exited $LASTEXITCODE" }
}

function Capture($Program, [string[]]$Arguments) {
    $stdout = & $Program @Arguments 2> (Join-Path $root 'native.stderr')
    $code = $LASTEXITCODE
    Get-Content -LiteralPath (Join-Path $root 'native.stderr') | Out-Host
    if ($code -ne 0) { throw "$Program exited $code" }
    return ($stdout -join "`n")
}

function Json-Receipt([string]$Name, $Value) {
    $Value | ConvertTo-Json -Depth 20 | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $root $Name)
}


if ((Capture git @('-C',$source,'rev-parse','HEAD')) -cne $ExpectedCommit -or
    (Capture git @('-C',$source,'rev-parse','HEAD^{tree}')) -cne $identity.tree) { throw 'Wrong checkout' }
if ($ExpectedCommit -cnotmatch '^[a-f0-9]{40}$' -or $identity.tree -cnotmatch '^[a-f0-9]{40}$' -or
    (Capture git @('-C',$source,'show','-s','--format=%ct','HEAD')) -cne $env:RELEASE_EPOCH -or
    (Capture git @('-C',$source,'status','--porcelain'))) { throw 'Unexpected product source or timestamp' }
$previous = "$env:RUNNER_TEMP/rc6/verdant-0.1.0-windows-x86_64-with-voice.zip"
if ((Hash $previous) -cne $pins.rc6_sha256) { throw 'Preceding archive changed' }
$previousRoot = "$env:RUNNER_TEMP/previous"
Expand-Archive -LiteralPath $previous -DestinationPath $previousRoot
$share = "$previousRoot/verdant-0.1.0-windows-x86_64/share/verdant"
$voice = "$share/bundled-voice"
$policy = "$share/BUNDLE-POLICY.json"
$files = @(Get-ChildItem $voice -File -Recurse)
if ($files.Count -ne $pins.voice_members.Count -or (Hash $policy) -cne $pin) { throw 'Incorrect release inputs' }
foreach ($file in $files) {
    $path = [IO.Path]::GetRelativePath($voice,$file.FullName)
    if ((Hash $file.FullName) -cne $pins.voice_members[$path]) { throw "Unexpected voice member $path" }
}
$env:VERDANT_BUNDLED_VOICE_POLICY_SHA256 = $pin
$env:VERDANT_VOICE_BANK_SHA256 = $pins.voice_members['bank.safetensors']
Set-Location "$source/frontend"
Run cargo @('fetch','--locked')
Run cargo @('build','--offline','--locked','--release','-p','verdant-worker','-p','verdant-test-worker','--features','verdant-test-worker/protocol-fixture')
$env:VERDANT_WORKER_BINARY = (Resolve-Path "$env:CARGO_TARGET_DIR/release/verdant-worker.exe").Path
$env:VERDANT_TEST_WORKER_BINARY = (Resolve-Path "$env:CARGO_TARGET_DIR/release/verdant-test-worker.exe").Path
$backendTests = & cargo test --offline --locked --release -p verdant-backend --test backend --test f0_artifacts 2>&1
$backendExit = $LASTEXITCODE
$backendTests | Out-Host
$backendTests | Set-Content "$root/windows-backend-tests.log"
if ($backendExit -ne 0) { throw 'Native backend or F0 regressions failed' }
foreach ($case in @('durable_cache_roundtrips_and_reports_corruption','real_rust_worker_handshake_without_models','full_loop_render_verify_bind_with_test_double','b6_voice_artifacts_remain_playable_after_verified_cache_roundtrip')) {
    if (($backendTests -join "`n") -notmatch ("test " + $case + ' \.\.\. ok')) { throw "Native test was not executed: $case" }
}
Run cargo @('build','--offline','--locked','--release','-p','verdant-app','-p','verdant-cli','-p','verdant-launcher','-p','verdant-worker','-p','verdant-plughost','-p','verdant-voice','-p','verdant-release')
$tests = & cargo test --offline --locked --release -p verdant-cli windows_path_tests 2>&1
$testExit = $LASTEXITCODE
$tests | Set-Content "$root/windows-path-tests.log"
if ($testExit -ne 0 -or ($tests -join "`n") -notmatch 'publishes_a_new_job_with_its_execution_lease_held \.\.\. ok') { throw 'Native Windows tests failed' }
$binaries = "$env:RUNNER_TEMP/binaries"
New-Item -ItemType Directory $binaries | Out-Null
$binaryHashes = @{}
foreach ($program in @('verdant','verdant-cli','verdant-gui','verdant-launcher','verdant-worker','verdant-plughost','verdant-voice','verdant-release')) {
    Copy-Item "$env:CARGO_TARGET_DIR/release/$program.exe" $binaries
    $binaryHashes[$program] = Hash "$binaries/$program.exe"
}
$compiler = Capture rustc @('-vV')
if ($compiler -notmatch '(?m)^host: x86_64-pc-windows-msvc\s*$') { throw 'Expected native x64 Windows MSVC' }
Json-Receipt 'windows-build.json' @{commit=$ExpectedCommit; target='windows-x86_64'; binaries=$binaryHashes; windows_path_tests=$testExit; backend_tests=$backendExit; release=true; worker_sha256=(Hash $env:VERDANT_WORKER_BINARY); fixture_worker_sha256=(Hash $env:VERDANT_TEST_WORKER_BINARY); cargo_lock_sha256=(Hash 'Cargo.lock'); actions_run_id=$env:GITHUB_RUN_ID; actions_job=$env:GITHUB_JOB; runner=$env:RUNNER_NAME; image_version=$env:ImageVersion; rustc=$compiler}
$release = "$binaries/verdant-release.exe"
$wheel = "$env:RUNNER_TEMP/onnxruntime.whl"
$runtime = (Capture $release @('runtime-inputs','--target','windows-x86_64')) | ConvertFrom-Json
Run curl.exe @('--fail','--location','--output',$wheel,$runtime.wheel_url)
if ((Hash $wheel) -cne $pins.wheel) { throw 'Incorrect ONNX Runtime wheel' }
Json-Receipt 'windows-inputs.json' $pins
$selectionFile = "$root/windows-source-selection.json"
$selected = (Capture $release @('select-source','--repo',$source,'--output',$selectionFile)) | ConvertFrom-Json
if ($selected.source_commit -cne $ExpectedCommit -or (Hash $selectionFile) -cne $selected.sha256) { throw 'Source selection differs' }
$selection = $selected.sha256
$output = "$root/dist"
$binding = @('--artifact-policy', $policy, '--artifact-policy-sha256', $pin)
$packaged = Capture $release (@('package', '--repo', $source, '--binaries', $binaries, '--target', 'windows-x86_64',
    '--onnxruntime-wheel', $wheel, '--output', $output,
    '--source-selection', $selectionFile, '--source-selection-sha256', $selection, '--voice-bundle', $voice) + $binding)
$packaged | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $root 'windows-package.json')
$guard = Capture $release (@('check', '--check', $output, '--require-package', '--require-voice',
    '--expected-source-commit', $ExpectedCommit, '--expected-source-selection-sha256', $selection) + $binding)
$guard | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $root 'windows-archive-guard.json')

$name = "verdant-$($identity.version)-windows-x86_64"
$archive = Join-Path $output "$name.zip"
$checksum = Join-Path $output "$name.sha256"
$archiveSha = Hash $archive
if ((Get-Content -Raw -LiteralPath $checksum).Trim() -cne "$archiveSha  $name.zip" -or
    ($packaged | ConvertFrom-Json).sha256 -cne $archiveSha) { throw 'Package archive checksum differs' }


Json-Receipt 'windows-result.json' @{status='archive-verified'; run_id=$env:GITHUB_RUN_ID; helper_sha=$env:GITHUB_SHA;
    commit=$ExpectedCommit; tree=$identity.tree; policy_sha256=$pin; source_selection_sha256=$selection;
    archive=('dist/' + (Split-Path -Leaf $archive)); archive_sha256=$archiveSha; checksum=('dist/' + (Split-Path -Leaf $checksum))}

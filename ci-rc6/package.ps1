$ErrorActionPreference = 'Stop'
$root = (New-Item -ItemType Directory -Path "$env:GITHUB_WORKSPACE/evidence").FullName
$source = (Resolve-Path "$env:GITHUB_WORKSPACE/source").Path
$ExpectedCommit = $env:RELEASE_COMMIT
$runId = $env:RELEASE_RUN_ID
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
$previous = "$env:RUNNER_TEMP/rc5/verdant-0.1.0-windows-x86_64-with-voice.zip"
if ((Hash $previous) -cne $pins.rc5_sha256) { throw 'Preceding archive changed' }
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
$backendTests = & cargo test --offline --locked --release -p verdant-backend --test backend --test f0_artifacts 2>&1
$backendExit = $LASTEXITCODE
$backendTests | Out-Host
$backendTests | Set-Content "$root/windows-backend-tests.log"
if ($backendExit -ne 0 -or ($backendTests -join "`n") -notmatch 'durable_cache_roundtrips_and_reports_corruption \.\.\. ok') { throw 'Native durable cache regression failed' }
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
$z3rm = '28f0728802599ad9ac902b86e322f783081c7df3'
if (!(Select-String -SimpleMatch "#$z3rm" -Path Cargo.lock -Quiet)) { throw 'Z3RM pin differs' }
Json-Receipt 'windows-build.json' @{commit=$ExpectedCommit; target='windows-x86_64'; binaries=$binaryHashes; windows_path_tests=$testExit; z3rm_commit=$z3rm; actions_run_id=$env:GITHUB_RUN_ID; actions_job=$env:GITHUB_JOB; runner=$env:RUNNER_NAME; image_version=$env:ImageVersion; rustc=(Capture rustc @('-vV'))}
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


$expanded = "$env:RUNNER_TEMP/expanded"
Expand-Archive -LiteralPath $archive -DestinationPath $expanded
$cli = Join-Path $expanded "$name\bin\verdant-cli.exe"
$smoke = Join-Path $env:RUNNER_TEMP 'smoke'
New-Item -ItemType Directory -Path $smoke | Out-Null
Get-ChildItem Env:VERDANT_* | ForEach-Object {
    Remove-Item -LiteralPath "Env:$($_.Name)"
    if (Test-Path -LiteralPath "Env:$($_.Name)") { throw "Inherited override remains: $($_.Name)" }
}
$env:VERDANT_STATE_DIR = Join-Path $smoke 'state'
$env:VERDANT_USER_DATA_DIR = Join-Path $smoke 'data'
$env:VERDANT_CACHE_DIR = Join-Path $smoke 'cache'
New-Item -ItemType Directory -Path $env:VERDANT_STATE_DIR,$env:VERDANT_USER_DATA_DIR,$env:VERDANT_CACHE_DIR | Out-Null
$registry = Join-Path $env:VERDANT_STATE_DIR 'voices.json'
$project = Join-Path $smoke 'singing.verp'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'windows-singing.verp') -Destination $project
$fixtureSha = Hash $project
if ($fixtureSha -cne $pins.fixture) { throw 'Smoke project copy differs from media' }
$wav = Join-Path $root 'windows-smoke.wav'
Set-Location -LiteralPath $smoke
$doctor = Capture $cli @('doctor', '--json')
$doctor | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $root 'windows-doctor.json')
$report = $doctor | ConvertFrom-Json
$caps = $report.result.capabilities
$voiceId = "verdant-voice-$($identity.version)"
if (!$report.ok -or $caps.voices.default_voice -cne $voiceId -or !$caps.voices.voices.$voiceId.available -or
    $caps.voice_package.bundled_voice_admission.policy_sha256 -cne $pin) { throw 'Packaged doctor did not admit the bundled default' }
$render = Capture $cli @('render', 'singing.verp', '--output', $wav, '--no-effects', '--json')
$render | Set-Content -Encoding utf8NoBOM -LiteralPath (Join-Path $root 'windows-render.json')
$report = $render | ConvertFrom-Json
if (!$report.ok -or $report.result.frames -ne 12000 -or $report.result.sample_rate -ne 48000 -or
    !$report.result.phrases -or @($report.result.phrases | Where-Object route -CNE 'verdant-voice').Count -ne 0 -or
    (Test-Path -LiteralPath $registry) -or (Hash $wav) -cne '052824eae685bdb9107b80df7838b0dd907e099220cb0db371b3306f677e702b') {
    throw 'Real first-run bundled synthesis differs from the verified Windows baseline'
}
$receipts = @{}
foreach ($name in @('windows-package.json', 'windows-archive-guard.json', 'windows-doctor.json',
                    'windows-render.json', 'windows-smoke.wav', 'windows-source-selection.json', 'windows-build.json')) {
    $receipts[$name] = Hash (Join-Path $root $name)
}
Json-Receipt 'windows-result.json' @{status='verified'; run_id=$runId; commit=$ExpectedCommit; tree=$identity.tree;
    policy_sha256=$pin; source_selection_sha256=$selection; archive=("dist/" + (Split-Path -Leaf $archive));
    archive_sha256=$archiveSha; checksum=("dist/" + (Split-Path -Leaf $checksum)); fresh_state=$true; empty_registry=$true;
    fixture_sha256=$fixtureSha; receipts=$receipts}

Set-Content -Encoding ascii -LiteralPath (Join-Path $root 'windows-exit.txt') -Value '0'

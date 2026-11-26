$ErrorActionPreference = 'Stop'
$root = (New-Item -ItemType Directory offline-evidence).FullName
function Save($name,$value) { ConvertTo-Json -InputObject $value -Depth 15 | Set-Content -Encoding utf8NoBOM "$root/$name.json" }
function Hash($path) { (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() }
$zip = (Get-Item package/dist/verdant-0.1.0-windows-x86_64.zip).FullName
if ((Hash $zip) -ne 'ca3f6da947a0d85d8b0be4726d86a7e92968157723ba582725711ec004e7d668') { throw 'Incorrect package' }
Expand-Archive $zip extracted
$base=(Resolve-Path extracted/verdant-0.1.0-windows-x86_64).Path
$manifest=Get-Content -Raw "$base/RELEASE-ASSETS.json" | ConvertFrom-Json
$programs=@(Get-ChildItem $base -Recurse -File -Filter '*.exe')
if ($manifest.source_offer.source_commit -cne '4fdcd984e08e845ac24a36f6d0763c805e4202b1' -or $manifest.programs.Count -ne 8) { throw 'Incorrect manifest' }
foreach ($p in $manifest.programs) { if ((Hash "$base/$($p.path)") -cne $p.sha256 -or (Get-Item "$base/$($p.path)").FullName -notin $programs.FullName) { throw 'Program identity mismatch' } }
$control=(Get-Item "$env:SystemRoot/System32/curl.exe").FullName
function Connect-Control {
    $result = & $control --disable --noproxy '*' --ipv4 --silent --show-error --output NUL --write-out '%{http_code}|%{remote_ip}' --connect-timeout 5 --max-time 10 https://api.github.com/ 2>&1
    $code=$LASTEXITCODE
    return @{exit=$code; output=($result -join "`n")}
}
$before=Connect-Control
Save 'control-before' $before
if ($before.exit -ne 0 -or $before.output -notmatch '[234]\d\d\|[0-9]+\.') { throw 'Positive network control failed' }
$group="Verdant offline $env:GITHUB_RUN_ID"
Set-NetFirewallProfile -Profile Domain,Private,Public -Enabled True
$profiles=@(Get-NetFirewallProfile -PolicyStore ActiveStore | Select-Object Name,Enabled)
Save 'firewall-profiles' $profiles
if ($profiles.Count -ne 3 -or @($profiles | Where-Object { "$($_.Enabled)" -ne 'True' }).Count) { throw 'Inactive firewall profile' }
$paths=@($programs.FullName)+@($control)
$rules=@()
foreach ($path in $paths) {
    $rule=New-NetFirewallRule -DisplayName "$group $path" -Group $group -Program $path -Enabled True -Direction Outbound -Action Block -Profile Any -Protocol Any
    $active=Get-NetFirewallRule -PolicyStore ActiveStore -Name $rule.Name
    $filter=$active | Get-NetFirewallApplicationFilter
    if ($active.Action -ne 'Block' -or $active.Direction -ne 'Outbound' -or "$($active.Enabled)" -ne 'True' -or $active.Profile -ne 'Any' -or [Environment]::ExpandEnvironmentVariables($filter.Program) -ine $path) { throw "Ineffective firewall rule $path" }
    $rules+=@{name=$rule.Name; program=$path; sha256=(Hash $path); direction=[string]$active.Direction; action=[string]$active.Action; enabled=[string]$active.Enabled; profile=[string]$active.Profile}
}
Save 'active-outbound-rules' $rules
Start-Sleep -Seconds 2
$blocked=Connect-Control
Save 'control-blocked' $blocked
if ($blocked.exit -eq 0 -or $blocked.output -notmatch '000\|') { throw 'Negative network control failed' }
$identity=@{version='0.1.0'}
$pin='0e94c67260a22a56ff2f38d41abadb60767ea8dccc81ad7737319b5b54bc9a15'
$pins=@{fixture='5bcd4b9df3223faaaa570c7410a30ce2caa72b637a82d05fb80862ab0f165c4c'}
$cli="$base/bin/verdant-cli.exe"
function Capture($Program, [string[]]$Arguments) {
    $stdout = & $Program @Arguments 2> (Join-Path $root 'native.stderr')
    $code = $LASTEXITCODE
    Get-Content -LiteralPath (Join-Path $root 'native.stderr') | Out-Host
    if ($code -ne 0) { throw "$Program exited $code" }
    return ($stdout -join "`n")
}

$smoke = Join-Path $env:RUNNER_TEMP 'offline-first-use'
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

$controlRule=$rules | Where-Object program -EQ $control
Remove-NetFirewallRule -Name $controlRule.name
$restored=Connect-Control
Save 'control-restored' $restored
if ($restored.exit -ne 0) { throw 'Restored network control failed' }
Save 'result' @{status='verified'; run_id=$env:GITHUB_RUN_ID; archive_sha256=(Hash $zip); fixture_sha256=$fixtureSha; wav_sha256=(Hash $wav); frames=$report.result.frames; sample_rate=$report.result.sample_rate; registry_absent=!(Test-Path $registry); default_voice=$voiceId; blocked_programs=$programs.Count; source_commit=$manifest.source_offer.source_commit}

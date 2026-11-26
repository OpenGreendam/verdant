cargo build --manifest-path "$repo/frontend/Cargo.toml" --offline --locked --release \
    -p verdant-app -p verdant-cli -p verdant-launcher -p verdant-worker -p verdant-plughost -p verdant-voice -p verdant-release
mkdir bin
while IFS= read -r binary; do
    strip --strip-debug -o "bin/$binary" "$CARGO_TARGET_DIR/release/$binary"
done < <(jq -er '.programs[]' run.json)

python3 - "$commit" <<'PY' > "$out/linux-build.json"
import hashlib, json, pathlib, subprocess, sys

def digest(path):
    with path.open('rb') as data:
        return hashlib.file_digest(data, 'sha256').hexdigest()

print(json.dumps(dict(commit=sys.argv[1], target='linux-x86_64',
                      actions_run_id=__import__('os').environ['GITHUB_RUN_ID'], rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                      binaries={p.name: digest(p) for p in pathlib.Path('bin').iterdir()})))
PY

release="$run/bin/verdant-release"
selection_file="$out/linux-source-selection.json"
selection=$("$release" select-source --repo "$repo" --output "$selection_file" | jq -er .sha256)
test "$(sha256sum "$selection_file" | cut -d ' ' -f 1)" = "$selection"
test "$selection" = 00706d5416959e83aa40f1c2875dca92a78bb321fe82ec8bebe1f7fbbcf50863
mkdir "$out/dist"
policy="$run/inputs/voice-policy.json"
"$release" package --repo "$repo" --binaries bin --target linux-x86_64 \
    --onnxruntime-dir inputs/ort --output "$out/dist" \
    --source-selection "$selection_file" --source-selection-sha256 "$selection" \
    --voice-bundle voice --artifact-policy "$policy" --artifact-policy-sha256 "$pin" > "$out/linux-package.json"
"$release" check --check "$out/dist" --require-package --require-voice \
    --expected-source-commit "$commit" --expected-source-selection-sha256 "$selection" \
    --artifact-policy "$policy" --artifact-policy-sha256 "$pin" > "$out/linux-archive-guard.json"

name="verdant-$version-linux-x86_64"
archive="dist/$name.tar.gz"
checksum="dist/$name.sha256"
(cd "$out/dist" && sha256sum -c -- "$name.sha256")
mkdir smoke
tar -C smoke -xzf "$out/$archive"
mkdir smoke/home
cp inputs/linux-singing.verp smoke/singing.verp
smoke="$run/smoke"
cli="$smoke/$name/bin/verdant-cli"
voice_id="verdant-voice-$version"
(
    cd "$smoke"
    export HOME="$smoke/home"
    python3 "$run/scripts/verify-voice-networkless.py" "$cli" doctor --json > "$out/linux-doctor.json"
    jq -e --arg pin "$pin" --arg voice "$voice_id" '.ok and
        (.result.capabilities | .voices.default_voice == $voice and .voices.voices[$voice].available
         and .voice_package.bundled_voice_admission.policy_sha256 == $pin)' "$out/linux-doctor.json" >/dev/null
    python3 "$run/scripts/verify-voice-networkless.py" "$cli" render singing.verp \
        --output "$out/linux-smoke.wav" --no-effects --json > "$out/linux-render.json"
)
jq -e '.ok and .result.frames == 12000 and .result.sample_rate == 48000 and
       (.result.phrases | length > 0 and all(.route == "verdant-voice"))' "$out/linux-render.json" >/dev/null
test ! -e "$smoke/home/.local/state/verdant/voices.json"
printf '%s  %s\n' "$(jq -er .linux_wav_sha256 run.json)" "$out/linux-smoke.wav" | sha256sum -c -

python3 - "$out" "$archive" "$checksum" "$selection" <<'PY'
import hashlib, json, pathlib, sys

out, archive, checksum, selection = sys.argv[1:]
root = pathlib.Path(out)
identity = json.loads(pathlib.Path('run.json').read_text())
def digest(path):
    with (root / path).open('rb') as data:
        return hashlib.file_digest(data, 'sha256').hexdigest()

receipts = [f'linux-{name}' for name in ('package.json', 'archive-guard.json', 'doctor.json',
                                       'render.json', 'smoke.wav', 'source-selection.json', 'build.json')]
result = {k: identity[k] for k in ('run_id', 'commit', 'tree', 'policy_sha256')}
result.update(status='verified', source_selection_sha256=selection, archive=archive, checksum=checksum,
              archive_sha256=digest(archive), fresh_state=True, empty_registry=True,
              fixture_sha256=identity['files']['inputs/linux-singing.verp'],
              receipts={name: digest(name) for name in receipts})
with (root / 'linux-result.json').open('x') as receipt:
    json.dump(result, receipt, indent=2)
    receipt.write('\n')
PY

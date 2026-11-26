#!/usr/bin/env bash
set -euo pipefail
run=/fast/run
out=/evidence
mkdir -p "$run/inputs/ort" "$run/scripts"
cd "$run"
repo=/workspace
trap 'code=$?; if ((code != 0)) && test -s "$out/linux-build.json"; then cp -R "$run/bin" "$out/recovery-bin"; chmod -R a+rX "$out"; fi; exit "$code"' EXIT
cp /driver/run.json run.json
cp /driver/verify-voice-networkless.py scripts/
cp /driver/singing.verp inputs/linux-singing.verp
cp /previous/share/verdant/BUNDLE-POLICY.json inputs/voice-policy.json
cp -a /previous/share/verdant/bundled-voice voice
cp /previous/lib/libonnxruntime.so.1.23.2 /previous/share/doc/verdant/onnxruntime/{LICENSE,ThirdPartyNotices.txt} inputs/ort/
jq -r '.files | to_entries[] | select(.key | test("^(inputs/(ort/|voice-policy.json|linux-singing.verp)|scripts/verify-voice-networkless.py)")) | "\(.value)  \(.key)"' run.json | sha256sum -c -
commit=$(jq -er .commit run.json)
pin=$(jq -er .policy_sha256 run.json)
version=$(jq -er .version run.json)
test "$(git -C "$repo" rev-parse HEAD)" = "$commit"
test "$(git -C "$repo" rev-parse 'HEAD^{tree}')" = "$(jq -er .tree run.json)"
test -z "$(git -C "$repo" status --porcelain --untracked-files=all)"
export SOURCE_DATE_EPOCH=$(jq -er .epoch run.json)
export VERDANT_BUNDLED_VOICE_POLICY_SHA256=$pin
export VERDANT_VOICE_BANK_SHA256=$(jq -er .bank_sha256 run.json)
export CARGO_TARGET_DIR=/build
mapfile -t workspace_packages < <(cargo metadata --manifest-path "$repo/frontend/Cargo.toml" --no-deps --locked --offline --format-version 1 | jq -r '.packages[].name')
clean=()
for package in "${workspace_packages[@]}"; do clean+=(-p "$package"); done
cargo clean --manifest-path "$repo/frontend/Cargo.toml" --offline --release "${clean[@]}"
source /driver/build-and-smoke.sh
printf '0\n' > "$out/linux-exit.txt"
chmod -R a+rX "$out"

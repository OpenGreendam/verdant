#!/usr/bin/env bash
set -euo pipefail
run=/fast/run
out=/evidence
mkdir -p "$run/inputs/ort" "$run/scripts"
cd "$run"
repo=/workspace
trap 'code=$?; printf "%s\n" "$code" > "$out/linux-exit.txt"; chmod -R a+rX "$out"; exit "$code"' EXIT
commit=$(cat /driver/source.sha)
[[ "$commit" =~ ^[0-9a-f]{40}$ ]]
test "$(git -C "$repo" rev-parse HEAD)" = "$commit"
tree=$(git -C "$repo" rev-parse 'HEAD^{tree}')
test "$tree" = "$(cat /driver/source.tree)"
export SOURCE_DATE_EPOCH=$(git -C "$repo" show -s --format=%ct HEAD)
jq --arg commit "$commit" --arg tree "$tree" --arg helper "$GITHUB_SHA" \
   --arg run "$GITHUB_RUN_ID" --arg attempt "$GITHUB_RUN_ATTEMPT" --arg epoch "$SOURCE_DATE_EPOCH" \
   '. + {commit:$commit,tree:$tree,helper_commit:$helper,run_id:$run,run_attempt:$attempt,epoch:($epoch|tonumber)}' \
   /driver/run.json > run.json
cp run.json "$out/linux-inputs.json"
cp /driver/verify-voice-networkless.py scripts/
cp /driver/singing.verp inputs/linux-singing.verp
cp /previous/share/verdant/BUNDLE-POLICY.json inputs/voice-policy.json
cp -a /previous/share/verdant/bundled-voice voice
cp /previous/lib/libonnxruntime.so.1.23.2 /previous/share/doc/verdant/onnxruntime/{LICENSE,ThirdPartyNotices.txt} inputs/ort/
jq -er '.files | to_entries[] | "\(.value)  \(.key)"' run.json | sha256sum -c -
jq -r '.files[] | "\(.sha256)  voice/\(.path)"' inputs/voice-policy.json | sha256sum -c -
pin=$(jq -er .policy_sha256 run.json)
version=$(jq -er .version run.json)
test -z "$(git -C "$repo" status --porcelain --untracked-files=all)"
export VERDANT_BUNDLED_VOICE_POLICY_SHA256=$pin
export VERDANT_VOICE_BANK_SHA256=$(jq -er .bank_sha256 run.json)
export CARGO_TARGET_DIR=/build
mapfile -t workspace_packages < <(cargo metadata --manifest-path "$repo/frontend/Cargo.toml" --no-deps --locked --offline --format-version 1 | jq -r '.packages[].name')
clean=()
for package in "${workspace_packages[@]}"; do clean+=(-p "$package"); done
cargo clean --manifest-path "$repo/frontend/Cargo.toml" --offline --release "${clean[@]}"
source /driver/build-and-smoke.sh

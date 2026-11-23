#!/bin/sh
# Rust production gate, run in the development container.
# Dependency acquisition precedes offline build, test, and packaging.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
: "${VERDANT_ONNXRUNTIME_LIBRARY:?Set the native ONNX Runtime 1.23.2 library for package validation}"
: "${VERDANT_ONNXRUNTIME_WHEEL:?Set the pinned ONNX Runtime 1.23.2 wheel for packaging}"
cd "$repo/frontend"
# Fresh compilation is the default production gate. A retained-only Rust lane
# must opt in explicitly, and CI makes that lane depend on proto-fresh. Missing
# upstream sources are never silently accepted as fresh-compilation evidence.
proto_ui_producer="$repo/frontend/proto-ui/tools/build-proto-ui.mjs"
proto_ui_rules="$repo/frontend/proto-ui/tools/check-proto-ui-rules.mjs"
if command -v node >/dev/null 2>&1; then
    node "$proto_ui_rules"
    case "${VERDANT_PROTO_UI_MODE:-fresh}" in
        fresh)
            if [ -n "${VERDANT_PROTO_UI_UPSTREAM:-}" ]; then
                node "$proto_ui_producer" --check --upstream "$VERDANT_PROTO_UI_UPSTREAM"
            else
                node "$proto_ui_producer" --check
            fi ;;
        retained)
            printf 'Checking retained Proto-UI artifacts; this lane does not prove fresh compilation.\n'
            node "$proto_ui_producer" --check-retained ;;
        *) printf 'VERDANT_PROTO_UI_MODE must be fresh or retained.\n' >&2
           exit 4 ;;
    esac
else
    printf 'Proto UI artifact validation requires node.\n' >&2
    exit 5
fi
# A cold CI cache must be populated explicitly before offline compilation.
# Pre-provisioned disconnected containers may set CARGO_NET_OFFLINE=true.
cargo fetch --locked
# Full metadata includes build/dev/target-specific edges. It also proves that
# the real session/protocol closure reaches the std-only data crates without
# reaching the native components, SDK, GPUI or image support in this workspace.
mkdir -p "${CARGO_TARGET_DIR:-target}"
proto_ui_graph="${CARGO_TARGET_DIR:-target}/proto-ui-gpui-graph.json"
node "$repo/frontend/proto-ui/tools/check-cargo-gpui.mjs" --offline > "$proto_ui_graph"
cat "$proto_ui_graph"
cargo build --offline --locked --workspace --features verdant-test-worker/protocol-fixture
worker_target=$(CDPATH= cd -- "${CARGO_TARGET_DIR:-target}" && pwd)
reject_package_target() {
    printf 'Refusing package cleanup in a root or source directory: %s\n' "$worker_target" >&2
    exit 2
}
case "$worker_target" in ""|/) reject_package_target;; esac
case "$repo/" in "$worker_target/"*) reject_package_target;; esac
case "$worker_target/" in
    "$repo/frontend/target/"*) ;;
    "$repo/frontend/"*|"$repo/backend/"*|"$repo/tools/"*|"$repo/docs/"*|\
    "$repo/neural/"*|"$repo/research/"*|"$repo/spec/"*|"$repo/.github/"*|"$repo/.git/"*)
        reject_package_target;;
esac
export VERDANT_WORKER_BINARY="$worker_target/debug/verdant-worker"
export VERDANT_TEST_WORKER_BINARY="$worker_target/debug/verdant-test-worker"
cargo test --offline --locked --workspace --features verdant-test-worker/protocol-fixture
cargo test --offline --locked -p proto-ui-gpui-native --features test-support,gpui/test-support --lib -- --test-threads=1
cargo test --offline --locked -p proto-ui-gpui-native --features test-support,gpui/test-support,gpui-modern-element-api,gpui-text-selection-notification --lib -- --test-threads=1
# Packages need the optimized DSP host: debug oversampling can miss realtime deadlines.
cargo build --offline --locked --release -p verdant-app -p verdant-cli -p verdant-launcher \
    -p verdant-worker -p verdant-plughost -p verdant-voice -p verdant-release
# CI caches the target directory and uploads package-smoke.*. Remove only
# these generated smoke outputs so an older cached package cannot be uploaded
# alongside the payload checked in this run. rm removes links without following
# them; unrelated target entries and their contents are retained.
for previous_package in "$worker_target"/package-smoke.*; do
    if [ -d "$previous_package" ] || [ -L "$previous_package" ]; then
        rm -rf -- "$previous_package"
    fi
done
package_output=$(mktemp -d "$worker_target/package-smoke.XXXXXX")
# mktemp creates 0700; CI uploads the package as the runner user, outside
# this container, so the directory must be listable by others.
chmod 0755 "$package_output"
source_selection=$(mktemp -d "$worker_target/source-selection.XXXXXX")/selection.json
release="$worker_target/release/verdant-release"
selection_sha=$("$release" select-source --repo "$repo" --output "$source_selection" | jq -er .sha256)
source_commit=$(git -C "$repo" rev-parse HEAD)
SOURCE_DATE_EPOCH=$(git -C "$repo" show -s --format=%ct HEAD)
export SOURCE_DATE_EPOCH
"$release" package --repo "$repo" --binaries "$worker_target/release" --output "$package_output" \
    --source-selection "$source_selection" --source-selection-sha256 "$selection_sha" \
    --onnxruntime-wheel "$VERDANT_ONNXRUNTIME_WHEEL"
"$release" check --check "$package_output" --require-package --expected-source-commit "$source_commit" \
    --expected-source-selection-sha256 "$selection_sha"

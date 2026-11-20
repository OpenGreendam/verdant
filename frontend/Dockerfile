# syntax=docker/dockerfile:1
ARG RUST_IMAGE=rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e
FROM ${RUST_IMAGE}

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    build-essential clang cmake ninja-build pkg-config ca-certificates curl jq nodejs unzip \
    libasound2-dev libfontconfig1-dev libfreetype6-dev libssl-dev libudev-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev \
    libxcb1-dev libxcb-xkb-dev libvulkan-dev libsndfile1 \
    xvfb xauth xdotool dbus-x11 at-spi2-core weston imagemagick \
    fonts-dejavu-core fonts-noto-cjk mesa-vulkan-drivers libgl1-mesa-dri \
    && apt-get clean

# Native CPU library only: the wheel is a hash-bound acquisition container,
# never installed or placed on the production interpreter/module search path.
ARG ONNXRUNTIME_WHEEL_URL=https://files.pythonhosted.org/packages/ef/88/9cc25d2bafe6bc0d4d3c1db3ade98196d5b355c0b273e6a5dc09c5d5d0d5/onnxruntime-1.23.2-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl
ARG ONNXRUNTIME_WHEEL_SHA256=0f9b4ae77f8e3c9bee50c27bc1beede83f786fe1d52e99ac85aa8d65a01e9b77
RUN mkdir -p /opt/verdant-native/lib \
    && curl --fail --location --max-time 120 "$ONNXRUNTIME_WHEEL_URL" --output /opt/verdant-native/runtime.whl \
    && printf '%s  %s\n' "$ONNXRUNTIME_WHEEL_SHA256" /opt/verdant-native/runtime.whl | sha256sum --check - \
    && unzip -p /opt/verdant-native/runtime.whl onnxruntime/capi/libonnxruntime.so.1.23.2 \
       > /opt/verdant-native/lib/libonnxruntime.so.1.23.2
ENV VERDANT_ONNXRUNTIME_LIBRARY=/opt/verdant-native/lib/libonnxruntime.so.1.23.2 \
    VERDANT_ONNXRUNTIME_WHEEL=/opt/verdant-native/runtime.whl

ENV CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_TARGET_DIR=/build
WORKDIR /workspace/frontend
CMD ["bash"]

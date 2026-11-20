# Plugin host third-party notices

These notices cover `verdant-plughost` SDK dependencies and the separately acquired Steinberg SDK used for acceptance fixtures. Versions refer to [Cargo.lock](../Cargo.lock) and the retained fixture source. Plugin operation and limits are documented in [PLUGIN-HOST.md](PLUGIN-HOST.md).

Retain the following license texts in distributions that include these components. They grant no rights to other installed plugins or voice models; package exclusions are defined in [RELEASE.md](../../RELEASE.md).

| Component | Version | Upstream license | Distribution choice here |
| --- | --- | --- | --- |
| `clack-host`, `clack-extensions`, `clack-common` | 0.2.0 | MIT OR Apache-2.0 | MIT |
| `clap-sys` | 0.5.0 | MIT OR Apache-2.0 | MIT |
| `vst3` Rust bindings | 0.3.0 | MIT OR Apache-2.0 | MIT |
| `vst3-host` | 0.11.1 | MIT | MIT |
| Steinberg VST3 SDK, external fixture build only | 3.8.1 | MIT, copyright 2026 Steinberg Media Technologies GmbH | MIT |

Source license files: [clack-host](https://docs.rs/crate/clack-host/0.2.0/source/LICENSE-MIT), [clack-extensions](https://docs.rs/crate/clack-extensions/0.2.0/source/LICENSE-MIT), [clack-common](https://docs.rs/crate/clack-common/0.2.0/source/LICENSE-MIT), [clap-sys](https://docs.rs/crate/clap-sys/0.5.0/source/LICENSE-MIT), [vst3](https://docs.rs/crate/vst3/0.3.0/source/LICENSE-MIT), [vst3-host](https://docs.rs/crate/vst3-host/0.11.1/source/LICENSE), and [Steinberg VST3 SDK](https://github.com/steinbergmedia/vst3sdk/blob/3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96/LICENSE.txt).

## clack 0.2.0

The following notice is present in each of `clack-host`, `clack-extensions`, and `clack-common`:

```text
Copyright (c) 2021 Adrien Prokopowicz

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## clap-sys 0.5.0 and vst3 0.3.0

Each of these two packages supplies the following `LICENSE-MIT` text, without a copyright header in that file:

```text
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## vst3-host 0.11.1

```text
MIT License

Copyright (c) 2026 Helge Sverre

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Steinberg VST3 SDK 3.8.1

This is the license of the retained external fixture SDK source, commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`. The SDK fixture and its built example plugins are not application release payloads.

```text
MIT License

Copyright (c) 2026, Steinberg Media Technologies GmbH

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Native system frameworks and release contents

AU hosting calls the macOS AudioToolbox, AppKit and CoreFoundation system frameworks. No Apple SDK or framework binaries are copied into the application package. Native system API declarations do not establish that an AU has passed runtime validation.

The package dependency inventory must retain platform-binding and transitive Rust dependency licenses alongside these SDK notices. Third-party plugin fixtures, original banks, derived bank data and learned weights remain outside release contents. Native framework declarations and package assembly do not establish Windows/macOS runtime acceptance; see [PLATFORM-ACCEPTANCE.md](PLATFORM-ACCEPTANCE.md).

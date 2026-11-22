# Linux AT-SPI protocol observations

These scripts observe the real application tree through AT-SPI. They record
role, accessible name, numeric values, focus state, advertised actions and
platform events. They can also request focus and explicitly requested actions.
They do not run a physical screen reader or establish speech, Braille or
assistive-technology usability.

Use a disposable project. A successful numeric write must also be checked at
the application boundary: save the project through Verdant, read the persisted
parameter, and check undo/cancellation as appropriate. A D-Bus reply alone does
not establish a domain commit.

## Isolated dependencies

The following was exercised on Debian 13 with Python 3.13 and matching Debian
packages. Package names and Python extension ABIs are distribution-specific.
Existing system `dbus-run-session`, `dbus-daemon`, Python and their base shared
libraries are required. No `apt install`, system service changes, or changes to
the user's D-Bus/GSettings configuration are needed.

```sh
deps=/absolute/path/to/private-a11y
mkdir -p "$deps/debs" "$deps/root"
cd "$deps/debs"
apt-get download at-spi2-core at-spi2-common libatspi2.0-0t64 \
  gir1.2-atspi-2.0 python3-dbus python3-gi python3-pyatspi \
  libgirepository-1.0-1 gir1.2-glib-2.0 gir1.2-girepository-2.0 \
  gir1.2-freedesktop gsettings-desktop-schemas libglib2.0-0t64
for package in ./*.deb; do dpkg-deb -x "$package" "$deps/root"; done
```

`libatspi2.0-0t64` is the Debian 13 package name; the older `libatspi2.0-0`
name has no candidate there. The runner compiles the extracted GSettings
schemas into the extracted directory if needed. It uses a matching
`glib-compile-schemas` binary from the extracted tree or the system's multiarch
library directory.

## Run a private session

Wrap the existing OS test harness, which starts and keeps the application
alive, in the session runner. Resource limits and compositor/window setup
belong to that harness. The wrapper itself starts only D-Bus and AT-SPI
processes; it does not start Verdant unless supplied as the command.

```sh
DISPLAY=:90 python3 tools/a11y/run-isolated.py \
  --deps-root "$deps/root" --run-dir "$deps/x11-observation" \
  -- /absolute/path/to/os-test-harness.sh
```

The run directory must be new, and its socket paths must be short enough for
Unix sockets. For Wayland, supply the private compositor's `WAYLAND_DISPLAY`
and `XDG_RUNTIME_DIR`. The runner preserves a relative Wayland socket as an
absolute path before creating a separate runtime directory for the test.
Unset `DISPLAY` when intentionally exercising only Wayland.

All children receive private `DBUS_SESSION_BUS_ADDRESS` and
`AT_SPI_BUS_ADDRESS`. The run directory contains `environment.json` and
`environment.sh` so another process can join that same session while the
harness remains alive. Only source the environment file produced by your own
private run.

The runner starts the packaged `at-spi-bus-launcher` only to publish
`org.a11y.Status.IsEnabled = true`, with `ScreenReaderEnabled = false` and the
GSettings memory backend. It starts a real accessibility `dbus-daemon` with
a private config, followed by the real `at-spi2-registryd`. This avoids the
launcher's compiled-in `/usr` config paths. `GetAddress` on that launcher is
deliberately unused: the tested AccessKit and libatspi clients both honor the
explicit `AT_SPI_BUS_ADDRESS`. Neither bus has system service activation
directories. The runner terminates its own bus/registry processes when the
supplied harness exits.

A dependency/registry smoke check, without starting an application:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY python3 tools/a11y/run-isolated.py \
  --deps-root "$deps/root" --run-dir "$deps/empty-session" -- \
  python3 -c 'import pyatspi; d=pyatspi.Registry.getDesktop(0); print(d.getRoleName(), d.childCount); assert d.childCount == 0'
```

## Observe and exercise the actual tree

In another process, while the private harness is running:

```sh
. "$deps/x11-observation/environment.sh"
python3 tools/a11y/observe-atspi.py --app verdant --output "$deps/tree.json"
```

The default role filter selects sliders, while the evidence includes the
whole matching application subtree. Use an exact accessible name to select
one knob before interaction. The following name and value are examples;
choose a live parameter in the disposable test project. Ensure the application
window is active before requiring the focused state.

Roles are read through the standard numeric `GetRole` method and the installed
AT-SPI enum definitions. The observer records both SCREEN and WINDOW extents;
it does not assume that a provider's SCREEN bounds are valid global coordinates.
During application discovery, only transient `UnknownObject`/`UnknownInterface`
errors from the registry's application references are retried and recorded.
This handles AccessKit's registration order without hiding a persistent
missing application or unrelated D-Bus errors.

Snapshots also include `windowReference` and `windowName`, derived by walking
the provider's actual frame/window ancestry. When the main and native plugin
windows contain identically named sliders, `--window NAME` limits selection
to the exact observed accessible frame name. If that name is empty or is
shared, `--window-reference BUS PATH` selects the exact frame reference from
the current session's snapshot. References from a terminated run must not be
reused. Ambiguous interaction targets fail rather than selecting the first
matching name. This multiple-window addition has only had local syntax/helper
validation; its native-window OS cases remain unexecuted after remote quota
exhaustion, as recorded in `tools/native-input/README.md`.

```sh
python3 tools/a11y/observe-atspi.py --app verdant --name Cutoff --focus \
  --output "$deps/focus.json"
python3 tools/a11y/observe-atspi.py --app verdant --name Cutoff \
  --set-value 0.63 --expect-value 0.63 --output "$deps/value.json"
```

`--set-value` invokes the actual `org.freedesktop.DBus.Properties.Set` call
for `org.a11y.atspi.Value.CurrentValue`. It requires `--expect-value`; success
requires the observed value to converge to that expectation and remain there
in the final snapshot. Use `--tolerance` for a parameter whose legitimate
discrete conversion requires it. The script never substitutes a keyboard or
pointer event for this protocol write.

`--require-action NAME` checks a name from `Action.GetActions`.
`--action NAME` invokes its actual index with `Action.DoAction`, and fails
if the provider did not advertise that action. Use `--expect-value` as well
when validating a value change. The currently pinned Linux
`accesskit_atspi_common 0.19.1` exposes the Action interface for click actions;
it does not map the AccessKit Increment/Decrement variants to AT-SPI actions.
Numeric editing on that bridge uses the Value setter. An absent Increment
action must not be replaced by a click or reported as tested.

JSON evidence uses `verdant.atspi-observation.v1` and includes:

- `before` and `after`: applications, complete traversed node records and any
  per-node read errors;
- `matches`: nodes selected by application, role and exact name;
- `operations`: method, requested value/action, protocol result where present,
  and fresh node state before/after each operation;
- `events`: actual object/window signals, path, sender and elapsed time;
- `ok`, `error`, `expectedValue` and `tolerance`: explicit assertion results.

`ok` applies only to the requested observations/assertions. Check the saved
project and real OS input/lifecycle observations separately. Preserve all
failed observations rather than counting an accepted D-Bus request as a
successful edit.

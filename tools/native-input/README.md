# Real window input probe

`probe.py` launches a disposable Verdant project and its real worker, finds
controls through the actual AT-SPI tree, and drives the visible application
using XTest (`xdotool`). It checks saved `.verp` data after real Ctrl+S events.
No editor methods, test contexts, exposed Proto methods or synthetic GPUI
events are called.

The caller supplies a private display/compositor, the private AT-SPI session
from `tools/a11y/run-isolated.py`, and CPU/memory limits for the entire harness.
The script does not build binaries or install dependencies. It needs Python,
the prepared D-Bus/GI environment, `xdotool`, and libX11 for screenshots.

## Execution status on 2026-11-22

The complete recorded suite passed **22/22 cases with no skips**: eleven on
X11 and eleven on windowed Wayland, including the actual native plugin
window close/reopen and preset-menu cancellation cases. The successful
[CI run 45026004598](https://github.com/OpenGreendam/verdant/actions/runs/45026004598)
used job `137223338246`, Ubuntu 24.04 and **Weston 13.0.0**.

| Identity | Recorded value |
|---|---|
| Branch head | `a55bdcd4cc73e3a48cdfc956151e9934cc865f48` |
| Actual checked-out/tested synthetic merge commit | `9e6cd34715b9da4b733466b95d749764e5c410b3` |
| Application SHA-256, both backends | `273e3ea6fa8015d436ea885ccc7eaf721c4e80d71950ae60d1659d412793cf20` |
| Worker SHA-256, both backends | `c4542e8a6f151c7b2f9556e0008547646c4a55ce180eda3ffdf8e40963765033` |
| Compiler pin | `5712266648fdbce2f6662636cb83f01eb849adb1` |
| Evidence ZIP SHA-256 | `588dd0087e554d621b40f0af2e860815a0c4175b5a5b7da8ccabed7feb967c5b` |

X11 took 111.849 seconds; Wayland took 114.044 seconds. Each retained **40
observed steps**, including AT-SPI trees, numeric/focus state, saved project
bytes and screenshots. Both projects finished at **−8 dB, revision 12**.
The independent local readback matched the marked job-log summary to the
downloaded artifact, verified its GitHub ZIP digest, and reran the unchanged
artifact validator against every retained step. Its reconstructed results
exactly matched the CI report. Source-selection, manifest and observation
source hashes also matched.

The full per-case gain/revision/hash/focus observations and Wayland contact
trace are in
[`NATIVE-INPUT-2026-11-22.json`](../../frontend/proto-ui/NATIVE-INPUT-2026-11-22.json).
The raw [native-input-linux artifact](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/artifacts/14756497529)
is retained until 2026-12-06 by CI policy. The historical 18/18 remote baseline
on `8fc5f4e675b8cb28801cb622ff58f45508da6e23` remains a separate record; it is
not substituted for this current 22-case result.

Offline tests cover frame selection, Weston scene parsing, recorded
pointer-contact interpretation, and refusal of incomplete, filtered or
mismatched CI observations:

```sh
python3 -B -m unittest discover -s tools/native-input -p 'test_*.py' -v
```

These offline regressions do not start GPUI, D-Bus or an operating-system window.
The native preset selector cases run the actual `Probe.native_preset_cancel`
lookup through `snapshot`, `nodes` and `node`, stopping before the accessibility
transport. They reject another window's matching name, a wrong role, the legacy
label and ambiguous native matches.

The control journey selects AT-SPI's actual `push button` role for dialog
actions and still requires the observed node to have physical focus. In
[run 45140053560](https://github.com/OpenGreendam/verdant/actions/runs/45140053560),
X11 had already focused the **取消** push button, but the former generic
`button` selector could not find it. The same run's Wayland trace delivered
the temporary Unicode keycodes without their X11-only symbol mappings;
the actual textbox consequently contained `AB` instead of `音😀AB`. Its
original 22 cases passed, while the expanded six control cases remained
incomplete. Those results are retained separately from any later rerun.

### Retained failed attempts

The replacement local environment loaded the official Debian 13 D-Bus/GI
dependencies, but `socket(AF_UNIX, SOCK_STREAM)` itself returned `EPERM`
before bind. Its process reported `Seccomp: 2` and `NoNewPrivs: 1`; consequently
even the private `dbus-run-session` stopped with “Failed to open socket:
Operation not permitted”. No application OS pass is claimed for that local
environment. The independent CI job below runs the expanded suite on its
own Ubuntu runner, without reclassifying either environment limitation as a pass.

The first CI attempt, [run 45023915036](https://github.com/OpenGreendam/verdant/actions/runs/45023915036),
passed package/source identity checks and all nine original X11 cases. Its
native close case stopped because the actual editor contains one **关闭** push
button and two identically named radio choices. The selector now requires
the push-button role inside that observed frame. Weston 13 created its output,
but title-based discovery failed before a Wayland app started; the wrapper now
uses that owned compositor's logged output ID and checks its actual X11
geometry. Both failures remain recorded in the original `native-input-linux`
artifact. The corrections have offline regressions and were then exercised by
the complete 22-case successful run identified above.

## CI package and evidence gate

The `native-input` job in `.github/workflows/frontend.yml` depends on `linux`
and downloads that same workflow run's `verdant-linux` artifact. It installs
the OS display/accessibility dependencies only in this separate Ubuntu 24.04
job. The plain Rust `headless-session` job and its dependency proof stay separate.

`ci.py prepare` requires exactly one archive/checksum pair, bootstraps only
its fixed `bin/verdant-release` member, then uses that existing checker to
verify both the archive and the extracted directory. Expected commit comes
from the actual workflow checkout, including a pull request's synthetic merge
commit; source selection is recomputed from that checkout. The checker verifies
the source offer, exact members/modes/hashes, executable identities and pinned
runtime library. The same-run build job is the bootstrap trust boundary;
this is not an independent signature or a reproducible-binary-build claim.

After integration of main `6ac8a3396f6155e6ade4968169bc43ca0c7469e4`, the
current gate requires `verdant-native-package/3`. It selects the manifest's
`verdant-gui` program, role `gpui`, at `libexec/verdant/verdant-gui`, with
its matching `bin/verdant-worker` and packaged resource/library environment.
The `bin/verdant` CLI and root `verdant-launcher` remain distinct programs;
neither can satisfy the GUI name, role and path checks. Manifest, application,
worker and observation-source hashes are checked again before execution.
The #79 result above retains its original checkout and package/2 identities;
validation of the later integration is recorded separately.
X11 and windowed Wayland run sequentially with no diagnostic options; the
Wayland attempt still runs after an X11 failure to preserve both observations.

`ci.py check` rejects missing results, skipped or duplicate cases, any count
other than eleven per backend, filters, different app/worker bytes, absent
screenshots or saved-project evidence, and missing actual Wayland input/contact
evidence. The complete sequence must finish at −8 dB and revision 12: six
commits and six single undos, with cancellation cases adding no revision.
Failure in preparation or either run fails the job. An `always()` upload keeps
all ordinary evidence files, including failed runs, as `native-input-linux`;
runtime sockets and links are listed but never followed or uploaded as files.

The evidence contains `result.json`, compact `summary.json`, package identity
and checker reports, all step trees/screenshots/project copies, application
stdout/stderr, D-Bus/compositor logs and actual installed package versions.
The same compact summary is printed between
`BEGIN_VERDANT_NATIVE_INPUT_JSON` and `END_VERDANT_NATIVE_INPUT_JSON` for
read-only job-log retrieval. Each case keeps its measured normalized values,
saved gain/revision/hash, observed focus, held contact and window scope.
The summary's `ok` can become true only after both complete runs pass.

## Invocation

`run-linux-isolated.py` can prepare the whole private Linux session from an
already extracted dependency prefix, or installed packages with `--deps-root /`.
Run the two backends sequentially with
the same application and worker. The run directory must be new and short
enough for Unix socket paths. On Debian 13, `/usr/bin/python3` matches the
packaged Python 3.13 D-Bus/GI extensions; the primary virtual-environment Python
may have another ABI.

```sh
/usr/bin/python3 tools/native-input/run-linux-isolated.py \
  --deps-root /absolute/path/to/extracted-dependencies \
  --repo /absolute/path/to/verdant-repository \
  --binary /absolute/path/to/verdant-gui \
  --worker-binary /absolute/path/to/verdant-worker \
  --backend x11 --run-dir /tmp/verdant-x11-new

/usr/bin/python3 tools/native-input/run-linux-isolated.py \
  --deps-root /absolute/path/to/extracted-dependencies \
  --repo /absolute/path/to/verdant-repository \
  --binary /absolute/path/to/verdant-gui \
  --worker-binary /absolute/path/to/verdant-worker \
  --backend wayland --run-dir /tmp/verdant-wayland-new
```

The wrapper starts its own Xvfb, optional windowed Weston, and the existing
isolated AT-SPI runner. It creates private Lavapipe/fontconfig/XDG settings,
sets a two-CPU affinity and an 8 GiB **per-process address-space** limit, and
records those actual limits. This is not an aggregate cgroup memory claim.
It terminates only the process groups it created. It needs Xvfb,
xdotool, xkbcomp (`x11-xkb-utils`), Weston 13 or 14 (including its desktop shell and debug client), Lavapipe,
fonts and the AT-SPI/Python dependencies described in `tools/a11y/README.md`.
It records the actual `weston --version` and `--help` output; it does not label
Ubuntu's Weston 13 as the Weston 14 used in the earlier remote baseline.
Its nested X11 host ID comes from the owned compositor's actual `x11 output`
log entry, and its observed X11 dimensions must match that entry. Missing,
invalid or multiple output IDs fail; dimensions never select a guessed ID.
With installed system packages, missing GSettings schemas cause a failure;
the wrapper does not compile or change system schemas.

For `--suite controls`, the wrapper uses `x11_keymap.py` to install persistent
Unicode key symbols on the Xvfb display it just created, before either Weston
or Verdant starts. It chooses unused keys, preserves existing bindings and
modifiers, and reads the compiled keymap back from that private X server.
The original, prepared and observed keymaps and their binding record are
retained with the run. `xdotool` then sends ordinary XTest key events using
these symbols, including the characters outside the BMP; no text is assigned
through application state or an accessibility text setter. The actual
AT-SPI text, character selections, focus, saved project and undo/cancel
checks remain the acceptance evidence. This Unicode keyboard route does
not claim input-method composition coverage or change a user's desktop map.

For an already prepared display/compositor and AT-SPI session, invoke the
probe directly:

```sh
python3 tools/native-input/probe.py \
  --binary /absolute/path/to/verdant-gui \
  --worker-binary /absolute/path/to/verdant-worker \
  --repo /absolute/path/to/verdant-repository \
  --output-dir /absolute/path/to/new-evidence-directory \
  --backend x11 --input-display :90
```

The worker defaults to `verdant-worker` beside the application. The output
directory must be new. It contains an independently constructed, single-track
builtin synth project, with initial gain **−8 dB** in the **[−24, 6] dB** range.
It copies no historical example, imported-score provenance or voice identity.
Private state, cache, data and worker directories belong to that run. The app
starts with `--silent-audio` and Chinese UI preferences.

The authored synth gain name is **音量**, not 增益. The toolbar names are
**效果器链** and **钢琴卷帘 (E)**. These names and the slider role are looked up
in the live tree before using their observed bounds.

`--only preflight` starts the app, checks the actual tree/coordinate mapping,
opens the devices pane, requests platform focus and saves the initial project.
It then cleans up. `--only NAME` can select individual cases. Use
`--skip-set-value` only to diagnose a binary that predates the numeric
accessibility action; the result records those cases as skipped.

### Nested Wayland

For Wayland add `--backend wayland --outer-window XID`, using the actual X11
host window of the private nested Weston compositor. For all eleven cases, also
pass `--wayland-windowed --weston-debug /private/path/to/weston-debug` and use
Weston 13/14's desktop shell with `--debug`, no panel or animations, and a
1600×1000 output inside a larger private X11 display (for example 1920×1200).
Set the shell's `client=` to the extracted `weston-desktop-shell` path. The
application stays unmaximized, leaving room inside the compositor output to
actually move outside its window. Debug access belongs only to this private
compositor.

The app process has `DISPLAY` removed and receives `WAYLAND_DEBUG=client`.
Only `xdotool` and the screenshot capture use `--input-display`. The route is
**XTest → nested Weston X11 host → Wayland → Verdant**, not a physical mouse
or direct hardware Wayland input test.

The probe records both AT-SPI SCREEN/WINDOW extents and the native X11
geometry. The pinned GPUI bridge can report invalid frame SCREEN bounds and
window-local child positions for SCREEN; these observations are preserved,
not treated as repaired global coordinates. Input uses standard WINDOW
coordinates plus the native window's observed origin.

In windowed mode the actual `weston-debug scene-graph` must identify exactly
one selected `xdg_toplevel` belonging to the launched PID and `verdant` app id.
Once the native editor opens, its exact title also selects the surface. Its
global view rectangle and attached buffer dimensions establish the surface
origin and size. The full observed scene is retained. Input uses this measured
offset, and the outside-release point must remain inside the compositor
output while leaving the application rectangle.

The Weston 13 compatibility check used Ubuntu's official
[`weston_13.0.0.orig.tar.xz`](https://archive.ubuntu.com/ubuntu/pool/universe/w/weston/weston_13.0.0.orig.tar.xz),
SHA-256 `52ff1d4aa2394a2e416c85a338b627ce97fa71d43eb762fd4aaf145d36fc795a`.
Its `libweston/compositor.c` prints the PID, surface ID, transformed rectangle
and buffer dimensions; `libweston/shell-utils/shell-utils.c` supplies the
actual title/app id. `compositor/main.c` advertises `--renderer=pixman`,
and `weston_module_path_from_env` accepts the same semicolon-delimited module
map. These source checks preceded the successful recorded Weston 13 OS run;
the actual runtime version remains part of its evidence.

The observer annotates every node with the frame reference reached through
the real tree's parent/child traversal. Main and plugin windows can both
contain an **音量** slider, so the probe selects the observed frame before
looking up a control. Native editor input uses its actual X11 title and
geometry, or its matching Weston view and buffer; the main window's origin
is never reused for the child. Every mapping and scene snapshot is retained.

The earlier kiosk configuration is still useful for limited observations:
its actual positive `xdg_toplevel.configure` dimensions must match the outer
client (within four pixels), unless an explicit measured `--surface-offset`
is supplied. It cannot establish window-exit cancellation when the application
fills the entire output: the observed compositor clamped an XTest position
of 1620 to client x=1599 without sending `wl_pointer.leave`. That failed
observation is retained, rather than counted as an application failure or a
passed exit test. No unknown surface position is inferred from its size.

Raw Wayland traffic is retained in `app.stderr.log`. The result counts
incoming `wl_pointer` and `wl_keyboard` events. A Wayland run must contain
actual pointer-button and keyboard-key events to pass its backend evidence
check. Before its first click, the probe also waits for actual pointer and
keyboard enter events: the accessibility tree can become visible before the
first rendered buffer maps the surface, when the compositor has no input
target yet. The final report also records the native contact state at both
Escape key events and actual outside-window motion/release coordinates. A
passed Escape case must preserve its held native button, and a passed
windowed exit case must contain a measured outside-window motion sample.

## Assertions and limits

| Case | Input and checked result |
|---|---|
| Keyboard | Click focus, Arrow Up, +0.01 normalized value = +0.3 dB, one saved revision, one undo restores −8 dB. |
| Drag | Hold the mouse and move upward 54 logical pixels, observe +9 dB provisional display, release, one revision and one undo. |
| Escape | Escape during a held preview, then late mouse up; saved gain, revision and document hash remain unchanged. |
| Focus | Real `Component.GrabFocus` on another knob during a held preview cancels it; late up writes nothing. A subsequent real mouse click verifies that other knob receives focus. |
| Window exit | Move outside the application window while held, observe restoration, release outside, then verify no saved revision. In nested Wayland, the point remains inside the compositor output so its edge cannot clamp the intended exit. |
| Detach | Invoke the real toolbar's advertised AT-SPI click while the mouse is held, observe the knob subtree leave the actual tree, late up writes nothing, then reopen with a real mouse click. |
| Numeric accessibility | Write `Value.CurrentValue = 0.63`, read it back, save −5.1 dB as one revision, then undo once. |
| Numeric takeover | Write the same numeric value during a pointer preview, save before up, send another move and late up, and require no second revision or document change; undo once. |
| Lease reuse | Perform another real drag and undo after the cancellation cases, demonstrating that a new pointer edit can start and finish. |
| Native window close | Click the instrument card's **打开界面** button, drag its native window's gain while held, invoke that window's advertised **关闭** push-button action, observe the frame disappear, send the late up, and save in the main window with identical gain/revision/hash. Reopen through the same real button, make a new 18-pixel drag, close, then save and undo once. |
| Native preset menu | Open the native window, hold a gain preview, invoke its **预设** combobox's advertised click, observe **上下文菜单** and the restored gain, dismiss with Escape while still held, then release, close and verify the main saved document is unchanged. |

The fixture's native editor is opened through the actual instrument card,
not by creating an application entity or calling an editor command. Its title
is **Verdant Synth — Verdant** and its named root is **Verdant Synth 原生编辑器**.
The close test calls the provider's actual `Action.DoAction`; it does not call
the shared Rust semantic executor. The preset case likewise requires the
real advertised action and an observed menu. Saving occurs only after a real
click focuses the original main window, since the plugin window has separate
keyboard handling.

The current native synth toolbar exposes **预设** as a combobox. The lookup
requires that exact name and role inside the observed native frame, where one
match must exist; the main window also has **预设** and **自定义** controls.
The retained #79 observation used the earlier native **自定义** chooser and
keeps that original label in its historical evidence.

A second primary-button click cannot occur while that same physical button is
already held. Therefore the focus and active-toolbar cases explicitly use
the real accessibility protocol for that independent input. Their evidence
does not call those operations mouse clicks during a drag. No click is used
to impersonate Increment/Decrement: numeric editing calls the actual Value
setter, and the toolbar action must be advertised by its provider.

While a pointer is held, keyboard commands deliberately omit xdotool's
`--clearmodifiers`: that option also releases mouse buttons. The original
failed Escape trace showed a pointer-up immediately before the key, which
tested a commit followed by Escape instead of cancellation. The corrected
probe preserves the held contact and records the actual command and contact
state. D-Bus Properties.Set uses its required `ssv` signature with an explicit
double variant, rather than relying on introspection or inferred scalar types.

Ctrl+S must produce an actual atomic file replacement (inode or modification
time changes). The probe then reads the new document and checks its revision
and numeric value. Thus an old saved value cannot satisfy a save check merely
because a key event was sent. Each undo must restore the preceding gain with
exactly one additional monotonic revision. Cancellations also require identical
saved document bytes after saving.

Only displacement-derived drag expectations allow **0.001 dB** of numeric
error: a real X11 54-pixel sample produced 1.0001373291015625 dB instead of
the ideal 1 dB after native floating-point pixel conversion. The failed strict
run is retained. The final CI run measured 1.0001424153645821 dB on X11 and
0.9999999999999964 dB on Wayland; both use that same predeclared 0.001 dB
pixel-drag tolerance. Keyboard and accessibility edits use the original strict
numeric comparisons; cancellation/undo values, revisions and hashes are not
relaxed. The result records both this tolerance and the actual observed values.

`result.json` records binary hashes, route, process identity, coordinate
mapping, exact input commands, case results and links to each step. Each step
has the actual AT-SPI tree/state/value, saved revision/hash, a copied `.verp`,
and a PNG of the private input display. A failed accessibility query still
gets an independent screenshot attempt. Failures remain failures; a skipped
numeric action is not a completed accessibility validation.

Normal completion, Python exceptions and SIGTERM clean up only the app/worker
process group created by the probe. The caller owns the compositor and the
private buses. This probe does not validate physical touch hardware or the
experience of a physical screen reader.

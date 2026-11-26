#!/usr/bin/env python3
"""Exercise a real Verdant window through XTest and the actual AT-SPI bus.

Run inside tools/a11y/run-isolated.py and the caller's resource-limited OS
harness. The script starts only its own Verdant/worker process group. On
Wayland, XTest targets the supplied nested Weston's X11 host window, which
then delivers real Wayland input to Verdant. No application test hooks are used.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import runpy
import shutil
import signal
import subprocess
import sys
import time
import traceback
import uuid

from x11_screenshot import capture
from controls import CONTROL_CASES, ControlCases


CASES = ("keyboard", "drag", "escape", "focus", "exit", "detach",
         "set-value", "set-value-during-drag", "lease-reusable",
         "native-window-close", "native-preset-cancel")
NATIVE_WINDOW_TITLE = "Verdant Synth — Verdant"
NATIVE_ROOT_NAME = "Verdant Synth 原生编辑器"
GAIN_MIN, GAIN_MAX, INITIAL_GAIN = -24.0, 6.0, -8.0
# XTest's integer input coordinates pass through GPUI's floating-point native
# pixel conversion. The measured 54-pixel X11 sample differed by 0.00013733 dB.
# Keep this tolerance local to displacement-derived expectations; keyboard,
# accessibility writes, canceled values, revision counts and hashes stay strict.
DRAG_TOLERANCE_DB = 0.001


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(4 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def nodes_in_window(snapshot, frame_reference):
    """Use observed tree ancestry, never a matching name in another window."""
    if frame_reference is None:
        return snapshot["nodes"]
    reference = list(frame_reference)
    return [node for node in snapshot["nodes"] if node.get("windowReference") == reference]


def weston_surface_from_scene(scene, pid, title=None, evidence="weston-scene-graph.txt"):
    # Weston 13/14's actual mapped view and attached buffer establish geometry.
    # Weston 13.0.0 source: libweston/compositor.c::debug_scene_view_print,
    # debug_scene_buffer_print; shell-utils.c::weston_shell_utils_surface_get_label.
    # The process may own both the project and a separate native plugin window.
    lines, frames = scene.splitlines(), []
    for index, line in enumerate(lines):
        header = re.match(r"\s*View \d+ \(role xdg_toplevel, PID (\d+), surface ID (\d+), "
                          r"top-level window '([^']*)' of ([^,]+),", line)
        if not header or int(header[1]) != pid or header[4] != "verdant":
            continue
        if title is not None and header[3] != title:
            continue
        body = []
        for entry in lines[index + 1:]:
            if re.match(r"\s*(View \d+|Layer \d+)", entry):
                break
            body.append(entry)
        if any('[view is not mapped!]' in entry or '[surface is not mapped!]' in entry for entry in body):
            continue
        rectangle = re.search(r"position: \((-?\d+), (-?\d+)\) -> \((-?\d+), (-?\d+)\)", "\n".join(body))
        buffer = re.search(r"width: (\d+), height: (\d+)", "\n".join(body))
        if not rectangle or not buffer:
            continue
        x, y, right, bottom = map(int, rectangle.groups())
        width, height = right - x, bottom - y
        if width <= 0 or height <= 0 or [width, height] != list(map(int, buffer.groups())):
            raise RuntimeError("Windowed surface has an unsupported transform or buffer geometry")
        frames.append({"x": x, "y": y, "width": width, "height": height,
                       "pid": int(header[1]), "surfaceId": int(header[2]),
                       "title": header[3], "appId": header[4], "evidence": evidence})
    if len(frames) > 1:
        raise RuntimeError(f"Multiple native top-levels match the launched PID and title {title!r}")
    return frames[0] if frames else None


def wayland_contact_evidence(trace, dimensions):
    held, position = False, None
    evidence = {"escapeKeyEvents": [], "outsideMotions": [], "outsideReleases": []}

    def outside(point):
        return dimensions and point and not (0 <= point[0] < dimensions[0] and 0 <= point[1] < dimensions[1])

    for line in trace.splitlines():
        motion = re.search(r"wl_pointer(?:[@#]\d+)?\s*\.\s*motion\(\s*(\d+),\s*([-0-9.]+),\s*([-0-9.]+)\)", line)
        button = re.search(r"wl_pointer(?:[@#]\d+)?\s*\.\s*button\(\s*\d+,\s*(\d+),\s*272,\s*([01])\)", line)
        escape = re.search(r"wl_keyboard(?:[@#]\d+)?\s*\.\s*key\(\s*\d+,\s*(\d+),\s*1,\s*([01])\)", line)
        if motion:
            position = [float(motion[2]), float(motion[3])]
            if outside(position):
                evidence["outsideMotions"].append({"nativeTimeMs": int(motion[1]), "position": position,
                                                    "pointerHeld": held, "trace": line.strip()})
        elif button:
            held = button[2] == "1"
            if not held and outside(position):
                evidence["outsideReleases"].append({"nativeTimeMs": int(button[1]), "position": position,
                                                     "trace": line.strip()})
        elif escape:
            evidence["escapeKeyEvents"].append({"nativeTimeMs": int(escape[1]),
                                                "state": "down" if escape[2] == "1" else "up",
                                                "pointerHeld": held, "trace": line.strip()})
    return evidence


def fixture():
    # Construct the native document directly. No example document, historical
    # voice identity, provenance or imported-score source fields are copied.
    return {
        "format": "greendam-project", "version": "0.1",
        "project_id": str(uuid.uuid4()), "revision": 0, "ppq": 960,
        "tempo_map": [{"tick": 0, "micro_bpm": 120000000}],
        "time_signatures": [{"tick": 0, "numerator": 4, "denominator": 4}],
        "voice_refs": {},
        "tracks": [{"id": "native-probe-synth", "name": "Synthetic input probe",
                    "monophonic": True, "notes": [], "color": "teal",
                    "instrument": {"format": "builtin", "id": "verdant.builtin.synth",
                                   "name": "Verdant Synth", "preset": "",
                                   "params": {"osc1": 0.0, "osc2": 1.0, "osc2_semi": 0.0,
                                              "detune": 8.0, "osc2_mix": 40.0, "attack": 5.0,
                                              "decay": 300.0, "sustain": 70.0, "release": 250.0,
                                              "cutoff": 4000.0, "resonance": 0.9,
                                              "env_amount": 30.0, "width": 50.0, "gain": -8.0}}}],
        # Vowel-onset alignment is always on; without it the app migrates the
        # document on open, which commits an edit before the probe starts.
        "render_preferences": {"protocol": "ogd-worker/1", "route": "verdant-voice",
                               "native_seed": 10008,
                               "phoneme_timing": {"method": "vowel-anchor-v1"}},
        "extensions": {},
    }


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True, help="New, disposable evidence directory")
    parser.add_argument("--backend", choices=("x11", "wayland"), required=True)
    parser.add_argument("--suite", choices=("original", "controls"), default="original")
    parser.add_argument("--input-display", required=True, help="Explicit private X11/XTest display")
    parser.add_argument("--outer-window", help="Nested Weston's X11 host window ID")
    parser.add_argument("--wayland-windowed", action="store_true",
                        help="Use an unmaximized app with observable compositor margins")
    parser.add_argument("--weston-debug", type=Path,
                        help="Private Weston's debug client for measured windowed surface geometry")
    parser.add_argument("--worker-binary", type=Path)
    parser.add_argument("--xdotool", default="xdotool")
    parser.add_argument("--gain-name", default="音量", help="Builtin synth gain's authored accessible name")
    parser.add_argument("--surface-offset", nargs=2, type=float, metavar=("X", "Y"),
                        help="Known Wayland surface origin inside the outer client; otherwise sizes must match")
    parser.add_argument("--surface-scale", type=float, default=1.0)
    parser.add_argument("--timeout", type=float, default=25)
    parser.add_argument("--skip-set-value", action="store_true")
    parser.add_argument("--only", action="append", choices=("preflight", *CASES, *CONTROL_CASES),
                        help="Run only selected cases; preflight always runs")
    args = parser.parse_args()
    allowed = CASES if args.suite == "original" else CONTROL_CASES
    if args.only and any(name not in ("preflight", *allowed) for name in args.only):
        parser.error("Diagnostic cases must belong to the selected suite")
    if args.backend == "wayland" and not args.outer_window:
        parser.error("Wayland needs --outer-window for the actual XTest-to-Weston route")
    if args.wayland_windowed and (args.backend != "wayland" or not args.weston_debug):
        parser.error("--wayland-windowed requires Wayland and --weston-debug")
    if not 0 < args.timeout <= 60 or not math.isfinite(args.timeout):
        parser.error("--timeout must be finite and between 0 and 60 seconds")
    if not math.isfinite(args.surface_scale) or args.surface_scale <= 0:
        parser.error("--surface-scale must be finite and positive")
    if args.surface_offset and not all(math.isfinite(v) for v in args.surface_offset):
        parser.error("--surface-offset must be finite")
    return args


class Probe(ControlCases):
    def __init__(self, args):
        self.args = args
        self.binary = args.binary.resolve(strict=True)
        self.repo = args.repo.resolve(strict=True)
        self.worker = (args.worker_binary or self.binary.with_name("verdant-worker")).resolve(strict=True)
        self.output = args.output_dir.absolute()
        self.output.mkdir(parents=True, exist_ok=False)
        self.project = self.output / "project.verp"
        self.project.write_text(json.dumps(fixture(), ensure_ascii=False, indent=2) + "\n")
        self.state = self.output / "state"
        self.state.mkdir()
        for name in ("cache", "data", "worker", "steps"):
            (self.output / name).mkdir()
        self.input_env = dict(os.environ, DISPLAY=args.input_display)
        self.started = time.monotonic()
        self.held = False
        self.last_press = 0.0
        self.process = None
        self.stdout = self.stderr = None
        self.observer = None
        self.knob_reference = None
        self.frame = None
        self.active_frame_reference = None
        self.main_frame_reference = None
        self.main_window_title = None
        self.active_window_name = "main"
        self.scene_serial = 0
        self.current_case = None
        self.input_window = args.outer_window
        self.transform = None
        self.wayland_input_ready = False
        self.report = {
            "format": "verdant.native-input-observation.v1", "ok": False,
            "backend": args.backend,
            "suite": args.suite,
            "inputRoute": "XTest → X11 → Verdant" if args.backend == "x11"
                          else "XTest → nested Weston X11 host → Wayland → Verdant",
            "scope": "Real OS/protocol observation; no physical screen reader or hardware touch device",
            "binary": {"path": str(self.binary), "sha256": sha256(self.binary)},
            "worker": {"path": str(self.worker), "sha256": sha256(self.worker)},
            "project": str(self.project), "cases": [], "steps": [], "input": [], "windowMappings": [],
            "skipSetValue": args.skip_set_value,
            "tolerances": {"uiNormalized": 1e-7, "savedGainDb": 1e-6,
                           "pixelDragGainDb": DRAG_TOLERANCE_DB,
                           "pixelDragReason": "Measured native pixel conversion error 0.0001373291015625 dB"},
        }

    def log_input(self, kind, **details):
        self.report["input"].append({"elapsedSeconds": round(time.monotonic() - self.started, 6),
                                     "kind": kind, "pointerHeld": self.held, **details})

    def xdo(self, *arguments):
        result = subprocess.run([self.args.xdotool, *map(str, arguments)], env=self.input_env,
                                text=True, capture_output=True, timeout=8)
        self.log_input("xdotool", arguments=list(map(str, arguments)), exitCode=result.returncode,
                       stdout=result.stdout.strip(), stderr=result.stderr.strip())
        if result.returncode:
            raise RuntimeError(f"xdotool {arguments}: {result.stderr.strip()}")
        return result.stdout.strip()

    def pause(self, duration=0.12):
        if self.observer:
            self.observer.drain(duration)
        else:
            time.sleep(duration)

    def wait(self, operation, predicate, description):
        deadline = time.monotonic() + self.args.timeout
        last = None
        while True:
            if self.process and self.process.poll() is not None:
                raise RuntimeError(f"Verdant exited with {self.process.returncode} while waiting for {description}")
            last = operation()
            if predicate(last):
                return last
            if time.monotonic() >= deadline:
                raise RuntimeError(f"Timed out waiting for {description}; last={str(last)[:800]}")
            self.pause(0.08)

    def snapshot(self):
        snapshot = self.observer.snapshot(self.binary.name)
        frames = [node for node in snapshot["nodes"] if node["role"] in ("frame", "window")]
        if self.active_frame_reference:
            frames = [node for node in frames if tuple(node["reference"]) == self.active_frame_reference]
        if len(frames) == 1:
            self.frame = frames[0]
        elif not frames:
            self.frame = None
        else:
            raise RuntimeError("Multiple accessible windows require an explicit observed frame reference")
        return snapshot

    def nodes(self, name, role=None):
        return [node for node in nodes_in_window(self.snapshot(), self.active_frame_reference) if node["name"] == name
                and (role is None or node["role"] == role)]

    def node(self, name, role=None):
        found = self.wait(lambda: self.nodes(name, role), lambda nodes: bool(nodes), f"{name!r} in AT-SPI")
        if len(found) != 1:
            raise RuntimeError(f"Ambiguous accessible target {name!r}: {len(found)} matches")
        return found[0]

    def knob(self):
        if self.knob_reference:
            try:
                return self.observer.read(self.knob_reference)
            except self.protocol["dbus"].DBusException:
                self.knob_reference = None
        node = self.node(self.args.gain_name, "slider")
        self.knob_reference = tuple(node["reference"])
        return node

    def geometry(self, window):
        raw = self.xdo("getwindowgeometry", "--shell", window)
        values = dict(line.split("=", 1) for line in raw.splitlines() if "=" in line)
        return {key: int(values[key]) for key in ("X", "Y", "WIDTH", "HEIGHT")}

    def configured_wayland_size(self):
        # The pinned GPUI bridge can report an invalid root frame rectangle.
        # Verify the actual compositor configure rather than inventing a
        # surface location or treating window-local SCREEN bounds as global.
        log = (self.output / "app.stderr.log").read_text(errors="replace")
        sizes = [(int(w), int(h)) for w, h in re.findall(
            r"\bxdg_toplevel(?:[@#]\d+)?\s*\.\s*configure\(\s*(\d+)\s*,\s*(\d+)", log)
                 if int(w) > 0 and int(h) > 0]
        return sizes[-1] if sizes else None

    def wayland_surface_entered(self):
        log = (self.output / "app.stderr.log").read_text(errors="replace")
        return all(re.search(rf"\b{interface}(?:[@#]\d+)?\s*\.\s*enter\(", log)
                   for interface in ("wl_pointer", "wl_keyboard"))

    def weston_surface_geometry(self, title=None):
        command = [str(self.args.weston_debug), "scene-graph"]
        result = subprocess.run(command, text=True, capture_output=True, timeout=4)
        stem = f"weston-scene-graph-{self.scene_serial:02d}"
        self.scene_serial += 1
        (self.output / f"{stem}.txt").write_text(result.stdout)
        (self.output / f"{stem}.stderr.txt").write_text(result.stderr)
        if result.returncode:
            raise RuntimeError(f"Private Weston scene graph query failed: {result.stderr}")
        return weston_surface_from_scene(result.stdout, self.process.pid, title, f"{stem}.txt")

    def align(self, title=None):
        if self.args.backend == "x11":
            raw = self.xdo("search", "--onlyvisible", "--pid", self.process.pid)
            candidates = [(window, self.geometry(window), self.xdo("getwindowname", window)) for window in raw.splitlines()]
            if title is not None:
                candidates = [item for item in candidates if item[2] == title]
                if len(candidates) != 1:
                    raise RuntimeError(f"Expected one visible X11 top-level titled {title!r}, found {len(candidates)}")
            if not candidates:
                raise RuntimeError("No visible X11 window owned by the launched Verdant PID")
            self.input_window, geometry, actual_title = max(candidates, key=lambda pair: pair[1]["WIDTH"] * pair[1]["HEIGHT"])
            self.transform = {"scale": 1.0, "offset": [geometry["X"], geometry["Y"]], "inputWindow": self.input_window,
                              "inputWindowGeometry": geometry, "atspiFrame": self.frame, "title": actual_title}
            self.xdo("windowraise", self.input_window)
        else:
            geometry = self.geometry(self.input_window)
            if self.args.wayland_windowed:
                surface = self.wait(lambda: self.weston_surface_geometry(title), lambda frame: frame is not None,
                                    "the mapped Verdant surface in the private compositor scene graph")
                if self.args.surface_scale != 1.0 or self.args.surface_offset is not None:
                    raise RuntimeError("Measured Weston geometry requires its actual unit scale without a manual offset")
                if surface["x"] < 0 or surface["y"] < 0 or surface["x"] + surface["width"] > geometry["WIDTH"] \
                        or surface["y"] + surface["height"] > geometry["HEIGHT"]:
                    raise RuntimeError("The measured application surface does not fit in the private compositor output")
                width, height = surface["width"], surface["height"]
                offset = (surface["x"], surface["y"])
            else:
                surface = None
                width, height = self.wait(self.configured_wayland_size, lambda size: size is not None,
                                          "an actual positive xdg_toplevel.configure size in the client trace")
                offset = self.args.surface_offset
            scale = self.args.surface_scale
            if offset is None:
                if abs(width * scale - geometry["WIDTH"]) > 4 or abs(height * scale - geometry["HEIGHT"]) > 4:
                    raise RuntimeError(f"Wayland configure and outer window do not align: configure={[width, height]}, "
                                       f"outer={geometry}. Use kiosk/fullscreen or a measured --surface-offset.")
                offset = (0, 0)
            self.transform = {"scale": scale,
                              "offset": [geometry["X"] + offset[0], geometry["Y"] + offset[1]],
                              "configuredWaylandSize": self.configured_wayland_size(),
                              "surfaceOffset": list(offset), "inputWindow": self.input_window,
                              "inputWindowGeometry": geometry, "atspiFrame": self.frame,
                              "title": surface["title"] if surface is not None else title}
            if surface is not None:
                self.transform["sceneGraphSurface"] = surface
                self.transform["surfaceInputGeometry"] = {"X": geometry["X"] + surface["x"],
                                                          "Y": geometry["Y"] + surface["y"],
                                                          "WIDTH": width, "HEIGHT": height}
        self.transform["atspiCoordinateType"] = "WINDOW"
        self.transform["screenBoundsPolicy"] = "Recorded verbatim; never used as window origin evidence"
        self.report.setdefault("coordinateMapping", self.transform)
        self.report["windowMappings"].append({"name": self.active_window_name,
                                               "elapsedSeconds": round(time.monotonic() - self.started, 6),
                                               "mapping": self.transform})
        self.xdo("windowfocus", "--sync", self.input_window)
        self.pause(0.25)

    def point(self, node):
        x, y, width, height = node["windowExtents"]
        if width <= 0 or height <= 0:
            raise RuntimeError(f"Target has no visible bounds: {node['name']}")
        scale = self.transform["scale"]
        ox, oy = self.transform["offset"]
        return round((x + width / 2) * scale + ox), round((y + height / 2) * scale + oy)

    def move(self, x, y):
        self.xdo("mousemove", round(x), round(y))
        self.pause()

    def press(self, point):
        if self.held:
            raise RuntimeError("Probe attempted a second pointer press before release")
        self.pause(max(0, 0.65 - (time.monotonic() - self.last_press)))
        self.move(*point)
        if self.args.backend == "wayland" and not self.wayland_input_ready:
            # Accessibility can publish layout before the first software-GPU
            # buffer maps the Wayland surface. The compositor discards clicks
            # with no mapped target. Wait for actual incoming enter events,
            # not just a tree node or an outgoing wl_surface.commit request.
            self.wait(self.wayland_surface_entered, bool, "the compositor's pointer and keyboard enter events")
            self.wayland_input_ready = True
            self.log_input("wayland-readiness", evidence="incoming wl_pointer.enter and wl_keyboard.enter")
        self.held = True
        self.xdo("mousedown", 1)
        self.last_press = time.monotonic()
        self.pause()

    def release(self):
        if self.held:
            self.xdo("mouseup", 1)
            self.held = False
            self.pause(0.18)

    def click(self, node):
        self.press(self.point(node))
        self.release()

    def key(self, key):
        # xdotool --clearmodifiers also releases held pointer buttons. Real
        # Wayland traffic showed that release immediately BEFORE Escape,
        # committing a preview instead of testing cancellation. This private
        # session owns its key chords, so preserve held input for this route.
        modifiers = [] if self.held else ["--clearmodifiers"]
        self.xdo("key", *modifiers, "--delay", 60, key)
        self.pause(0.15)

    def focus(self, node):
        reference = tuple(node["reference"])
        accepted = bool(self.observer.interface(reference, self.protocol["COMPONENT"]).GrabFocus(timeout=3))
        self.log_input("atspi", method="Component.GrabFocus", reference=list(reference), accepted=accepted,
                       target=node["name"], windowReference=node.get("windowReference"))
        if not accepted:
            raise RuntimeError("AT-SPI focus request was rejected")
        self.wait(lambda: self.observer.read(reference), lambda n: "focused" in n["states"], "AT-SPI focused state")

    def advertised_action(self, node, name="click"):
        names = [item["name"] for item in node.get("actions", [])]
        if name not in names:
            raise RuntimeError(f"Actual {node['name']!r} target has no advertised {name!r} action: {names}")
        reference = tuple(node["reference"])
        accepted = bool(self.observer.interface(reference, self.protocol["ACTION"]).DoAction(
            self.protocol["dbus"].Int32(names.index(name)), timeout=3))
        self.log_input("atspi", method="Action.DoAction", name=name, target=node["name"],
                       reference=list(reference), windowReference=node.get("windowReference"), accepted=accepted)
        if not accepted:
            raise RuntimeError(f"Actual {node['name']!r} target rejected {name!r}")

    def ui_gain(self, gain, tolerance_db=None):
        normalized = (gain - GAIN_MIN) / (GAIN_MAX - GAIN_MIN)
        tolerance = 1e-7 if tolerance_db is None else tolerance_db / (GAIN_MAX - GAIN_MIN)
        def read():
            node = self.knob()
            if "value" not in node:
                raise RuntimeError(f"Rendered {self.args.gain_name!r} slider does not expose AT-SPI Value; "
                                   f"actual interfaces={node['interfaces']}")
            return node
        return self.wait(read, lambda node: abs(node["value"]["CurrentValue"] - normalized) <= tolerance,
                         f"normalized UI value {normalized}")

    def persisted(self):
        content = self.project.read_bytes()
        project = json.loads(content)
        return {"revision": project["revision"], "gainDb": project["tracks"][0]["instrument"]["params"]["gain"],
                "trackName": project["tracks"][0]["name"], "panPermille": project["tracks"][0].get("pan_permille", 0),
                "sha256": hashlib.sha256(content).hexdigest(), "projectId": project["project_id"],
                "mtimeNs": self.project.stat().st_mtime_ns}

    def save(self, gain, revision, tolerance_db=1e-6):
        before = self.project.stat()
        self.key("ctrl+s")
        self.wait(lambda: self.project.stat(),
                  lambda stat: (stat.st_ino, stat.st_mtime_ns) != (before.st_ino, before.st_mtime_ns),
                  "the actual Ctrl+S atomic file replacement")
        saved = self.persisted()
        if saved["revision"] != revision or abs(saved["gainDb"] - gain) > tolerance_db:
            raise RuntimeError(f"Persisted edit mismatch: expected gain={gain}, revision={revision}; got {saved}")
        return saved

    def undo(self, gain, revision):
        self.key("ctrl+z")
        self.ui_gain(gain)
        return self.save(gain, revision)

    def record(self, label, **extra):
        index = len(self.report["steps"])
        stem = f"{index:02d}-{label}"
        tree = self.snapshot()
        nodes = [n for n in nodes_in_window(tree, self.active_frame_reference)
                 if n["name"] == self.args.gain_name and n["role"] == "slider"]
        saved = self.persisted()
        shot = capture(self.args.input_display, self.output / "steps" / f"{stem}.png")
        detail = {"label": label, "case": self.current_case,
                  "elapsedSeconds": round(time.monotonic() - self.started, 6),
                  "pointerHeld": self.held, "savedProject": saved, "target": nodes,
                  "window": self.active_window_name, "windowReference": self.active_frame_reference,
                  "tree": tree, "screenshot": shot, "events": list(self.observer.events), **extra}
        (self.output / "steps" / f"{stem}.json").write_text(json.dumps(detail, ensure_ascii=False, indent=2) + "\n")
        shutil.copyfile(self.project, self.output / "steps" / f"{stem}.verp")
        self.report["steps"].append({"label": label, "case": self.current_case,
                                     "evidence": f"steps/{stem}.json",
                                     "screenshot": f"steps/{stem}.png", "savedProject": saved,
                                     "observedNormalizedValues": [n.get("value", {}).get("CurrentValue") for n in nodes],
                                     "pointerHeld": self.held, "window": self.active_window_name,
                                     "windowReference": self.active_frame_reference, **extra})
        self.write_report()

    def write_report(self):
        (self.output / "result.json").write_text(json.dumps(self.report, ensure_ascii=False, indent=2) + "\n")

    def start_drag(self, pixels=54):
        before = self.persisted()
        self.ui_gain(before["gainDb"])
        origin = self.point(self.knob())
        self.press(origin)
        for fraction in (1 / 3, 2 / 3, 1):
            self.move(origin[0], origin[1] - pixels * self.transform["scale"] * fraction)
        expected = min(GAIN_MAX, max(GAIN_MIN, before["gainDb"] + pixels / 180 * 30))
        self.ui_gain(expected, tolerance_db=DRAG_TOLERANCE_DB)
        if self.persisted() != before:
            raise RuntimeError("A pointer preview changed the saved project before release")
        return before, origin, expected

    def commit_and_undo(self, label, gain, base, tolerance_db=1e-6):
        self.save(gain, base["revision"] + 1, tolerance_db=tolerance_db)
        self.record(f"{label}-committed", expectedRevisionDelta=1,
                    expectedGainDb=gain, toleranceDb=tolerance_db)
        self.undo(base["gainDb"], base["revision"] + 2)
        self.record(f"{label}-single-undo", expectedUndoCount=1)

    def check_canceled(self, label, before):
        self.ui_gain(before["gainDb"])
        self.release()
        after = self.save(before["gainDb"], before["revision"])
        if after["sha256"] != before["sha256"]:
            raise RuntimeError(f"{label}: cancellation changed saved document bytes")
        self.record(f"{label}-canceled", expectedRevisionDelta=0)

    def keyboard(self):
        before = self.persisted()
        self.click(self.knob())
        self.wait(self.knob, lambda n: "focused" in n["states"], "mouse click focus")
        self.key("Up")
        self.ui_gain(before["gainDb"] + 0.3)
        self.commit_and_undo("keyboard", before["gainDb"] + 0.3, before)

    def drag(self, label="drag", pixels=54):
        before, origin, expected = self.start_drag(pixels)
        self.record(f"{label}-preview", logicalDisplacement=pixels, pointerOrigin=list(origin),
                    expectedGainDb=expected, toleranceDb=DRAG_TOLERANCE_DB)
        self.release()
        self.ui_gain(expected, tolerance_db=DRAG_TOLERANCE_DB)
        self.commit_and_undo(label, expected, before, tolerance_db=DRAG_TOLERANCE_DB)

    def escape(self):
        before, _, _ = self.start_drag()
        self.record("escape-preview")
        self.key("Escape")
        self.check_canceled("escape", before)

    def focus_handoff(self):
        before, _, _ = self.start_drag()
        self.record("focus-preview")
        # A second same-button mouse click cannot occur while that button is
        # physically held. Request real platform focus on another knob first,
        # then verify a real click there after the retired contact's late up.
        other = self.node("失谐", "slider")
        self.focus(other)
        self.check_canceled("platform-focus-handoff", before)
        self.click(self.node("失谐", "slider"))
        self.wait(lambda: self.node("失谐", "slider"), lambda n: "focused" in n["states"], "other knob click focus")
        self.save(before["gainDb"], before["revision"])
        self.record("other-knob-click-focus", cancellationRoute="Component.GrabFocus during held XTest drag")

    def outside_point(self):
        g = self.transform.get("surfaceInputGeometry", self.transform["inputWindowGeometry"])
        x, y, width, height = g["X"], g["Y"], g["WIDTH"], g["HEIGHT"]
        sw, sh = map(int, self.xdo("getdisplaygeometry").split())
        for point in ((x + width + 20, y + height / 2), (x - 20, y + height / 2),
                      (x + width / 2, y + height + 20), (x + width / 2, y - 20)):
            if not (0 <= point[0] < sw and 0 <= point[1] < sh):
                continue
            if self.args.backend == "wayland" and self.args.wayland_windowed:
                outer = self.transform["inputWindowGeometry"]
                if not (outer["X"] <= point[0] < outer["X"] + outer["WIDTH"]
                        and outer["Y"] <= point[1] < outer["Y"] + outer["HEIGHT"]):
                    continue  # Stay inside the compositor; output edges clamp.
            return point
        raise RuntimeError("Input display needs a margin outside the application/Weston window")

    def exit_window(self):
        before, origin, _ = self.start_drag()
        self.record("window-exit-preview")
        outside = self.outside_point()
        self.move(*outside)
        self.ui_gain(before["gainDb"])
        self.record("window-exited-with-button-held", outsideInputPosition=list(outside))
        self.release()
        self.move(*origin)
        self.xdo("windowfocus", "--sync", self.input_window)
        self.check_canceled("window-exit-outside-release", before)

    def detach(self):
        before, _, _ = self.start_drag()
        self.record("toolbar-detach-preview")
        button = self.node("钢琴卷帘 (E)")
        self.advertised_action(button)
        self.wait(lambda: self.nodes(self.args.gain_name, "slider"), lambda nodes: not nodes, "actual knob subtree detachment")
        self.knob_reference = None
        self.record("toolbar-detached", activationRoute="advertised AT-SPI toolbar click during held XTest drag")
        self.release()
        after = self.save(before["gainDb"], before["revision"])
        if after["sha256"] != before["sha256"]:
            raise RuntimeError("Toolbar detach changed persisted project content")
        self.click(self.node("效果器链"))
        self.ui_gain(before["gainDb"])
        self.record("toolbar-remounted", expectedRevisionDelta=0)

    def set_numeric(self, value):
        node = self.knob()
        reference = tuple(node["reference"])
        self.observer.interface(reference, self.protocol["PROPERTIES"]).Set(
            self.protocol["VALUE"], "CurrentValue", self.protocol["dbus"].Double(value, variant_level=1),
            signature="ssv", timeout=3)
        self.log_input("atspi", method="Properties.Set(Value.CurrentValue)", reference=list(reference),
                       value=value, signature="ssv", variantType="double")
        self.ui_gain(GAIN_MIN + (GAIN_MAX - GAIN_MIN) * value)

    def set_value(self):
        before = self.persisted()
        self.focus(self.knob())
        self.record("atspi-focus")
        self.set_numeric(0.63)
        self.commit_and_undo("atspi-value", -5.1, before)

    def set_value_during_drag(self):
        before, origin, _ = self.start_drag()
        self.record("atspi-takeover-preview")
        self.set_numeric(0.63)
        accepted = self.save(-5.1, before["revision"] + 1)
        self.record("atspi-takeover-committed-before-up", expectedRevisionDelta=1)
        self.move(origin[0], origin[1] - 66 * self.transform["scale"])
        self.release()
        self.ui_gain(-5.1)
        late = self.save(-5.1, before["revision"] + 1)
        if late["sha256"] != accepted["sha256"]:
            raise RuntimeError("Late pointer up changed the AT-SPI committed document")
        self.record("atspi-takeover-late-up", additionalRevisionDelta=0)
        self.undo(before["gainDb"], before["revision"] + 2)
        self.record("atspi-takeover-single-undo", expectedUndoCount=1)

    def native_roots(self):
        return [node for node in self.snapshot()["nodes"] if node["name"] == NATIVE_ROOT_NAME]

    def open_native_window(self):
        if self.active_frame_reference != self.main_frame_reference:
            raise RuntimeError("Native editor must be opened from the observed project window")
        if self.native_roots():
            raise RuntimeError("A native plugin editor was already open before the requested UI action")
        if self.args.backend == "wayland" and not self.args.wayland_windowed:
            raise RuntimeError("Native-window cases require measured windowed Wayland geometry")
        self.click(self.node("打开界面"))
        roots = self.wait(self.native_roots, bool, "the real native plugin editor subtree")
        if len(roots) != 1 or not roots[0].get("windowReference"):
            raise RuntimeError("Native plugin editor did not identify exactly one accessible window")
        reference = tuple(roots[0]["windowReference"])
        if reference == self.main_frame_reference:
            raise RuntimeError("Open editor action did not create a distinct native window")
        self.active_frame_reference = reference
        self.active_window_name = "native-plugin"
        self.knob_reference = None
        self.wait(self.snapshot, lambda _: self.frame is not None, "the native editor's frame")
        self.align(NATIVE_WINDOW_TITLE)
        self.ui_gain(self.persisted()["gainDb"])
        self.record("native-window-opened", openingRoute="XTest click on the instrument card's 打开界面 button")

    def close_native_window(self):
        if self.active_frame_reference in (None, self.main_frame_reference):
            raise RuntimeError("Close action requires the observed native plugin window")
        reference = self.active_frame_reference
        # The real synth editor also has radio choices named 关闭. Its window
        # close control is the distinct advertised push-button role.
        self.advertised_action(self.node("关闭", "push button"))
        self.wait(self.snapshot, lambda tree: all(tuple(node["reference"]) != reference
                  and all(tuple(child) != reference for child in node["children"])
                  for node in tree["nodes"]),
                  "the native plugin frame to leave the actual accessibility tree")
        self.knob_reference = None

    def return_to_main(self):
        if self.held:
            raise RuntimeError("Late pointer release must precede explicit main-window focus")
        self.active_frame_reference = self.main_frame_reference
        self.active_window_name = "main"
        self.knob_reference = None
        self.wait(self.snapshot, lambda _: self.frame is not None, "the original project frame")
        self.align(self.main_window_title)
        # NativePluginWindow has its own keyboard handlers. Saving must be
        # routed to the real main surface after the child closes, not merely
        # to the X11 host window of a nested Wayland compositor.
        self.click(self.knob())
        self.wait(self.knob, lambda node: "focused" in node["states"], "main-window click focus")

    def native_window_close(self):
        before = self.persisted()
        self.open_native_window()
        drag_base, _, _ = self.start_drag()
        if drag_base != before:
            raise RuntimeError("Opening the native editor changed the saved project")
        self.record("native-close-preview")
        self.close_native_window()
        self.record("native-closed-while-held", activationRoute="advertised AT-SPI native-window 关闭 click")
        self.release()
        self.return_to_main()
        after = self.save(before["gainDb"], before["revision"])
        if after["sha256"] != before["sha256"]:
            raise RuntimeError("Native window close or late up changed the saved document")
        self.record("native-close-canceled", expectedRevisionDelta=0)

        # Reopen through the same real button. A new native owner must accept
        # a subsequent drag, commit once and be undoable once in the main UI.
        self.open_native_window()
        reuse_base, origin, expected = self.start_drag(18)
        self.record("native-reopened-preview", logicalDisplacement=18, pointerOrigin=list(origin),
                    expectedGainDb=expected, toleranceDb=DRAG_TOLERANCE_DB)
        self.release()
        self.ui_gain(expected, tolerance_db=DRAG_TOLERANCE_DB)
        self.close_native_window()
        self.return_to_main()
        self.commit_and_undo("native-reopened", expected, reuse_base, tolerance_db=DRAG_TOLERANCE_DB)

    def native_preset_cancel(self):
        before = self.persisted()
        self.open_native_window()
        drag_base, _, _ = self.start_drag()
        if drag_base != before:
            raise RuntimeError("Opening the native editor changed the saved project")
        self.record("native-preset-preview")
        # The native synth toolbar owns 预设; 自定义 belongs to the main
        # instrument card. node() retains the observed native frame scope.
        self.advertised_action(self.node("预设", "combo box"))
        self.wait(lambda: self.nodes("上下文菜单", "menu"), bool, "the actual native preset menu")
        self.ui_gain(before["gainDb"])
        self.record("native-preset-menu-opened", activationRoute="advertised AT-SPI native 预设 combobox click")
        self.key("Escape")
        self.wait(lambda: self.nodes("上下文菜单", "menu"), lambda nodes: not nodes, "native preset menu dismissal")
        self.ui_gain(before["gainDb"])
        self.release()
        self.record("native-preset-menu-dismissed")
        self.close_native_window()
        self.return_to_main()
        after = self.save(before["gainDb"], before["revision"])
        if after["sha256"] != before["sha256"]:
            raise RuntimeError("Native preset menu cancellation or late up changed the saved document")
        self.record("native-preset-canceled", expectedRevisionDelta=0)

    def launch(self):
        if not os.environ.get("AT_SPI_BUS_ADDRESS") or not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
            raise RuntimeError("Launch the probe in the prepared private AT-SPI environment")
        sw, sh = map(int, self.xdo("getdisplaygeometry").split())
        width, height = min(1480, sw - 180), min(920, sh - 180)
        if width < 1000 or height < 700:
            raise RuntimeError("Use a larger private input display for the full device pane")
        (self.state / "prefs.json").write_text(json.dumps({
            "window": [80, 80, width, height],
            "maximized": self.args.backend == "wayland" and not self.args.wayland_windowed,
            "ui_locale": "zh-CN"}))
        env = dict(os.environ, VERDANT_STATE_DIR=str(self.state),
                   VERDANT_USER_DATA_DIR=str(self.output / "data"), VERDANT_CACHE_DIR=str(self.output / "cache"))
        if self.args.backend == "wayland":
            if not env.get("WAYLAND_DISPLAY"):
                raise RuntimeError("Wayland requires the prepared compositor's WAYLAND_DISPLAY")
            env.pop("DISPLAY", None)
            env["WAYLAND_DEBUG"] = "client"
        else:
            env["DISPLAY"] = self.args.input_display
            env.pop("WAYLAND_DISPLAY", None)
            env.pop("WAYLAND_SOCKET", None)
        command = [str(self.binary), str(self.project), "--silent-audio", "--repo", str(self.repo),
                   "--workspace", str(self.output / "worker"), "--worker-binary", str(self.worker)]
        self.stdout = (self.output / "app.stdout.log").open("w")
        self.stderr = (self.output / "app.stderr.log").open("w")
        self.process = subprocess.Popen(command, cwd=self.repo, env=env,
                                        stdout=self.stdout, stderr=self.stderr, start_new_session=True)
        self.report["process"] = {"pid": self.process.pid, "ownedProcessGroup": self.process.pid, "command": command}
        self.protocol = runpy.run_path(str(Path(__file__).resolve().parents[1] / "a11y/observe-atspi.py"))
        self.observer = self.protocol["Observer"]()
        self.wait(self.snapshot, lambda snapshot: bool(snapshot["applications"]) and self.frame is not None,
                  "the launched application on the actual AT-SPI bus")
        self.main_frame_reference = tuple(self.frame["reference"])
        self.active_frame_reference = self.main_frame_reference
        self.align()
        self.main_window_title = self.transform["title"]
        if not self.nodes(self.args.gain_name, "slider"):
            self.click(self.node("效果器链"))
        self.ui_gain(INITIAL_GAIN)
        self.focus(self.knob())
        self.save(INITIAL_GAIN, 0)
        self.record("preflight", expectedGainRange=[GAIN_MIN, GAIN_MAX])

    def cleanup(self):
        if self.held:
            try:
                self.release()
            except Exception as error:
                self.report.setdefault("cleanupErrors", []).append(str(error))
        if self.process:
            # This group was created by this probe; no name-based pkill or
            # compositor/bus/session termination is performed here.
            try:
                os.killpg(self.process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                self.process.wait(timeout=4)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=4)
            try:
                # A worker must not outlive an app that exits before its own
                # cleanup. This is still only the process group we created.
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            self.report["process"]["returnCodeAfterCleanup"] = self.process.returncode
        for stream in (self.stdout, self.stderr):
            if stream:
                stream.close()

    def run(self):
        try:
            self.launch()
            cases = CASES if self.args.suite == "original" else CONTROL_CASES
            selected = set(self.args.only or cases)
            operations = {"keyboard": self.keyboard, "drag": self.drag, "escape": self.escape,
                          "focus": self.focus_handoff, "exit": self.exit_window, "detach": self.detach,
                          "set-value": self.set_value, "set-value-during-drag": self.set_value_during_drag,
                          "lease-reusable": lambda: self.drag("lease-reusable", 18),
                          "native-window-close": self.native_window_close,
                          "native-preset-cancel": self.native_preset_cancel}
            if self.args.suite == "controls":
                operations = self.control_operations()
            for name in cases:
                if name not in selected:
                    continue
                if self.args.skip_set_value and name.startswith("set-value"):
                    self.report["cases"].append({"name": name, "status": "skipped", "reason": "--skip-set-value"})
                    continue
                case = {"name": name, "status": "running"}
                self.current_case = name
                self.report["cases"].append(case)
                self.write_report()
                operations[name]()
                case["status"] = "passed"
                self.write_report()
            self.report["ok"] = True
        except (Exception, KeyboardInterrupt) as error:
            self.report["error"] = str(error)
            self.report["traceback"] = traceback.format_exc()
            if self.report["cases"] and self.report["cases"][-1]["status"] == "running":
                self.report["cases"][-1]["status"] = "failed"
            # A broken accessibility query must not also prevent recording
            # what was visible on the real input display at failure.
            try:
                self.report["failureScreenshot"] = capture(self.args.input_display, self.output / "failure.png")
            except Exception as screenshot_error:
                self.report["failureScreenshotError"] = str(screenshot_error)
            if self.observer:
                try:
                    self.record("failure")
                except Exception as evidence_error:
                    self.report["failureEvidenceError"] = str(evidence_error)
        finally:
            self.cleanup()
            self.report["elapsedSeconds"] = round(time.monotonic() - self.started, 3)
            if (self.output / "app.stderr.log").exists():
                stderr = (self.output / "app.stderr.log").read_text(errors="replace")
                if self.args.backend == "wayland":
                    incoming = {
                        "wl_pointer": {"enter", "leave", "motion", "button", "axis", "frame", "axis_source",
                                       "axis_stop", "axis_discrete", "axis_value120", "axis_relative_direction"},
                        "wl_keyboard": {"keymap", "enter", "leave", "key", "modifiers", "repeat_info"},
                    }
                    counts = {name: {} for name in incoming}
                    for interface, event in re.findall(r"\b(wl_pointer|wl_keyboard)(?:[@#]\d+)?\s*\.\s*([a-z_0-9]+)\(", stderr):
                        if event in incoming[interface]:
                            counts[interface][event] = counts[interface].get(event, 0) + 1
                    self.report["waylandProtocol"] = {"debug": "client", "trace": "app.stderr.log",
                                                      "incomingEvents": counts}
                    mapping = self.report.get("coordinateMapping", {})
                    surface = mapping.get("sceneGraphSurface")
                    dimensions = [surface["width"], surface["height"]] if surface else mapping.get("configuredWaylandSize")
                    contact = wayland_contact_evidence(stderr, dimensions)
                    self.report["waylandProtocol"]["contactEvidence"] = contact
                    passed = {case["name"] for case in self.report["cases"] if case["status"] == "passed"}
                    escape_inputs = [entry for entry in self.report["input"] if entry["kind"] == "xdotool"
                                     and entry["arguments"][0] == "key" and "Escape" in entry["arguments"]]
                    expected_escape_contacts = [entry["pointerHeld"] for entry in escape_inputs for _ in range(2)]
                    self.report["waylandProtocol"]["expectedEscapeContacts"] = expected_escape_contacts
                    if passed.intersection({"escape", "native-preset-cancel"}) and (
                            [event["pointerHeld"] for event in contact["escapeKeyEvents"]] != expected_escape_contacts):
                        self.report["ok"] = False
                        self.report["waylandContactError"] = "Escape did not preserve the held native pointer contact"
                    if "exit" in passed and self.args.wayland_windowed and not contact["outsideMotions"]:
                        self.report["ok"] = False
                        self.report["waylandContactError"] = "Window-exit case had no measured outside-window pointer sample"
                    if not counts["wl_pointer"].get("button") or not counts["wl_keyboard"].get("key"):
                        self.report["ok"] = False
                        self.report["waylandEvidenceError"] = "No matching incoming wl_pointer.button and wl_keyboard.key trace"
                fatal = [line for line in stderr.splitlines() if "panicked at" in line
                         or ("[Proto-UI:" in line and "failed" in line)]
                if fatal:
                    self.report["ok"] = False
                    self.report["nativeErrors"] = fatal
            self.write_report()
        print(json.dumps({"ok": self.report["ok"], "result": str(self.output / "result.json"),
                          "cases": self.report["cases"], "error": self.report.get("error")}, ensure_ascii=False))
        return 0 if self.report["ok"] else 1


if __name__ == "__main__":
    def interrupted(signum, _frame):
        raise InterruptedError(f"Probe received signal {signum}")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        sys.exit(Probe(arguments()).run())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"native input probe failed: {error}", file=sys.stderr)
        sys.exit(1)

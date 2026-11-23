#!/usr/bin/env python3
"""Observe the real AT-SPI tree, values, focus and explicitly requested actions.

Use only with a private test project/session. Without --focus, --action or
--set-value this is read-only. This observes the platform protocol; it does not
run a physical screen reader or claim to assess speech/Braille usability.
"""

import argparse
from collections import deque
import json
import math
import os
from pathlib import Path
import sys
import time

import dbus
from dbus.mainloop.glib import DBusGMainLoop
import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib


ACCESSIBLE = "org.a11y.atspi.Accessible"
VALUE = "org.a11y.atspi.Value"
ACTION = "org.a11y.atspi.Action"
COMPONENT = "org.a11y.atspi.Component"
TEXT = "org.a11y.atspi.Text"
PROPERTIES = "org.freedesktop.DBus.Properties"
ROOT = ("org.a11y.atspi.Registry", "/org/a11y/atspi/accessible/root")


def plain(value):
    if isinstance(value, (dbus.Boolean, bool)):
        return bool(value)
    if isinstance(value, (dbus.String, dbus.ObjectPath, dbus.Signature, str)):
        return str(value)
    if isinstance(value, (dbus.Double, float)):
        return float(value)
    if isinstance(value, (dbus.Array, dbus.Struct, list, tuple)):
        return [plain(item) for item in value]
    if isinstance(value, (dbus.Dictionary, dict)):
        return {str(key): plain(item) for key, item in value.items()}
    return int(value)


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", default="verdant", help="Application-name substring")
    parser.add_argument("--name", help="Exact accessible name of one target")
    parser.add_argument("--role", default="slider", help="Target AT-SPI role name")
    windows = parser.add_mutually_exclusive_group()
    windows.add_argument("--window", help="Exact observed AT-SPI frame/window name")
    windows.add_argument("--window-reference", nargs=2, metavar=("BUS", "PATH"),
                         help="Exact frame reference from a previous snapshot of this live session")
    parser.add_argument("--wait", type=float, default=10, help="Discovery/action timeout seconds")
    parser.add_argument("--focus", action="store_true", help="Request Component.GrabFocus and require focused state")
    parser.add_argument("--require-action", action="append", default=[])
    parser.add_argument("--action", action="append", default=[], help="Invoke an advertised Action name")
    parser.add_argument("--set-value", type=float, action="append", default=[],
                        help="Invoke the real Value.CurrentValue setter")
    parser.add_argument("--expect-value", type=float, help="Required final numeric value")
    parser.add_argument("--tolerance", type=float, default=1e-6)
    parser.add_argument("--watch", type=float, default=0.25, help="Event drain after actions")
    parser.add_argument("--output", type=Path, help="JSON evidence path; stdout otherwise")
    args = parser.parse_args()
    if not math.isfinite(args.wait) or args.wait < 0 or args.wait > 60:
        parser.error("--wait must be between 0 and 60 seconds")
    for value in [*args.set_value, args.expect_value, args.tolerance, args.watch]:
        if value is not None and not math.isfinite(value):
            parser.error("Numeric options must be finite")
    if args.tolerance < 0 or not 0 <= args.watch <= 60:
        parser.error("--tolerance must be nonnegative and --watch between 0 and 60")
    if args.set_value and args.expect_value is None:
        parser.error("--set-value requires --expect-value to verify actual readback")
    return args


def canonical_role(role_number, platform_role):
    # AT-SPI role 43 is the same button across GLib versions whose enum nick
    # changed from "push-button" to "button". Keep the original nick as evidence.
    return "push button" if role_number == 43 else platform_role


class Observer:
    def __init__(self):
        address = os.environ.get("AT_SPI_BUS_ADDRESS")
        if not address:
            raise RuntimeError("Set the private AT_SPI_BUS_ADDRESS first")
        DBusGMainLoop(set_as_default=True)
        self.bus = dbus.bus.BusConnection(address)
        self.events = []
        self.started = time.monotonic()
        self.context = GLib.MainContext.default()
        for interface in ("org.a11y.atspi.Event.Object", "org.a11y.atspi.Event.Window"):
            self.bus.add_signal_receiver(self.event, dbus_interface=interface,
                                         interface_keyword="interface", member_keyword="member",
                                         path_keyword="path", sender_keyword="sender")

    def event(self, *args, **kwargs):
        self.events.append({"elapsedSeconds": round(time.monotonic() - self.started, 6),
                            **kwargs, "arguments": plain(args)})

    def drain(self, seconds=0):
        deadline = time.monotonic() + seconds
        while True:
            while self.context.pending():
                self.context.iteration(False)
            if time.monotonic() >= deadline:
                return
            time.sleep(min(0.02, max(0, deadline - time.monotonic())))

    def interface(self, reference, interface):
        return dbus.Interface(self.bus.get_object(*reference, introspect=False), interface)

    def set_text_selection(self, reference, selection, start, end):
        """Ask the real Text service to modify an existing selection."""
        if not all(isinstance(value, int) for value in (selection, start, end)) or min(selection, start, end) < 0:
            raise ValueError("Selection coordinates must be non-negative integers")
        return bool(self.interface(reference, TEXT).SetSelection(
            dbus.Int32(selection), dbus.Int32(start), dbus.Int32(end), timeout=3))

    def read_text(self, reference):
        """Capture two equal platform observations; retain each completed sample.

        Text, caret and selection use separate D-Bus calls. Real typing can
        advance between them. Only a changing tuple is retried; a stable
        illegal range remains an error and is never clamped.
        """
        samples = []
        deadline = time.monotonic() + 3
        def rpc(method, *args):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError("Platform Text capture deadline expired: " + repr(samples))
            value = method(*args, timeout=remaining)
            if time.monotonic() >= deadline:
                raise RuntimeError("Platform Text capture deadline expired: " + repr(samples))
            return value
        interfaces = plain(rpc(self.interface(reference, ACCESSIBLE).GetInterfaces))
        if TEXT not in interfaces:
            raise RuntimeError(f"Actual textbox does not advertise AT-SPI Text: {interfaces}")
        interface = self.interface(reference, TEXT)
        properties = self.interface(reference, PROPERTIES)
        while len(samples) < 8:
            text = str(rpc(interface.GetText, dbus.Int32(0), dbus.Int32(-1)))
            caret = int(rpc(properties.Get, TEXT, "CaretOffset"))
            count = int(rpc(interface.GetNSelections))
            if not 0 <= count <= 16:
                raise RuntimeError(f"Unexpected number of platform selections: {count}")
            selections = [plain(rpc(interface.GetSelection, dbus.Int32(index))) for index in range(count)]
            current = {"text": text, "caret": caret, "selections": selections}
            samples.append(current)
            if len(samples) > 1 and samples[-1] == samples[-2]:
                if time.monotonic() >= deadline:
                    raise RuntimeError("Platform Text capture deadline expired: " + repr(samples))
                if not 0 <= caret <= len(text) or any(
                        len(item) != 2 or not 0 <= item[0] <= item[1] <= len(text) for item in selections):
                    raise RuntimeError("Stable platform Text offsets are invalid: " + repr(samples))
                return {**current, "samples": samples}
        raise RuntimeError("Platform Text did not stabilize within the capture budget: " + repr(samples))

    def read(self, reference):
        accessible = self.interface(reference, ACCESSIBLE)
        properties = self.interface(reference, PROPERTIES)
        interfaces = plain(accessible.GetInterfaces(timeout=3))
        bits = plain(accessible.GetState(timeout=3))
        states = [enum.value_nick for number, enum in Atspi.StateType.__enum_values__.items()
                  if number // 32 < len(bits) and bits[number // 32] & (1 << (number % 32))]
        # The pinned AccessKit application root implements GetRole but not
        # GetRoleName. The standard numeric role is shared by both the real
        # registry and providers; use the installed AT-SPI enum vocabulary.
        role_number = int(accessible.GetRole(timeout=3))
        role = Atspi.Role.__enum_values__.get(role_number)
        platform_role = role.value_nick.replace("-", " ") if role else f"unknown role {role_number}"
        node = {"reference": list(reference),
                "name": str(properties.Get(ACCESSIBLE, "Name", timeout=3)),
                "role": canonical_role(role_number, platform_role), "platformRole": platform_role,
                "roleNumber": role_number,
                "interfaces": interfaces, "stateBits": bits, "states": states,
                "children": plain(accessible.GetChildren(timeout=3))}
        if VALUE in interfaces:
            node["value"] = plain(properties.GetAll(VALUE, timeout=3))
        if ACTION in interfaces:
            node["actions"] = [dict(zip(("name", "description", "keyBinding"), row))
                               for row in plain(self.interface(reference, ACTION).GetActions(timeout=3))]
        if COMPONENT in interfaces:
            component = self.interface(reference, COMPONENT)
            node["screenExtents"] = plain(component.GetExtents(
                dbus.UInt32(int(Atspi.CoordType.SCREEN)), timeout=3))
            node["windowExtents"] = plain(component.GetExtents(
                dbus.UInt32(int(Atspi.CoordType.WINDOW)), timeout=3))
        return node

    def snapshot(self, app_filter):
        desktop = self.read(ROOT)
        applications, errors = [], []
        for reference in desktop["children"]:
            try:
                app = self.read(tuple(reference))
            except dbus.DBusException as error:
                # AccessKit registers its application with Registry.Socket
                # before installing RootAccessibleInterface. A listed child
                # can therefore be temporarily incomplete during activation.
                # Keep that evidence and let discovery retry; other protocol
                # errors remain errors, and a persistent gap still times out.
                if error.get_dbus_name() not in ("org.freedesktop.DBus.Error.UnknownObject",
                                                  "org.freedesktop.DBus.Error.UnknownInterface"):
                    raise
                errors.append({"reference": list(reference), "stage": "application-registration",
                               "error": str(error)})
                continue
            if app_filter.casefold() in app["name"].casefold():
                applications.append(app)
        nodes, visited = [], set()
        queue = deque((app["reference"], None, None) for app in applications)
        while queue:
            reference, window_reference, window_name = queue.popleft()
            reference = tuple(reference)
            if reference in visited or not reference[0] or reference[1].endswith("/null"):
                continue
            visited.add(reference)
            if len(visited) > 4096:
                raise RuntimeError("AT-SPI tree exceeded the 4096-node observation limit")
            try:
                node = self.read(reference)
                if node["role"] in ("frame", "window"):
                    window_reference, window_name = node["reference"], node["name"]
                node["windowReference"] = window_reference
                node["windowName"] = window_name
                nodes.append(node)
                queue.extend((child, window_reference, window_name) for child in node["children"])
            except dbus.DBusException as error:
                errors.append({"reference": list(reference), "error": str(error)})
        return {"applications": applications, "nodes": nodes, "readErrors": errors}

    def wait_for(self, operation, predicate, timeout, description):
        deadline = time.monotonic() + timeout
        while True:
            value = operation()
            if predicate(value):
                return value
            if time.monotonic() >= deadline:
                raise RuntimeError(f"Timed out waiting for {description}")
            self.drain(0.05)


def observe(args, report):
    observer = Observer()
    report["events"] = observer.events

    def matches(snapshot):
        return [node for node in snapshot["nodes"]
                if (not args.role or node["role"].casefold() == args.role.casefold())
                and (args.name is None or node["name"] == args.name)
                and (args.window is None or node.get("windowName") == args.window)
                and (args.window_reference is None or node.get("windowReference") == args.window_reference)]

    def discover():
        snapshot = observer.snapshot(args.app)
        report["before"] = snapshot
        return snapshot

    snapshot = observer.wait_for(discover, lambda s: bool(matches(s)),
                                 args.wait, "the requested accessible control")
    report["before"] = snapshot
    targets = matches(snapshot)
    report["matches"] = targets
    requires_one = args.focus or args.action or args.set_value or args.expect_value is not None
    if requires_one and len(targets) != 1:
        raise RuntimeError(f"Expected one target for interaction, found {len(targets)}; use --name and a window selector")
    for node in targets:
        names = [item["name"] for item in node.get("actions", [])]
        for name in args.require_action:
            if name not in names:
                raise RuntimeError(f"{node['name']} does not advertise Action {name!r}; advertised {names}")
    if requires_one:
        target = tuple(targets[0]["reference"])
        operations = report.setdefault("operations", [])
        if args.focus:
            accepted = bool(observer.interface(target, COMPONENT).GrabFocus(timeout=3))
            after = observer.wait_for(lambda: observer.read(target),
                                      lambda node: "focused" in node["states"], args.wait, "focused state")
            operations.append({"method": "Component.GrabFocus", "accepted": accepted, "after": after})
            if not accepted:
                raise RuntimeError("Component.GrabFocus returned false")
        for name in args.action:
            before = observer.read(target)
            names = [item["name"] for item in before.get("actions", [])]
            if name not in names:
                raise RuntimeError(f"Target does not advertise Action {name!r}; advertised {names}")
            accepted = bool(observer.interface(target, ACTION).DoAction(dbus.Int32(names.index(name)), timeout=3))
            observer.drain(args.watch)
            operations.append({"method": "Action.DoAction", "name": name,
                               "accepted": accepted, "before": before, "after": observer.read(target)})
            if not accepted:
                raise RuntimeError(f"Action {name!r} returned false")
        for value in args.set_value:
            before = observer.read(target)
            observer.interface(target, PROPERTIES).Set(VALUE, "CurrentValue",
                                                      dbus.Double(value, variant_level=1),
                                                      signature="ssv", timeout=3)
            observer.drain(args.watch)
            operations.append({"method": "Properties.Set(Value.CurrentValue)", "requested": value,
                               "before": before, "after": observer.read(target)})
        if args.expect_value is not None:
            observer.wait_for(lambda: observer.read(target),
                              lambda node: abs(node["value"]["CurrentValue"] - args.expect_value) <= args.tolerance,
                              args.wait, f"numeric value {args.expect_value}")
            report["expectedValue"] = args.expect_value
            report["tolerance"] = args.tolerance
    observer.drain(args.watch)
    report["after"] = observer.snapshot(args.app)
    if args.expect_value is not None:
        final = matches(report["after"])
        if len(final) != 1 or abs(final[0]["value"]["CurrentValue"] - args.expect_value) > args.tolerance:
            raise RuntimeError("Final tree did not retain the expected numeric value")
    report["ok"] = True


if __name__ == "__main__":
    args = arguments()
    report = {"format": "verdant.atspi-observation.v1", "ok": False,
              "scope": "AT-SPI protocol observation; no physical screen-reader validation",
              "selector": {"app": args.app, "name": args.name, "role": args.role,
                           "window": args.window, "windowReference": args.window_reference}}
    try:
        observe(args, report)
    except (dbus.DBusException, RuntimeError, KeyError) as error:
        report["error"] = str(error)
    output = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(output)
        print(json.dumps({"ok": report["ok"], "output": str(args.output),
                          "error": report.get("error")}, ensure_ascii=False))
    else:
        print(output, end="")
    sys.exit(0 if report["ok"] else 1)

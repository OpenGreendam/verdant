#!/usr/bin/env python3
"""Run a command with real, private session/AT-SPI buses and registry.

The Debian AT-SPI launcher has compiled-in /usr paths for its bus config.
Use it only for org.a11y.Status, and start the real accessibility bus ourselves.
AccessKit and libatspi both honor AT_SPI_BUS_ADDRESS. No system services or
GSettings are changed. The command must remain alive for as long as its app.
"""

import argparse
import json
import os
from pathlib import Path
import selectors
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from xml.sax.saxutils import escape


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deps-root", required=True, type=Path,
                        help="Extracted dependency prefix, or / for installed system packages")
    parser.add_argument("--run-dir", type=Path,
                        help="New evidence/socket directory; must not exist")
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command.pop(0)
    if not args.command:
        parser.error("Provide a command after --")
    return args


def bus_config(path):
    return f"""<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <listen>unix:path={escape(str(path))}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow user="{os.getuid()}"/>
    <allow own="*"/>
    <allow send_destination="*" eavesdrop="true"/>
    <allow receive_sender="*" eavesdrop="true"/>
  </policy>
</busconfig>
"""


def prepare(args):
    deps = args.deps_root.resolve(strict=True)
    for executable in ("at-spi-bus-launcher", "at-spi2-registryd"):
        if not (deps / "usr/libexec" / executable).is_file():
            raise RuntimeError(f"Missing AT-SPI dependency {executable}")
    if args.run_dir:
        run = args.run_dir.absolute()
        run.mkdir(parents=True, mode=0o700, exist_ok=False)
    else:
        run = Path(tempfile.mkdtemp(prefix="a11y-", dir=deps.parent))
    run = run.resolve(strict=True)
    os.chmod(run, 0o700)
    if len(os.fsencode(run / "accessibility.sock")) >= 104:
        raise RuntimeError("Choose a shorter --run-dir for Unix socket paths")
    for name in ("runtime", "config", "cache", "data"):
        (run / name).mkdir(mode=0o700)
    for name in ("session", "accessibility"):
        (run / f"{name}.conf").write_text(bus_config(run / f"{name}.sock"))
    env = os.environ.copy()
    # A compositor can live outside this private XDG runtime directory. The
    # Wayland client accepts an absolute socket path, so preserve that endpoint
    # before replacing XDG_RUNTIME_DIR for our buses and test application.
    wayland = env.get("WAYLAND_DISPLAY")
    if wayland and not Path(wayland).is_absolute():
        if not env.get("XDG_RUNTIME_DIR"):
            raise RuntimeError("Relative WAYLAND_DISPLAY requires XDG_RUNTIME_DIR")
        env["WAYLAND_DISPLAY"] = str(Path(env["XDG_RUNTIME_DIR"]) / wayland)
    library_dirs = sorted(deps.glob("usr/lib/*-linux-gnu"))
    for variable, paths in (
        ("PYTHONPATH", [deps / "usr/lib/python3/dist-packages"]),
        ("LD_LIBRARY_PATH", library_dirs),
        ("GI_TYPELIB_PATH", [p / "girepository-1.0" for p in library_dirs]),
    ):
        additions = [str(p) for p in paths if p.exists()]
        if env.get(variable):
            additions.append(env[variable])
        env[variable] = os.pathsep.join(additions)
    env.update({
        "XDG_RUNTIME_DIR": str(run / "runtime"),
        "XDG_CONFIG_HOME": str(run / "config"),
        "XDG_CACHE_HOME": str(run / "cache"),
        "XDG_DATA_HOME": str(run / "data"),
        "XDG_DATA_DIRS": str(deps / "usr/share"),
        "GSETTINGS_BACKEND": "memory",
        "GSETTINGS_SCHEMA_DIR": str(deps / "usr/share/glib-2.0/schemas"),
    })
    # Debian puts this tool in a multiarch library directory; a /usr/bin
    # symlink from an extracted package can have a missing local target.
    schema_compiler = next((str(p) for p in (
        deps / "usr/bin/glib-compile-schemas",
        *deps.glob("usr/lib/*-linux-gnu/glib-2.0/glib-compile-schemas"),
        *Path("/usr/lib").glob("*-linux-gnu/glib-2.0/glib-compile-schemas"),
    ) if p.is_file()), shutil.which("glib-compile-schemas"))
    schemas = Path(env["GSETTINGS_SCHEMA_DIR"])
    if not (schemas / "gschemas.compiled").is_file():
        if deps == Path('/'):
            raise RuntimeError("System GSettings schemas are missing; install the dependency packages before this run")
        if not schema_compiler:
            raise RuntimeError("glib-compile-schemas is needed for the extracted schemas")
        subprocess.run([schema_compiler, str(schemas)], env=env, check=True)
    command = ["dbus-run-session", f"--config-file={run / 'session.conf'}", "--",
               sys.executable, str(Path(__file__).resolve()), "--inside",
               "--deps-root", str(deps), "--run-dir", str(run), "--", *args.command]
    print(json.dumps({"runDirectory": str(run), "command": args.command}), flush=True)
    return subprocess.call(command, env=env)


def inside(args):
    import dbus

    deps, run = args.deps_root, args.run_dir
    processes = []
    logs = []
    env = os.environ.copy()

    def start(name, command, stdout=None):
        log = (run / f"{name}.log").open("w")
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdout=stdout or log,
                                   stderr=log, text=True)
        processes.append(process)
        return process

    try:
        daemon = start("accessibility-bus", ["dbus-daemon", "--nofork",
                       f"--config-file={run / 'accessibility.conf'}", "--print-address=1"],
                       stdout=subprocess.PIPE)
        selector = selectors.DefaultSelector()
        selector.register(daemon.stdout, selectors.EVENT_READ)
        if not selector.select(timeout=5):
            raise RuntimeError("Private accessibility bus did not return an address")
        env["AT_SPI_BUS_ADDRESS"] = daemon.stdout.readline().strip()
        selector.close()
        if not env["AT_SPI_BUS_ADDRESS"].startswith("unix:"):
            raise RuntimeError("Private accessibility bus failed; see accessibility-bus.log")
        # Never request GetAddress from this launcher: its upstream config
        # paths are /usr-relative. Every child gets our actual private address.
        start("a11y-status", [str(deps / "usr/libexec/at-spi-bus-launcher"),
                              "--a11y=1", "--screen-reader=0"])
        session = dbus.SessionBus()
        bus = dbus.bus.BusConnection(env["AT_SPI_BUS_ADDRESS"])
        deadline = time.monotonic() + 8
        while not session.name_has_owner("org.a11y.Bus"):
            if time.monotonic() > deadline:
                raise RuntimeError("org.a11y.Status did not start; see a11y-status.log")
            time.sleep(0.05)
        status = dbus.Interface(session.get_object("org.a11y.Bus", "/org/a11y/bus"),
                                "org.freedesktop.DBus.Properties")
        if not status.Get("org.a11y.Status", "IsEnabled"):
            raise RuntimeError("Private AT-SPI status was not enabled")
        start("registry", [str(deps / "usr/libexec/at-spi2-registryd")])
        while not bus.name_has_owner("org.a11y.atspi.Registry"):
            if time.monotonic() > deadline:
                raise RuntimeError("AT-SPI registry did not start; see registry.log")
            time.sleep(0.05)
        exported_names = ("DBUS_SESSION_BUS_ADDRESS", "AT_SPI_BUS_ADDRESS", "DISPLAY",
                          "XAUTHORITY", "WAYLAND_DISPLAY", "PYTHONPATH", "GI_TYPELIB_PATH",
                          "LD_LIBRARY_PATH", "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME",
                          "XDG_CACHE_HOME", "XDG_DATA_HOME", "XDG_DATA_DIRS",
                          "GSETTINGS_BACKEND", "GSETTINGS_SCHEMA_DIR")
        exported = {key: env[key] for key in exported_names if key in env}
        (run / "environment.json").write_text(json.dumps(exported, indent=2) + "\n")
        (run / "environment.sh").write_text("".join(
            f"export {key}={shlex.quote(value)}\n" for key, value in exported.items()))
        print(json.dumps({"ready": True, "runDirectory": str(run),
                          "statusEnabled": True, "screenReaderEnabled": False}), flush=True)
        return subprocess.call(args.command, env=env)
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        for log in logs:
            log.close()


if __name__ == "__main__":
    try:
        args = arguments()
        sys.exit(inside(args) if args.inside else prepare(args))
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"private AT-SPI session failed: {error}", file=sys.stderr)
        sys.exit(1)

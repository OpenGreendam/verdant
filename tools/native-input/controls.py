"""Production-control journeys driven through the OS and the AT-SPI tree.

These cases run in a separate application/project from the original knob
suite. Unicode key injection is deliberately not described as an IME test.
"""

import copy
import math


CONTROL_CASES = ("text-selection-focus", "collection-scroll-cancel", "mixer-pan")
TEXT_INITIAL = "音😀AB"
TEXT_ASSISTIVE = "音😀乐"
TEXT_ACCEPTED = "音😺乐"
LIBRARY_TITLE = "插件预设库 — Verdant"
SEARCH_NAME = "搜索名称、类别、作者、标签与用途"
SEARCH_MISSING = "verdant-no-matching-preset-79f2"


def intersects(first, second):
    if len(first) != 4 or len(second) != 4 or not all(math.isfinite(v) for v in [*first, *second]):
        raise RuntimeError("Invalid observed control bounds")
    x, y, width, height = first
    ox, oy, ow, oh = second
    return width > 0 and height > 0 and ow > 0 and oh > 0 \
        and min(x + width, ox + ow) > max(x, ox) \
        and min(y + height, oy + oh) > max(y, oy)


def pan_after_displacement(initial, pixels, width):
    """Public mixer contract: -100..100 percent, stored in integer permille."""
    if not all(math.isfinite(value) for value in (initial, pixels, width)) or width <= 16:
        raise RuntimeError("Invalid measured pan geometry")
    value = min(100.0, max(-100.0, initial + pixels / (width - 16.0) * 200.0))
    # Rust f64::round uses half away from zero; this is the domain's permille
    # precision, not a relaxed floating-point comparison of a saved value.
    return math.copysign(math.floor(abs(value) * 10.0 + 0.5), value) / 10.0


class ControlCases:
    def control_operations(self):
        return {"text-selection-focus": self.text_selection_focus,
                "collection-scroll-cancel": self.collection_scroll_cancel,
                "mixer-pan": self.mixer_pan}

    def text(self, node, expected=None, selections=None):
        reference = tuple(node["reference"])
        result = self.wait(lambda: self.observer.read_text(reference),
                           lambda value: (expected is None or value["text"] == expected)
                           and (selections is None or value["selections"] == selections),
                           f"real AT-SPI text={expected!r}, selections={selections!r}")
        self.log_input("atspi-observation", method="Text.GetText/GetSelection", reference=list(reference),
                       value=result)
        return {"reference": list(reference), **result}

    def type_unicode(self, value):
        if self.held:
            raise RuntimeError("Text journey must not release an unrelated held pointer")
        self.xdo("type", "--clearmodifiers", "--delay", 45, "--", value)
        self.log_input("text-route", route="XTest key events; no input-method composition", value=value)
        self.pause()

    def save_controls(self, name, pan, revision, before=None):
        saved = self.save(-8.0, revision)
        if saved["trackName"] != name or saved["panPermille"] != pan:
            raise RuntimeError(f"Saved control result differs: name={name!r}, pan={pan}; got {saved}")
        if before is not None and saved["sha256"] != before["sha256"]:
            raise RuntimeError("Canceled/navigation-only control changed complete saved project bytes")
        return saved

    def rename_prompt(self):
        # The inspector button is a real reachable production entry. The
        # devices knob's focus is outside the workspace-only chord context.
        self.click(self.node("重命名", "toggle button"))
        self.node("轨道名称", "dialog")
        field = self.node("输入文字…", "entry")
        self.wait(lambda: self.observer.read(tuple(field["reference"])),
                  lambda node: "focused" in node["states"], "rename textbox auto focus")
        return field

    def text_selection_focus(self):
        before = self.persisted()
        field = self.rename_prompt()
        self.type_unicode(TEXT_INITIAL)
        self.record("text-unicode", control={"text": self.text(field, TEXT_INITIAL), "imeComposition": False})
        # Select the astral character through physical navigation. AT-SPI
        # offsets are Unicode characters; the native editor internally uses
        # UTF-16. This crosses the surrogate-pair boundary without supplying
        # a private editor selection or an application-owned text buffer.
        self.key("Home")
        self.key("Right")
        self.key("shift+Right")
        self.record("text-astral-selection", control={"text": self.text(field, TEXT_INITIAL, [[1, 2]])})
        # Change a different range through the real platform action. Typing
        # below must replace AB, proving the bridge changed this same editor.
        reference = tuple(field["reference"])
        accepted = self.observer.set_text_selection(reference, 0, 2, 4)
        self.log_input("atspi", method="Text.SetSelection", reference=list(reference),
                       arguments=[0, 2, 4], accepted=accepted)
        if not accepted:
            raise RuntimeError("AT-SPI rejected the real text selection action")
        self.record("text-assistive-selection", control={"text": self.text(field, TEXT_INITIAL, [[2, 4]]),
                    "action": {"method": "Text.SetSelection", "arguments": [0, 2, 4], "accepted": accepted}})
        self.type_unicode("乐")
        self.record("text-assistive-edited", control={"text": self.text(field, TEXT_ASSISTIVE)})
        self.key("Home")
        self.key("Right")
        self.key("shift+Right")
        self.text(field, TEXT_ASSISTIVE, [[1, 2]])
        self.type_unicode("😺")
        self.record("text-edited", control={"text": self.text(field, TEXT_ACCEPTED)})
        self.key("Tab")
        cancel = self.node("取消", "button")
        cancel = self.wait(lambda: self.observer.read(tuple(cancel["reference"])),
                           lambda node: "focused" in node["states"], "one Tab to the dialog cancel button")
        self.record("text-focus-forward", control={"focused": cancel})
        self.key("shift+Tab")
        current = self.wait(lambda: self.observer.read(tuple(field["reference"])),
                            lambda node: "focused" in node["states"], "Shift+Tab returns to the actual textbox")
        self.record("text-focus-back", control={"focused": current, "text": self.text(field, TEXT_ACCEPTED)})
        self.key("Return")
        self.wait(lambda: self.nodes("轨道名称", "dialog"), lambda nodes: not nodes, "rename prompt submission")
        self.save_controls(TEXT_ACCEPTED, before["panPermille"], before["revision"] + 1)
        self.record("text-committed", control={"acceptedName": TEXT_ACCEPTED})
        self.key("ctrl+z")
        undone = self.save_controls(before["trackName"], before["panPermille"], before["revision"] + 2)
        self.record("text-single-undo", control={"undoCount": 1})
        field = self.rename_prompt()
        self.type_unicode("不应保存")
        self.text(field, "不应保存")
        self.key("Escape")
        self.wait(lambda: self.nodes("轨道名称", "dialog"), lambda nodes: not nodes, "rename prompt cancellation")
        self.save_controls(before["trackName"], before["panPermille"], undone["revision"], undone)
        self.record("text-canceled", control={"expectedRevisionDelta": 0})

    def library_options(self):
        from probe import nodes_in_window
        return [node for node in nodes_in_window(self.snapshot(), self.active_frame_reference)
                if node["role"] == "list item"]

    def collection_observation(self):
        options = self.library_options()
        viewport = self.node("预设列表")["windowExtents"]
        return {"viewport": viewport,
                "options": [{"reference": node["reference"], "name": node["name"],
                             "selected": "selected" in node["states"], "bounds": node["windowExtents"],
                             "visible": intersects(node["windowExtents"], viewport)} for node in options]}

    def selected_option(self, name=None):
        def selected(observation):
            chosen = [option for option in observation["options"] if option["selected"]]
            return len(chosen) == 1 and chosen[0]["visible"] and (name is None or chosen[0]["name"] == name)
        return self.wait(self.collection_observation, selected, f"visible, uniquely selected preset {name!r}")

    def collection_scroll_cancel(self):
        before = self.persisted()
        self.click(self.node("预设库", "button"))
        def frames():
            return [node for node in self.snapshot()["nodes"]
                    if node["role"] in ("frame", "window") and node["name"] == LIBRARY_TITLE]
        library = self.wait(frames, bool, "the production preset library window")
        if len(library) != 1 or tuple(library[0]["reference"]) == self.main_frame_reference:
            raise RuntimeError("Preset library did not identify one distinct native window")
        self.active_frame_reference = tuple(library[0]["reference"])
        self.active_window_name = "preset-library"
        self.wait(self.snapshot, lambda _: self.frame is not None, "the preset library frame")
        self.align(LIBRARY_TITLE)
        observation = self.wait(self.collection_observation, lambda item: len(item["options"]) >= 2,
                                "the real factory collection with multiple selectable rows")
        options = observation["options"]
        if len({item["name"] for item in options}) != len(options):
            raise RuntimeError("Preset journey needs unambiguous observed option names")
        if all(item["visible"] for item in options):
            raise RuntimeError("Factory collection must visibly overflow its real viewport")
        first, last = options[0]["name"], options[-1]["name"]
        self.record("collection-open", control=observation)
        self.click(self.node(first, "list item"))
        self.key("End")
        self.record("collection-end", control=self.selected_option(last))
        self.key("Prior")
        self.record("collection-page-up", control=self.selected_option(options[max(0, len(options) - 21)]["name"]))
        self.key("Home")
        self.record("collection-home", control=self.selected_option(first))
        initial_visible = {row["name"] for row in self.collection_observation()["options"] if row["visible"]}
        self.move(*self.point(self.node("预设列表")))
        self.xdo("click", "--repeat", 8, "--delay", 60, 5)
        scrolled = self.wait(self.collection_observation,
                             lambda item: {row["name"] for row in item["options"] if row["visible"]} != initial_visible,
                             "real wheel input changes the visible preset rows")
        if [item["name"] for item in scrolled["options"] if item["selected"]] != [first]:
            raise RuntimeError("Physical scrolling changed the selected preset")
        self.record("collection-wheel", control=scrolled)
        search = self.node(SEARCH_NAME, "entry")
        self.click(search)
        self.key("ctrl+a")
        self.type_unicode(SEARCH_MISSING)
        self.text(search, SEARCH_MISSING)
        self.wait(self.library_options, lambda items: not items, "search removes every unmatched real row")
        self.record("collection-search-empty", control={"text": self.text(search), "options": self.library_options()})
        self.key("ctrl+a")
        self.key("BackSpace")
        self.text(search, "")
        self.wait(self.library_options, lambda items: len(items) == len(options), "clearing search restores the collection")
        self.key("Down")
        observation = self.selected_option(options[1]["name"])
        focused = self.observer.read(tuple(search["reference"]))
        if "focused" not in focused["states"]:
            raise RuntimeError("Collection navigation stole the search textbox's focus")
        self.record("collection-search-navigation", control={**observation, "focused": focused, "text": self.text(search, "")})
        reference = self.active_frame_reference
        self.key("Escape")
        self.wait(self.snapshot, lambda tree: all(tuple(node["reference"]) != reference for node in tree["nodes"]),
                  "Escape closes the production collection window")
        self.return_to_main()
        self.save_controls(before["trackName"], before["panPermille"], before["revision"], before)
        self.record("collection-canceled", control={"expectedRevisionDelta": 0})

    def pan(self, expected=None):
        def read():
            node = self.node("轨道声像", "slider")
            if "value" not in node:
                raise RuntimeError("The real mixer pan lacks AT-SPI Value")
            return node
        return self.wait(read, lambda node: expected is None or abs(node["value"]["CurrentValue"] - expected) < 1e-7,
                         f"mixer pan value {expected}")

    def mixer_pan(self):
        before = self.persisted()
        self.click(self.node("混音器 (X)"))
        node = self.pan(before["panPermille"] / 10.0)
        width = node["windowExtents"][2]
        distance = min(22, math.floor(width / 4))
        if distance < 8:
            raise RuntimeError("Observed mixer pan is too small for the native drag journey")
        origin = self.point(node)
        self.press(origin)
        self.move(origin[0] + (distance - 4) * self.transform["scale"], origin[1])
        preview = pan_after_displacement(before["panPermille"] / 10.0, distance - 4, width)
        preview_node = self.pan(preview)
        if self.persisted() != before:
            raise RuntimeError("Mixer preview wrote the saved project before release")
        self.record("mixer-preview", control={"pan": preview_node, "displacement": distance - 4, "width": width})
        # The terminal motion/up burst differs from the last observed preview.
        # Both are actual OS events; do not fake a coordinate on a host callback.
        self.xdo("mousemove", round(origin[0] + distance * self.transform["scale"]), origin[1], "mouseup", 1)
        self.held = False
        expected = pan_after_displacement(before["panPermille"] / 10.0, distance, width)
        final_node = self.pan(expected)
        self.save_controls(before["trackName"], round(expected * 10), before["revision"] + 1)
        self.record("mixer-committed", control={"pan": final_node, "displacement": distance, "width": width})
        self.key("ctrl+z")
        self.pan(before["panPermille"] / 10.0)
        undone = self.save_controls(before["trackName"], before["panPermille"], before["revision"] + 2)
        self.record("mixer-single-undo", control={"undoCount": 1})
        node = self.pan(before["panPermille"] / 10.0)
        origin = self.point(node)
        self.press(origin)
        self.move(origin[0] + distance * self.transform["scale"], origin[1])
        self.pan(expected)
        self.key("Escape")
        restored = self.pan(before["panPermille"] / 10.0)
        self.record("mixer-canceled-while-held", control={"pan": restored})
        self.release()
        self.save_controls(before["trackName"], before["panPermille"], undone["revision"], undone)
        self.record("mixer-canceled", control={"expectedRevisionDelta": 0})


def validate_control_trace(observations, final):
    """Recheck domain and platform observations from retained step files.

    This receives actual decoded .verp snapshots, not the probe's summaries.
    A passed case flag, screenshot, or the requested input value is insufficient.
    """
    steps = {}
    for detail, document in observations:
        label = detail["label"]
        if label in steps:
            raise RuntimeError(f"Duplicate control observation: {label}")
        steps[label] = (detail, document)
    required = {"preflight", "text-unicode", "text-astral-selection", "text-assistive-selection",
                "text-assistive-edited", "text-edited",
                "text-focus-forward", "text-focus-back", "text-committed", "text-single-undo", "text-canceled",
                "collection-open", "collection-end", "collection-page-up", "collection-home", "collection-wheel",
                "collection-search-empty", "collection-search-navigation", "collection-canceled",
                "mixer-preview", "mixer-committed", "mixer-single-undo", "mixer-canceled-while-held", "mixer-canceled"}
    if set(steps) != required:
        raise RuntimeError(f"Incomplete control trace: missing={sorted(required - set(steps))}, extra={sorted(set(steps) - required)}")
    if {detail.get("case") for detail, _ in observations} != {None, *CONTROL_CASES}:
        raise RuntimeError("Control cases lack their own recorded observations")

    def observed(label):
        return steps[label][0].get("control", {})

    def document(label):
        return steps[label][1]

    def require(condition, message):
        if not condition:
            raise RuntimeError(message)

    baseline = document("preflight")
    require(baseline["revision"] == 0 and len(baseline["tracks"]) == 1, "Unexpected controls project baseline")
    require(baseline["tracks"][0]["instrument"]["params"]["gain"] == -8.0
            and baseline["tracks"][0].get("pan_permille", 0) == 0, "Unexpected controls parameter baseline")

    def expected_project(revision, name=None, pan=None):
        expected = copy.deepcopy(baseline)
        expected["revision"] = revision
        if name is not None:
            expected["tracks"][0]["name"] = name
        if pan is not None:
            expected["tracks"][0]["pan_permille"] = pan
        return expected

    for label, expected in (("text-unicode", TEXT_INITIAL), ("text-astral-selection", TEXT_INITIAL),
                            ("text-assistive-selection", TEXT_INITIAL), ("text-assistive-edited", TEXT_ASSISTIVE),
                            ("text-edited", TEXT_ACCEPTED), ("text-focus-back", TEXT_ACCEPTED)):
        require(observed(label).get("text", {}).get("text") == expected, f"{label}: platform text differs")
        require(document(label) == baseline, f"{label}: unsubmitted text changed the project")
    require(observed("text-unicode").get("imeComposition") is False, "Unicode input must not claim IME composition")
    require(observed("text-astral-selection")["text"].get("selections") == [[1, 2]],
            "Actual astral selection was not observed")
    require(observed("text-assistive-selection")["text"].get("selections") == [[2, 4]]
            and observed("text-assistive-selection").get("action") ==
                {"method": "Text.SetSelection", "arguments": [0, 2, 4], "accepted": True},
            "The platform selection action did not change the real editor range")
    for label, name, role in (("text-focus-forward", "取消", "button"),
                              ("text-focus-back", "输入文字…", "entry")):
        focus = observed(label).get("focused", {})
        require(focus.get("name") == name and focus.get("role") == role and "focused" in focus.get("states", []),
                f"{label}: wrong physical focus target")
        require(any(node.get("reference") == focus.get("reference") and "focused" in node.get("states", [])
                    for node in steps[label][0]["tree"]["nodes"]), f"{label}: focus is absent from the actual tree")
    require(document("text-committed") == expected_project(1, name=TEXT_ACCEPTED), "Text commit changed another domain field")
    require(document("text-single-undo") == expected_project(2), "Text single undo did not restore the original domain state")
    require(document("text-canceled") == document("text-single-undo"), "Canceled text changed the saved project")

    initial = observed("collection-open")["options"]
    names = [item["name"] for item in initial]
    require(len(names) >= 2 and len(set(names)) == len(names), "The observed collection is incomplete or ambiguous")
    require(any(not item["visible"] for item in initial), "The real collection did not overflow its viewport")
    for label in ("collection-open", "collection-end", "collection-page-up", "collection-home", "collection-wheel",
                  "collection-search-navigation"):
        collection = observed(label)
        require([item["name"] for item in collection["options"]] == names, f"{label}: collection identity/order changed")
        require(all(item["visible"] is intersects(item["bounds"], collection["viewport"])
                    for item in collection["options"]), f"{label}: visibility disagrees with platform geometry")
    for label, name in (("collection-end", names[-1]), ("collection-page-up", names[max(0, len(names) - 21)]),
                        ("collection-home", names[0]), ("collection-search-navigation", names[1])):
        chosen = [item for item in observed(label)["options"] if item["selected"]]
        require(len(chosen) == 1 and chosen[0]["name"] == name and chosen[0]["visible"],
                f"{label}: keyboard selection/reveal differs")
    visible = lambda label: {item["name"] for item in observed(label)["options"] if item["visible"]}
    require(visible("collection-home") != visible("collection-wheel"), "Real wheel input did not change visible rows")
    require([item["name"] for item in observed("collection-wheel")["options"] if item["selected"]] == [names[0]],
            "Scrolling altered collection selection")
    empty = observed("collection-search-empty")
    require(empty.get("text", {}).get("text") == SEARCH_MISSING and empty.get("options") == [],
            "Search did not remove unmatched real options")
    search = observed("collection-search-navigation")
    require(search.get("text", {}).get("text") == "" and "focused" in search.get("focused", {}).get("states", []),
            "Search navigation did not preserve textbox value/focus")
    for label in required:
        if label.startswith("collection-"):
            require(document(label) == expected_project(2), f"{label}: navigation changed the durable project")

    preview, committed = observed("mixer-preview"), observed("mixer-committed")
    require(committed["displacement"] > preview["displacement"] > 0 and committed["width"] == preview["width"],
            "Terminal mixer sample must differ from the last observed preview")
    for label, item in (("mixer-preview", preview), ("mixer-committed", committed)):
        expected = pan_after_displacement(0, item["displacement"], item["width"])
        require(abs(item["pan"]["value"]["CurrentValue"] - expected) < 1e-7, f"{label}: actual pan differs from geometry")
    require(steps["mixer-preview"][0]["pointerHeld"] is True and document("mixer-preview") == expected_project(2),
            "Mixer preview was not provisional under a held native pointer")
    pan = round(pan_after_displacement(0, committed["displacement"], committed["width"]) * 10)
    require(document("mixer-committed") == expected_project(3, pan=pan), "Mixer final sample did not make exactly one pan edit")
    require(document("mixer-single-undo") == expected_project(4), "Mixer single undo did not restore the domain state")
    require(steps["mixer-canceled-while-held"][0]["pointerHeld"] is True
            and observed("mixer-canceled-while-held")["pan"]["value"]["CurrentValue"] == 0,
            "Mixer cancellation did not restore the value before the native button release")
    require(document("mixer-canceled-while-held") == expected_project(4)
            and document("mixer-canceled") == expected_project(4) and final == expected_project(4),
            "Canceled mixer input or its late release changed the domain state")
    for first, last in (("text-single-undo", "text-canceled"), ("text-canceled", "collection-canceled"),
                        ("mixer-single-undo", "mixer-canceled")):
        require(steps[first][0]["savedProject"]["sha256"] == steps[last][0]["savedProject"]["sha256"],
                f"{last}: canceled/navigation-only input changed saved bytes")

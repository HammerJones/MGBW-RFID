#!/usr/bin/env python3
"""Find a node on the current Android screen by its text or description and
print the centre of its bounds as "x y". Usage: uitap.py <text> [nth] [--contains]"""
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

args = [a for a in sys.argv[1:] if not a.startswith("--")]
contains = "--contains" in sys.argv
target = args[0]
nth = int(args[1]) if len(args) > 1 else 0

subprocess.run(["adb", "shell", "uiautomator", "dump", "/sdcard/ui.xml"], check=False, capture_output=True)
xml = subprocess.run(["adb", "exec-out", "cat", "/sdcard/ui.xml"], capture_output=True).stdout.decode("utf-8", "replace")
try:
    root = ET.fromstring(xml)
except ET.ParseError:
    sys.exit(f"could not read the screen (looking for {target!r})")

hits = []
for node in root.iter("node"):
    for attr in ("text", "content-desc"):
        value = node.get(attr, "")
        if value == target or (contains and target in value):
            m = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
            if m:
                x1, y1, x2, y2 = map(int, m.groups())
                hits.append(((x1 + x2) // 2, (y1 + y2) // 2))
            break
if len(hits) <= nth:
    texts = sorted({n.get("text") or n.get("content-desc") for n in root.iter("node")} - {"", None})
    sys.exit(f"{target!r} not on screen. Visible: {texts[:60]}")
print(*hits[nth])

#!/usr/bin/env bash
# Drives the app on a running emulator and saves screenshots.
# Usage: ui-smoke.sh <apk> <output dir>
set -uo pipefail
APK="$1"
OUT="$2"
PKG=com.hammerjones.snowtrack
HERE="$(dirname "$0")"
mkdir -p "$OUT"

shot() { adb exec-out screencap -p > "$OUT/$1.png"; echo "saved $1"; }
tap() {
  local xy
  if xy=$(python3 "$HERE/uitap.py" "$@"); then
    adb shell input tap $xy
  else
    echo "TAP FAILED: $*" | tee -a "$OUT/problems.txt"
    return 1
  fi
}
finish() {
  adb logcat -d -v brief AndroidRuntime:E Mbgl:W "*:S" > "$OUT/logcat.txt" 2>&1
  adb logcat -d | grep -iE "FATAL|snowtrack|UnsatisfiedLink|maplibre|Mbgl" | tail -300 > "$OUT/logcat-app.txt" 2>&1
  if grep -q "FATAL EXCEPTION" "$OUT/logcat.txt"; then echo "APP CRASHED" | tee -a "$OUT/problems.txt"; fi
  [ -f "$OUT/problems.txt" ] && cat "$OUT/problems.txt"
  exit 0
}
trap finish EXIT

adb install -r "$APK"
adb shell cmd uimode night yes
adb logcat -c
adb shell am start -n "$PKG/.MainActivity"
sleep 8
shot 01-empty

tap "Add a resort" && sleep 3
shot 02-add-list
tap "Resort or state" && sleep 1
adb shell input text "vail" && sleep 3
shot 03-search
adb shell input keyevent 111   # hide the keyboard
sleep 1
tap "Vail" && sleep 30
shot 04-home-one-card

tap "Add resort" && sleep 2
tap "Resort or state" && sleep 1
adb shell input text "killington" && sleep 3
adb shell input keyevent 111
sleep 1
tap "Killington" && sleep 30
shot 05-home-two-cards

tap "Turn on snow notifications for Vail" && sleep 4
shot 06-bell-pressed
tap "Allow" && sleep 4
shot 07-bell-on

tap "Vail" && sleep 25
shot 08-resort-radar
tap "Play loop" && sleep 20
shot 09-radar-playing
tap "Stop"
adb shell input swipe 540 1900 540 700 400 && sleep 2
shot 10-resort-totals-chart
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 11-resort-table
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 12-resort-outlook-news
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 13-resort-bottom

adb shell input keyevent 4 && sleep 2
tap "Settings" && sleep 2
shot 14-settings
adb shell cmd notification list 2>/dev/null | head -20 > "$OUT/notifications.txt" || true
adb shell dumpsys notification --noredact 2>/dev/null | grep -A3 "pkg=$PKG" | head -60 >> "$OUT/notifications.txt" || true

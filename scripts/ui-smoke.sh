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
  adb logcat -d -b crash > "$OUT/crash.txt" 2>&1
  if [ -s "$OUT/crash.txt" ] && grep -qE "FATAL|signal|backtrace" "$OUT/crash.txt"; then
    echo "APP CRASHED" | tee -a "$OUT/problems.txt"
    echo "----- crash log -----"; head -150 "$OUT/crash.txt"; echo "----- end crash log -----"
  fi
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

(tap "Add resort" || adb shell input tap 890 2220) && sleep 2
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
tap "Play" && sleep 20
shot 09-radar-playing
tap "Stop"
# Drag the scrub bar to an older frame.
if xy=$(python3 "$HERE/uitap.py" "Radar time"); then
  set -- $xy
  adb shell input swipe "$1" "$2" 330 "$2" 600 && sleep 8
else
  echo "TAP FAILED: Radar time" | tee -a "$OUT/problems.txt"
fi
shot 09b-radar-scrubbed
adb shell input swipe 540 1900 540 700 400 && sleep 2
shot 10-resort-totals-chart
# Scroll the chart sideways to the long-range days.
if xy=$(python3 "$HERE/uitap.py" "Snow by day, scrolls sideways" 0 --contains); then
  set -- $xy
  adb shell input swipe 900 "$2" 150 "$2" 400 && sleep 2
  adb shell input swipe 900 "$2" 150 "$2" 400 && sleep 2
fi
shot 10b-chart-scrolled
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 11-resort-table
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 12-resort-outlook-news
adb shell input swipe 540 1900 540 500 400 && sleep 2
shot 13-resort-bottom

adb shell input keyevent 4 && sleep 2
tap "Settings" && sleep 2
shot 14-settings
tap "Cancel" && sleep 1
# The emulator build is version 1, so the published release should be offered.
tap "Update" && sleep 6
shot 15-update-permission
adb shell appops set com.hammerjones.snowtrack REQUEST_INSTALL_PACKAGES allow
adb shell am start -n "$PKG/.MainActivity" && sleep 3
tap "Update" && sleep 45
shot 16-update-installer
adb shell cmd notification list 2>/dev/null | head -20 > "$OUT/notifications.txt" || true
adb shell dumpsys notification --noredact 2>/dev/null | grep -A3 "pkg=$PKG" | head -60 >> "$OUT/notifications.txt" || true

"""Renode capture sequence for QR Code Generator. Written but not yet run.

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app qrcode-app --script ../precursor-qrcode/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os
import time


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_menu.png")
    ctl.inject_line("")                       # Enter: "New QR code"
    time.sleep(1.0)
    ctl.inject_line("HELLO WORLD")            # version 1
    time.sleep(2.0)
    shot("02_v1.png")
    ctl.timed_key("BackSpace", after=1.0)     # back to input; the text is kept
    ctl.timed_key("F4", after=1.0)            # clear the field
    # 150 bytes at EC-M needs version 8: exercises the version-information blocks
    ctl.inject_line("https://example.com/" + "a" * 130)
    time.sleep(3.0)
    shot("03_v8.png")
    ctl.timed_key("Right", after=2.0)         # same text at EC-Q
    shot("04_v8_ecq.png")
    ctl.timed_key("BackSpace", after=1.0)
    ctl.timed_key("F4", after=1.0)            # clear
    ctl.timed_key("F1", after=1.0)            # WiFi template
    shot("05_wifi_template.png")
    ctl.timed_key("F4", after=1.0)            # clear
    ctl.timed_key("F4", after=1.0)            # back to menu
    shot("06_menu_again.png")
    return failures

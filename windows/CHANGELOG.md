# QCBuddy changelog

What changed in each QCBuddy release, newest first. The release
page for a version shows its section from this file.

## 0.1.6

- **Automatic updates on macOS.** QCBuddy checks for new macOS releases and
  offers a signed update for Apple silicon and Intel. Choose **Install and
  restart** to update without downloading another DMG.
- macOS update checks now use a published macOS feed instead of failing
  because the release feed is missing. Draft releases are not offered.

## 0.1.5

- Fix headphones remaining shown as connected after powering off or running
  out of battery. Failed Bluetooth writes now clear stale battery readings
  and controls so QCBuddy can reconnect.

## 0.1.4

- **QCBuddy for macOS.** A universal DMG (Apple silicon and Intel) of the
  same app, released alongside the Windows installer. It runs in the menu
  bar and reads the headphones through macOS Bluetooth. It doesn't update
  itself yet.
- The name is now shown as **QCBuddy** everywhere: the tray tooltip and
  menu, the flyout, the command-line tool and error messages. The install
  folder and your EQ presets stay where they were.

## 0.1.3

- **`qcb update`**: the command-line tool says whether a newer QCBuddy is
  available; `qcb update --install` lets the tray app offer it. (The tool is
  built from source for now; it isn't in the installer yet.)
- Unknown `qcb` commands are rejected straight away instead of first trying
  to connect to the headphones.
- The first release QCBuddy 0.1.2 can install by itself.

## 0.1.2

- **Automatic updates.** QCBuddy checks for a new version when it starts
  and every 6 hours; the flyout offers it and **Install and restart**
  installs it. Updates are signed and verified first, and nothing installs
  without your click.
- **Check for updates by hand:** right-click the tray icon → **Check for
  updates**, or `QCBuddy.exe --check-for-updates`.

## 0.1.1

- The tray battery follows the taskbar's light or dark mode while QCBuddy
  runs (the white digits were invisible after switching to light mode).
- A connection that can't be made fails in about 5 seconds with a clear
  message, instead of taking up to a minute.
- If another copy of QCBuddy or the `qcbuddy` command is using the
  headphones, you're told so instead of getting timeouts.
- Replies are matched to the request that asked for them, and opening or
  closing the flyout is never lost when QCBuddy is busy.

## 0.1.0

First release: a tray app for Bose QuietComfort headphones (tested on a
QC45). Battery on the tray icon; the headphones' devices with connect,
disconnect and switch; noise modes with editing and "+ Custom"; equalizer
presets; sidetone; multipoint and pairing mode.

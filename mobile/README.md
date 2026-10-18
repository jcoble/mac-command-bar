# Assembly phone app

Android app (Flutter) for chatting with the orchestrator, watching runs and answering the inbox on
the owner's machines. Design and build plan:
`docs/superpowers/specs/2026-10-12-tsk-1416-phone-app-design.md`. The look and several files are
copied from Rental Command's app (`~/dev/work/rental-management/mobile`).

The app talks only to a remote backend's phone listener (`/phone/v1`), reached over Tailscale with
HTTPS. It pairs with a machine by scanning the QR code from the desktop's
Settings → Connections → the machine → **Pair phone**, or by typing the URL and code.

## Build

Run everything from this folder with the workbox's Flutter (`~/develop/flutter`, Android SDK at
`~/Android/Sdk`). Check `free -g` first; with 6 GB or less available, wait.

```sh
~/develop/flutter/bin/flutter analyze
~/develop/flutter/bin/flutter test
~/develop/flutter/bin/flutter build apk --debug     # proof builds
~/develop/flutter/bin/flutter build apk --release   # daily use (debug-signed until an upload key exists)
```

Push notifications need Firebase's `google-services.json`, which is never committed. Copy it in
before a build that should have push; without it the app builds and runs with push off:

```sh
cp ~/.config/assembly-phone/google-services.json android/app/
```

## Install

The workbox has no emulator. Install on the phone over Tailscale with wireless debugging
(Developer options → Wireless debugging; the pairing port and code change each time):

```sh
adb pair <phone>:<pairing-port> <code>
adb connect <phone>:<port>
adb install -r build/app/outputs/flutter-apk/app-debug.apk
```

Stop the Gradle daemon when done: `cd android && ./gradlew --stop`.

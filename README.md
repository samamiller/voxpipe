# voxpipe

Voxpipe is a Linux-first speech recognition app focused on reliable dictation and hands-free control on modern desktops. It uses `whisper.cpp` as the core ASR engine and targets a simple, always-available experience for everyday work.

## Goals

- Fast, accurate offline speech recognition on Linux
- Dictation that works across common apps
- Voice commands for navigation, editing, and system control
- Accessibility-first workflows
- Packaged for Fedora and Debian with system OpenBLAS acceleration
- Automated tracking of `whisper.cpp` releases

## Status

Early stage. This repository currently focuses on packaging `whisper.cpp` and the automation that keeps it up to date.

## Dependencies

Runtime (prototype):
- `whisper.cpp` built locally (or installed from packages)
- `ffmpeg`
- PipeWire: `pw-record`
- PulseAudio tools: `pactl` and `paplay`
- Wayland clipboard: `wl-copy` and `wl-paste`

Optional:
- `ydotool` (for simulated input once command mode is implemented)

## Usage (prototype)

Initial prototype script: `scripts/voxpipe.sh`.

Notes:
- Expects a local `whisper.cpp` build at `~/whisper.cpp` and a quantized model file.
- Uses PipeWire (`pw-record`) and PulseAudio tools (`pactl`, `paplay`).
- Copies transcription output to the clipboard (`wl-copy`/`wl-paste`).
- Bluetooth card/source identifiers are currently hard-coded and should be customized.

### Configuration

Copy the example config and edit for your device:

```bash
mkdir -p ~/.config/voxpipe
cp config/voxpipe.env.example ~/.config/voxpipe/voxpipe.env
```

### Model download

Use the helper script to download a model via `whisper.cpp`:

```bash
scripts/model-download.sh base.en-q5_1
```

## ydotool (user service)

When command mode is added, `ydotool` will be used for safe input injection.
User unit files are provided in `systemd/user/`.

```bash
systemctl --user enable --now ydotool.socket
systemctl --user status ydotool.socket
```

## Tests

```bash
scripts/test.sh
```

## Packaging

See `packaging/whisper-cpp/README.md` for RPM/DEB build instructions and update workflow details.

## Planned capabilities

- Dictation with punctuation and formatting
- Text correction via voice (select, replace, delete, undo)
- Command mode for navigation and window control
- Wake word and push-to-talk options
- Multiple language profiles and per-app preferences

## Roadmap (short)

- Desktop app scaffolding (service + UI)
- Microphone capture and audio pipeline
- Dictation and command modes
- Profiles, hotkeys, and language selection

## License

See `LICENSE`.

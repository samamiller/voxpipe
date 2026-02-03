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

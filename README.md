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

Early stage, but now includes:
- GTK4/libadwaita desktop shell UI
- GStreamer mic monitor path
- `whisper.cpp` vendored CMake build driven by `cargo build`
- Bindgen-generated Rust FFI (`whisper_sys`) and a safe wrapper (`whisper_rs`)
- System OpenBLAS linkage for `whisper.cpp`

## Build and run

`cargo build` compiles the Rust app and also builds `vendor/` (`whisper.cpp`) as a shared library with CMake.

Requirements:
- Rust toolchain (`rustup`, `cargo`)
- GTK4 development libraries
- libadwaita development libraries
- GStreamer 1.0 development libraries
- C toolchain + CMake + pkg-config
- Clang/LLVM (required by `bindgen`)
- System OpenBLAS development package discoverable via `pkg-config`

Run:

```bash
cargo run
```

## Development Samples (Optional)

Extra audio samples are for local development/debug only and are not required for release builds or runtime.

To fetch extra samples and convert to 16-bit WAV:
```bash
scripts/fetch-samples.sh
```

This script:
- runs `make -j samples` in `vendor/` to download extra sample audio
- converts downloaded files to 16-bit mono WAV (16 kHz) using `ffmpeg`
- writes converted files to `~/.cache/voxpipe/samples/` (or `VOXPIPE_SAMPLES_DIR`)

## Whisper integration details

- `build.rs` drives CMake in `vendor/` with BLAS enabled and shared library output.
- OpenBLAS is resolved from the system via `pkg-config` (`openblas64` fallback to `openblas`).
- Raw generated bindings live behind `src/whisper_sys/`.
- Safe wrapper APIs are exposed from `src/whisper_rs.rs`.

Optional environment variables:
- `VOXPIPE_WHISPER_CMAKE_BUILD_TYPE` (default: `Release`)
- `VOXPIPE_WHISPER_MODEL` (path used by dev smoke model-load trigger)

ASR model discovery order:
1. CLI `--model`
2. `ASR_MODEL`
3. Preinstalled default in `/usr/share/voxpipe/models/ggml-base.en-q5_1.bin`
4. User cache under `~/.cache/voxpipe/models/`

Default bundled model:
- `ggml-base.en-q5_1.bin` is the expected preinstalled default for Voxpipe packages.
- Approximate footprint is model-dependent; `base.en-q5_1` is typically larger than tiny-class models and should be accounted for in package sizing.
- Overrides remain supported via `--model` or `ASR_MODEL`.

If the default model is missing locally, fetch a cache copy:
```bash
scripts/fetch-model.sh base.en-q5_1
export ASR_MODEL="$HOME/.cache/voxpipe/models/ggml-base.en-q5_1.bin"
```

## Runtime smoke checks

On startup, the app logs Whisper system info to stderr:
- `[whisper] system_info: ...`

Hidden dev trigger:
- Press `Ctrl+Shift+W` in the app window.
- If `VOXPIPE_WHISPER_MODEL` is set, Voxpipe attempts `WhisperContext::new(...)` and reports success/failure in the status label.
- Context is immediately dropped on success to validate clean teardown.

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

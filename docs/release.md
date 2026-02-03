# Release process

This project is early-stage and releases are lightweight. The current goal is to
ship packaging updates and prototype improvements quickly while keeping a clear
record of changes.

## Versioning

- Voxpipe does not yet have its own semantic version.
- Track the upstream `whisper.cpp` version in `packaging/whisper-cpp/VERSION`.

## Release checklist

1. Update `packaging/whisper-cpp/VERSION` and run `scripts/update-whispercpp.sh`.
2. Validate packaging metadata renders correctly (`scripts/render-packaging.sh`).
3. Run `scripts/test.sh`.
4. Update README or docs if behavior or usage changed.
5. Commit and push.
6. Create a GitHub release note (if needed).

## Packaging outputs

- Fedora: `rpmbuild -ba packaging/whisper-cpp/rpm/whisper-cpp.spec`
- Debian: `dpkg-buildpackage -us -uc` (see `packaging/whisper-cpp/README.md`)

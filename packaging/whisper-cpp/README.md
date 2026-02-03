# voxpipe packaging: whisper.cpp (OpenBLAS)

This repo packages `whisper.cpp` using system OpenBLAS on x86_64 for Fedora and Debian.
The source is a release tarball from `ggml-org/whisper.cpp`.

## Current upstream version

See `packaging/whisper-cpp/VERSION`.

## Update workflow

- `scripts/update-whispercpp.sh` checks for a newer upstream release.
- If newer, it updates `VERSION`, regenerates packaging files, and computes SHA256.
- GitHub Actions runs this on a schedule and opens a PR.

## Fedora (latest)

Build prerequisites:

```bash
sudo dnf install -y rpmdevtools cmake gcc-c++ make pkgconfig openblas-devel
```

Build:

```bash
rpmdev-setuptree
spectool -g -R packaging/whisper-cpp/rpm/whisper-cpp.spec
rpmbuild -ba packaging/whisper-cpp/rpm/whisper-cpp.spec
```

RPMs will land under `~/rpmbuild/RPMS/x86_64/`.

## Debian (latest)

Build prerequisites:

```bash
sudo apt-get update
sudo apt-get install -y build-essential debhelper cmake pkg-config libopenblas-dev
```

Build:

```bash
VER=$(cat packaging/whisper-cpp/VERSION)
curl -LO "https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v${VER}.tar.gz"
mv "v${VER}.tar.gz" "whisper.cpp-${VER}.tar.gz"
tar -xzf "whisper.cpp-${VER}.tar.gz"
cd "whisper.cpp-${VER}"
cp -r ../packaging/whisper-cpp/debian ./debian
DEB_BUILD_OPTIONS=parallel=2 dpkg-buildpackage -us -uc
```

DEBs will be in the parent directory.

#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

VERSION="$1"
RELEASE_DATE_RFC2822=$(date -R)
RELEASE_DATE_RPM=$(date "+%a %b %d %Y")

cat > packaging/whisper-cpp/rpm/whisper-cpp.spec <<SPEC
Name:           whisper-cpp
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Fast C/C++ implementation of OpenAI Whisper

License:        MIT
URL:            https://github.com/ggml-org/whisper.cpp
Source0:        https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cmake
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  pkgconfig
BuildRequires:  openblas-devel

Requires:       openblas

%description
whisper.cpp is a fast C/C++ implementation of OpenAI's Whisper ASR model. This
package builds whisper.cpp with system OpenBLAS for accelerated inference.

%prep
%autosetup -n whisper.cpp-%{version}

%build
%cmake -B build -S . \
  -DGGML_BLAS=ON \
  -DGGML_BLAS_VENDOR=OpenBLAS \
  -DWHISPER_BUILD_EXAMPLES=ON \
  -DWHISPER_BUILD_TESTS=OFF \
  -DBUILD_SHARED_LIBS=ON
%cmake_build -C build

%install
%cmake_install -C build

%files
%license LICENSE
%doc README.md
%{_bindir}/whisper-*
%{_bindir}/quantize
%{_bindir}/vad-speech-segments
%{_includedir}/whisper.h
%{_includedir}/ggml*.h
%{_libdir}/libwhisper*.so*
%{_libdir}/libggml*.so*
%{_libdir}/cmake/whisper/*
%{_libdir}/cmake/ggml/*
%{_libdir}/pkgconfig/whisper.pc
%{_libdir}/pkgconfig/ggml.pc

%changelog
* ${RELEASE_DATE_RPM} Voxpipe Builders <voxpipe@example.com> - ${VERSION}-1
- Auto-update to upstream release ${VERSION}
SPEC

cat > packaging/whisper-cpp/debian/control <<'DEBCTRL'
Source: whisper-cpp
Section: sound
Priority: optional
Maintainer: Voxpipe Builders <voxpipe@example.com>
Build-Depends: debhelper-compat (= 13), cmake, g++, pkg-config, libopenblas-dev
Standards-Version: 4.7.0
Homepage: https://github.com/ggml-org/whisper.cpp
Rules-Requires-Root: no

Package: whisper-cpp
Architecture: amd64
Depends: ${shlibs:Depends}, ${misc:Depends}, libopenblas0
Description: Fast C/C++ implementation of OpenAI Whisper
 whisper.cpp is a fast C/C++ implementation of OpenAI's Whisper ASR model.
 This package builds whisper.cpp with system OpenBLAS for accelerated inference.
DEBCTRL

cat > packaging/whisper-cpp/debian/rules <<'DEBRULES'
#!/usr/bin/make -f

export DEB_BUILD_MAINT_OPTIONS = hardening=+all

%:
	dh $@ --buildsystem=cmake

override_dh_auto_configure:
	dh_auto_configure -- \
		-DGGML_BLAS=ON \
		-DGGML_BLAS_VENDOR=OpenBLAS \
		-DWHISPER_BUILD_EXAMPLES=ON \
		-DWHISPER_BUILD_TESTS=OFF \
		-DBUILD_SHARED_LIBS=ON
DEBRULES

chmod +x packaging/whisper-cpp/debian/rules

cat > packaging/whisper-cpp/debian/changelog <<DEBCHANGE
whisper-cpp (${VERSION}-1) unstable; urgency=medium

  * Auto-update to upstream release ${VERSION}.

 -- Voxpipe Builders <voxpipe@example.com>  ${RELEASE_DATE_RFC2822}
DEBCHANGE

cat > packaging/whisper-cpp/debian/copyright <<'DEBCOPY'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: whisper.cpp
Upstream-Contact: https://github.com/ggml-org/whisper.cpp
Source: https://github.com/ggml-org/whisper.cpp

Files: *
Copyright: 2023-2026 The ggml-org/whisper.cpp contributors
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 .
 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 SOFTWARE.
DEBCOPY

cat > packaging/whisper-cpp/debian/source/format <<'DEBFORMAT'
3.0 (quilt)
DEBFORMAT

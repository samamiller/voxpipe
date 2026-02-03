Name:           whisper-cpp
Version:        1.8.2
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
%cmake -B build -S .   -DGGML_BLAS=ON   -DGGML_BLAS_VENDOR=OpenBLAS   -DWHISPER_BUILD_EXAMPLES=ON   -DWHISPER_BUILD_TESTS=OFF   -DBUILD_SHARED_LIBS=ON
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
* Mon Feb 02 2026 Voxpipe Builders <voxpipe@example.com> - 1.8.2-1
- Auto-update to upstream release 1.8.2

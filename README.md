<!-- cargo-sync-rdme title [[ -->
# elden-analyzer
<!-- cargo-sync-rdme ]] -->
<!-- cargo-sync-rdme badge [[ -->
[![Maintenance: experimental](https://img.shields.io/badge/maintenance-experimental-blue.svg?style=flat-square)](https://doc.rust-lang.org/cargo/reference/manifest.html#the-badges-section)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/elden-analyzer.svg?style=flat-square)](#license)
[![Rust: ^1.81.0](https://img.shields.io/badge/rust-^1.81.0-93450a.svg?logo=rust&style=flat-square)](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field)
[![GitHub Actions: CI](https://img.shields.io/github/actions/workflow/status/gifnksm/elden-analyzer/ci.yml.svg?label=CI&logo=github&style=flat-square)](https://github.com/gifnksm/elden-analyzer/actions/workflows/ci.yml)
[![Codecov](https://img.shields.io/codecov/c/github/gifnksm/elden-analyzer.svg?label=codecov&logo=codecov&style=flat-square)](https://codecov.io/gh/gifnksm/elden-analyzer)
<!-- cargo-sync-rdme ]] -->

Analyze videos of Eldenring playing and extract information.

## Installation

There are multiple ways to install elden-analyzer.
Choose any one of the methods below that best suits your needs.

### Pre-built binaries

Executable binaries are available for download on the [GitHub Release page].

[GitHub Release page]: https://github.com/gifnksm/elden-analyzer/releases/

### Build from source using Rust

To build `elden-analyzer` executable from the source, you must have the Rust toolchain installed.
To install the rust toolchain, follow [this guide](https://www.rust-lang.org/tools/install).

`elden-analyzer` also requires `tesseract`, `leptonica` and `ffmpeg` libraries.
To install those libraries, run the following commands:

```console
# Debian or Ubuntu
$ apt-get install -y libtesseract-dev libleptonica-dev clang libavcodec-dev libavformat-dev libavutil-dev libswscale-dev pkg-config

# Arch Linux
$ pacman -S tesseract leptonica ffmpeg
```

Once you have installed Rust and libraries, the following command can be used to build and install `elden-analyzer`:

```console
# Install released version
$ cargo install elden-analyzer

# Install latest version
$ cargo install --git https://github.com/gifnksm/elden-analyzer.git elden-analyzer
```

## License

This project is licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).

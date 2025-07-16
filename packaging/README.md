# Packaging

This directory contains the tools and processes for generating the various installers for NeoTerm.

## Tools

- `cargo-deb`: For creating `.deb` packages (Debian/Ubuntu).
- `cargo-rpm`: For creating `.rpm` packages (Fedora/CentOS/RHEL).
- `cargo-wix`: For creating `.msi` packages (Windows).
- `cargo-bundle`: For creating macOS `.app` bundles and `.dmg` images.

## Scripts

- `build_deb.sh`: Builds the `.deb` package.
- `build_rpm.sh`: Builds the `.rpm` package.
- `build_msi.ps1`: Builds the `.msi` package.
- `build_app.sh`: Builds the macOS `.app` bundle.
- `build_dmg.sh`: Creates the macOS `.dmg` image.

## Process

1.  Install the required tools.
2.  Run the appropriate script for your target platform.
3.  The installer will be generated in the `target/release` directory.

# NeoTerm Distribution & Packaging

This directory outlines the process and tools used to package NeoTerm for various operating systems. The actual packaging steps are typically executed via `cargo` subcommands or dedicated build scripts, not directly within the application's source code.

## Linux Packages

NeoTerm aims to provide distribution-specific packages for Linux users.

### AppImage
AppImage is a universal Linux package format that allows applications to run on various distributions without needing to be installed.

**Tool**: `cargo-bundle` (or custom shell scripts)
**Command Example**:
\`\`\`bash
cargo install cargo-bundle
cargo bundle appimage --release
\`\`\`
This will generate an `.AppImage` file in `target/release/bundle/AppImage/`.

### Debian (.deb)
Debian packages are used by Debian, Ubuntu, Mint, and other derivative distributions.

**Tool**: `cargo-deb`
**Command Example**:
\`\`\`bash
cargo install cargo-deb
cargo deb --release
\`\`\`
This will generate a `.deb` file in `target/debian/`.

### RPM (.rpm)
RPM packages are used by Fedora, CentOS, RHEL, openSUSE, and other RPM-based distributions.

**Tool**: `cargo-rpm`
**Command Example**:
\`\`\`bash
cargo install cargo-rpm
cargo rpm --release
\`\`\`
This will generate an `.rpm` file in `target/release/rpm/`.

## macOS Installer (.dmg)

For macOS, NeoTerm provides a disk image (`.dmg`) for easy distribution.

**Tool**: `cargo-bundle` (or custom shell scripts)
**Command Example**:
\`\`\`bash
cargo install cargo-bundle
cargo bundle dmg --release
\`\`\`
This will generate a `.dmg` file in `target/release/bundle/dmg/`.

## Windows Installer (.msi)

For Windows, NeoTerm provides a Microsoft Installer (`.msi`) package.

**Tool**: `cargo-wix` (requires WiX Toolset installed)
**Command Example**:
\`\`\`bash
cargo install cargo-wix
cargo wix --release
\`\`\`
This will generate an `.msi` file in `target/wix/`.

---

**Note**: Before running these commands, ensure you have the respective `cargo` subcommands installed and any external dependencies (like WiX Toolset for Windows) are met. These commands should be executed from the root directory of the NeoTerm project.

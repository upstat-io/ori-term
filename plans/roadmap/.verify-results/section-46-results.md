# Section 46 Verification: macOS App Bundle & Platform Packaging

## Status: PARTIALLY STARTED

### Evidence of Prior Work

**Existing `bundle-macos.sh` at repo root.** This is a working (but basic) macOS app bundle script that already:
- Builds `oriterm` and `oriterm-mux` binaries (debug or release)
- Creates `.app` directory structure (`Contents/MacOS/`, `Contents/Resources/`)
- Copies binaries into `Contents/MacOS/`
- Generates `.icns` from existing PNG assets via `iconutil`
- Writes a basic `Info.plist` with correct bundle keys (`CFBundleName`, `CFBundleIdentifier`, `CFBundleExecutable`, `CFBundleIconFile`, `CFBundlePackageType`, `NSHighResolutionCapable`, `LSMinimumSystemVersion`)

**Key differences from the plan:**
- Existing script is at `bundle-macos.sh` (root), plan targets `scripts/build-macos-bundle.sh` (new directory)
- Existing `Info.plist` is simpler (no usage description strings, no `NSRequiresAquaSystemAppearance`, no `LSApplicationCategoryType`, no `CFBundleSupportedPlatforms`, no `__VERSION__` placeholder)
- Existing script does NOT build universal binaries (no `--target x86_64-apple-darwin` / `aarch64-apple-darwin`, no `lipo`)
- Existing script does NOT codesign
- No DMG packaging
- No CI integration (nightly/release pipelines don't use it)
- `LSMinimumSystemVersion` differs: existing says `13.0`, plan says `11.0`
- `CFBundleIconFile` differs: existing says `oriterm` (without `.icns`), plan says `oriterm.icns`

**Existing icon assets** (`assets/`): `icon.svg`, `icon-16.png`, `icon-32.png`, `icon-48.png`, `icon-64.png`, `icon-128.png`, `icon-256.png`, `icon.ico`. Missing: `icon-512.png`, `icon-1024.png` (needed for Retina `.icns`). The existing `bundle-macos.sh` works around this by reusing `icon-256.png` for `icon_256x256@2x.png`.

**No `assets/macos/` directory** exists yet (plan calls for checked-in template bundle).
**No `scripts/` directory** exists yet.
**No `.gitattributes`** file exists yet.

### CI Workflows Analysis

**Nightly (`nightly.yml`):** Has a `build-macos` job, but it produces a single-arch `aarch64` tarball (not `.app` bundle, not DMG, not universal). Plan correctly identifies this needs upgrading.

**Release (`release.yml`):** Has NO macOS build job. `needs: [build-linux, build-windows]` confirms macOS is missing. Plan correctly identifies this gap.

**CI (`ci.yml`):** Has `test-macos` job (runs `cargo test --workspace` on `macos-latest`). No macOS-specific clippy job. Plan proposes optional `clippy-macos` addition.

### TODOs/FIXMEs

No TODOs or FIXMEs found related to macOS bundling in the codebase.

### Gap Analysis

The plan is **thorough and well-researched**. Specific observations:

1. **Good**: The plan references Alacritty, WezTerm, and Ghostty patterns correctly. The `Info.plist` usage description strings match WezTerm's approach (forwarding access descriptions for processes launched inside the terminal).

2. **Good**: The plan correctly identifies the universal binary requirement and the `lipo` approach (Alacritty pattern).

3. **Good**: The plan correctly notes that ad-hoc signing (`--sign -`) is required for aarch64 and that `--deep` is deprecated.

4. **Minor issue**: The plan should explicitly mention that `bundle-macos.sh` already exists and needs to be migrated/replaced, not created from scratch. The existing script provides a baseline to build upon.

5. **Minor issue**: The plan's `Info.plist` includes many usage description strings (Camera, Microphone, Bluetooth, Calendar, Contacts, Location, Reminders) that are not relevant to a terminal emulator. These come from WezTerm's template, which includes them defensively for processes launched inside the terminal. However, Apple's App Review guidelines flag unnecessary usage descriptions. Consider keeping only `NSAppleEventsUsageDescription` and `NSSystemAdministrationUsageDescription` and adding others only if needed.

6. **Good**: The `__VERSION__` placeholder replacement approach is clean and avoids build-time plist manipulation in Rust.

7. **Missing from plan**: The existing `bundle-macos.sh` should be removed or replaced as part of this section to avoid having two competing scripts.

### Infrastructure from Other Sections

- **Section 03 (Cross-Platform)** is listed as a dependency and is complete (macOS compiles and runs).
- The existing nightly/release CI infrastructure provides the foundation for the CI changes.

### Verdict

**PARTIALLY STARTED** due to existing `bundle-macos.sh`. The plan is solid but should acknowledge and subsume the existing script rather than treating this as greenfield work. The core gap (universal binary, codesign, DMG, CI integration) is correctly identified.

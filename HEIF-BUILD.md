# Building with HEIC support (`heif` feature)

The `heif` cargo feature (**on by default** since `default=["heif"]`) pulls in
`libheif-rs 2.7.0` → `libheif-sys 5.2.0+1.21.2`, which links the native
**libheif** C++ library. Getting that native library in place is the hard part;
this file records the exact process.

To build *without* HEIC (no native deps at all):

```bash
cargo build --no-default-features
```

## How libheif-sys finds the library (what actually happens)

`libheif-sys`'s `build.rs` has two discovery paths:

- **Windows + MSVC** → the `vcpkg` crate: `vcpkg::Config::find_package("libheif")`.
  Reads `VCPKG_ROOT`, requires a *static* triplet (`x64-windows-static-md` with
  the default Rust MSVC toolchain). `VCPKGRS_DYNAMIC` unset ⇒ static linking.
- **Everything else (Linux/macOS)** → `system_deps` (pkg-config). Probes
  `libheif.pc`; the minimum version comes from the libheif-rs version feature.
  Our build uses libheif-rs default features = `latest` = `v1_21`, so
  **pkg-config must report libheif >= 1.21**.

Bindings are pre-generated (no bindgen/clang needed).

## Windows — VERIFIED (this is exactly what this machine uses)

Toolchain: MSVC (`stable-x86_64-pc-windows-msvc`).

1. Clone and bootstrap vcpkg (here: `D:\Applications\vcpkg`):

   ```powershell
   git clone https://github.com/microsoft/vcpkg
   cd vcpkg
   .\bootstrap-vcpkg.bat
   ```

2. Set the environment variable **`VCPKG_ROOT`** to that directory
   (System Properties → Environment Variables, or
   `[Environment]::SetEnvironmentVariable('VCPKG_ROOT','D:\Applications\vcpkg','User')`).
   New terminals only — restart the shell.

3. Install libheif for the **static-md** triplet (this is the one vcpkg-rs
   looks for; plain `x64-windows` is ignored):

   ```powershell
   .\vcpkg install libheif:x64-windows-static-md
   ```

   The port's default feature `hevc` pulls the HEVC codecs. Versions this
   machine has: `libheif 1.21.2`, `libde265 1.0.18`, `x265 4.1#1`.
   Expect a long build (x265 especially).

4. `cargo build` in quick-viewer. The build script emits (recorded from
   `target/debug/build/libheif-sys-*/output`):

   ```
   cargo:rustc-link-search=native=<VCPKG_ROOT>\installed\x64-windows-static-md\lib
   cargo:rustc-link-lib=heif
   cargo:rustc-link-lib=x265-static
   cargo:rustc-link-lib=libde265
   ```

Gotchas hit / worth knowing:

- The triplet **must** be `x64-windows-static-md`. `x64-windows` (dynamic) is
  only used if you set `VCPKGRS_DYNAMIC=1`, and then you must ship `heif.dll`.
  `x64-windows-static` (static CRT) mismatches Rust's default `/MD` CRT.
- If vcpkg-rs can't find the port it prints a `cargo:warning` and exits —
  check `VCPKG_ROOT` spelling and the triplet of the installed package
  (`vcpkg list | findstr heif`).

## Linux — UNVERIFIED (derived from libheif-sys build.rs, not yet run)

pkg-config must find **libheif >= 1.21** (because libheif-rs's default
`latest` feature maps to `v1_21`). Three routes:

1. **Distro package, if new enough** (check with `pkg-config --modversion libheif`):

   ```bash
   # Arch
   sudo pacman -S libheif
   # Debian/Ubuntu
   sudo apt install libheif-dev pkg-config
   ```

   Ubuntu LTS ships older libheif (e.g. 24.04 has 1.17.x) — that will FAIL
   the >= 1.21 probe with default features. Then either:

2. **Pin a lower API version** matching the distro lib — in
   `qv-app/Cargo.toml` and `ee-viewer/Cargo.toml`:

   ```toml
   libheif-rs = { version="2.7.0", default-features=false, features=["image","v1_17"], optional=true }
   ```

   (a version feature is mandatory; with none, the build aborts with
   "You MUST enable one of the crate features".)

3. **Vendored build** — add the `embedded-libheif` feature to libheif-rs:
   compiles the bundled libheif via cmake (needs `cmake` + C++ toolchain, and
   it enables codecs like x265/de265/aom, so their dev packages are needed at
   link time). Slowest, most self-contained.

When a Linux build is first done for real, replace this section with the exact
commands that worked and mark it VERIFIED.

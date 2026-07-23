# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`quick-viewer` is a fast, keyboard-driven, full-screen directory image viewer built on **Rust + iced 0.14**. It is a cargo workspace (`edition = "2024"`, `resolver = "3"`) with two members:

- **`ee-viewer`** — library crate holding the actual viewer logic. This is the core (~2200 LOC); `viewer.rs` alone is ~1400.
- **`qv-app`** — binary crate; the package/binary name is **`qv`**. Thin iced shell that owns window, keyboard, file-drop, and CLI-arg concerns and delegates everything else to `ee-viewer`.

A parent-directory `D:\Applications\CLAUDE.md` describes the surrounding container of ~27 unrelated repos; this file supersedes it for anything inside `quick-viewer`.

## Build & run

```bash
cargo build
cargo run                       # scans "." by default (see args.rs default_values_t)
cargo run -- <files/dirs...>    # e.g. cargo run -- ./test_imgs_large -F
cargo run -p qv -- <dirs...>    # explicit binary select if the workspace default is ambiguous
cargo test
cargo test <name>               # single test by name filter
cargo clippy --all-targets
```

- **Debug builds are NOT fast to compile:** `Cargo.toml` forces `opt-level = 3` in the dev profile *and* for all dependencies (`[profile.dev.package."*"]`) because image decoding is unusably slow otherwise. Expect slow first builds.
- **`heif` feature is ON by default** (`default=["heif"]` in `qv-app/Cargo.toml`). It pulls in `libheif-rs` and, in `main`, calls `libheif_rs::integration::image::register_all_decoding_hooks()` so the `image` crate can decode HEIC. It links the **native libheif** library — setup is machine-specific and recorded exactly in **`HEIF-BUILD.md`** (Windows/vcpkg verified; Linux/pkg-config unverified). Build with `--no-default-features` to omit it; `.heic` files (still in the scan `EXTENSIONS` list) then fail to decode at load time.
- **iced version swap:** both member `Cargo.toml`s build against crates.io **iced 0.14** but carry commented-out `path = "../../iced"` lines for the local iced **0.15-dev** clone. To switch, comment/uncomment the paired `iced` + `iced_core` lines in *both* crates together.
- Useful runtime flags (see `qv-app/src/args.rs`): `-F/--fs` fullscreen, `-S` slideshow, `--delay <ms>`, `--cache-size`, `--look-ahead`/`--look-behind`, `--no-canvas` (start in `Image` mode), `--view-exif`, `--ns/--no-splash`, `--max-depth`.

## Architecture

Textbook Elm/iced app: state + `Message` enum + `update` + `view` + `subscription`, entered via `application::timed(...)` in `qv-app/src/main.rs`. **Two layers, each with its own message type:**

- **`qv-app::App` (`main.rs`)** — the shell. Its `Msg` enum wraps viewer messages as `Msg::Qv(QVMsg)` and forwards via `self.qv.update(m, now).map(Msg::Qv)`. **All keyboard bindings live in the `const BINDINGS` table in `main.rs`** (`Chord`/`Binding`; chords match key + modifiers exactly, first match wins) — `App::subscription` just scans it; unmatched keys go to the debug-only `probe()`. Handles window events, fullscreen toggle (F11/`f`), and quit (Esc/`q`).
- **`ee-viewer::QuickViewer` (`viewer.rs`)** — the viewer proper. Owns `QVConfig`, the `ImageList`, the LRU image cache, and pending-load bookkeeping. `App` constructs a `QVConfig` from CLI `Args`, then `QuickViewer::new(config)`.

The public surface of `ee-viewer` is small — re-exports in `lib.rs`: `QuickViewer`, `QVConfig`, `QVMsg`, `RenderMode`, `SipProgress`, `ImageKey`, `FileSystemHelper`.

### Navigation model — `ImageList` (`img_list.rs`)

The single source of truth for "what images exist and where we are."

- Items stored in `HashMap<ImageKey, Box<dyn ImageDyn>>`; `list: Vec<ImageKey>` is the ordered/sorted/shuffled *view*; `list_index` is the current position.
- **`ImageKey = NonZeroUsize`**, monotonically increasing, seeded at `0x10000EE5`. Keys are stable identity; list order changes under sort/shuffle without changing keys.
- **Indexing is normalized:** every position is a **0-based `usize`**, in the public API and internally — there is no 1-based domain. Positions (`usize`) and keys (`ImageKey`/`NonZeroUsize`) are now distinct types, so the compiler catches mixing them. The **only** place `+1` may appear is display formatting ("current/total" in `view()` and `Display for ImageList`). Sorts/shuffle preserve the *current* image by finding its key's new position afterward. Unit tests in `img_list.rs` lock the math down — run `cargo test -p ee-viewer` after touching it.
- **`peek_range(r)`** returns a `PeekWalker` iterator of 0-based positions that **wraps around both ends** (circular, true modular: `(pos+offset).rem_euclid(len)`). This is how Left/Right and the preload window work; PageUp/PageDown are `peek_range(±100)`.

### Loading, caching, preloading (in `viewer.rs`)

- `image_cache: LruCache<ImageKey, ImageCacheItem>` where `ImageCacheItem` is either `Handle(ImageHandle)` or `Alloc(ImageAllocation, Duration)` (GPU-uploaded). Capacity = `config.cache_size`.
- `pending_image_requests: HashMap<ImageKey, CacheData>` tracks in-flight loads. Each `CacheData` holds an abortable `TaskHandle` stored with `.abort_on_drop()`, so dropping/replacing cancels the load.
- **`preload_task(only_one)`** is the heart of prefetch: for the current key plus the `look_behind..=look_ahead` window (via `peek_range`), if a key is neither cached nor pending, insert into `pending_image_requests` and emit `QVMsg::RequestAnImage(key)`.
- `RequestAnImage` → `Task::perform(FileSystemHelper::load_image(...))` → `QVMsg::ImageLoaded(Result<LoadData,_>)`. Decode happens off-thread in `load_image` (open → read+exif → guess format → decode → `to_rgba8`, with `tokio::task::yield_now()` between stages).
- `show_when_loaded: Option<ImageKey>` handles "user navigated to an image that isn't cached yet" — the frame shows once that specific load completes.
- **Navigation handlers (Home/End/PageUp/PageDown/Random/Shuffle) typically `pending_image_requests.drain()` first**, cancelling now-irrelevant prefetches, then call `goto_image_task` / `preload_task`.

### Render modes (`RenderMode`)

- **`Canvas`** (default) — custom `canvas::Program for QuickViewer` impl at the bottom of `viewer.rs`. Manual fit-to-bounds math lives in `fit()` (aspect-preserving, centered). Draws `current_image_handle` or the splash (`empty_image`).
- **`Image`** — plain iced image widget (selected at startup via `--no-canvas`).
- **`Viewer`** — zoomable. **Middle-click emits `QVMsg::Swap`**, which toggles `zoom`, switches `render_mode` to `Viewer` (or back to `config.primary_render`), and **rebuilds the LRU cache from scratch** (allocation vs handle differ between modes).

### File discovery — `FileSystemHelper` (`file_system_helper.rs`)

- `find_files_sipper(dirs, max_depth)` returns an iced `Straw`/`sipper` that streams `SipProgress` (`CurrentDir(String)` for status, `SomeFiles(Vec<Box<dyn ImageDyn>>)` for batches). `App` runs it as an **abortable `Task::sip`** (`sip_dir_task: Option<TaskHandle>`), feeding files into the viewer via `QVMsg::AddFiles` as they arrive — so large trees populate incrementally.
- Filters by a hardcoded `EXTENSIONS` list (jpg/jpeg/png/gif/heic/webp, upper+lower). Supports **glob patterns** (`*?[`) via `globset`, splitting a path into a non-glob root + a `GlobMatcher`. Batches are drained at growing thresholds as the walk gets deeper.
- **File drop:** `Msg::FileDropped` kicks off the same sipper for the dropped path.

### Traits & errors

- `ImageDyn` (`img_traits.rs`) = `ImageOrigin + Display + Debug + DynClone + Send`; blanket-impl'd. `ImageOrigin` exposes `name/fqp/size/ftime/...`. `FileSystemImage` (`file_system_image.rs`) is the concrete filesystem-backed implementor. Trait objects abstract over image *sources* (only filesystem exists today).
- Fallible ops return `Result<_, ImageError>` (a plain enum in `img_traits.rs`: `NoImages`, `IndexOverflow`, `InvalidItemKey`, decode/read/open errors carrying the `ImageKey`, etc.).
- `LoadData` (`file_traits.rs`) carries the decoded `ImageHandle`, per-stage timing (`open/read/decode`), dimensions, and optional `exif::Exif`. NOTE: its `Clone` impl is `todo!()` — cloning a `LoadData` panics.

### Toasts (`toast.rs`)

Self-contained overlay-widget manager wrapping the main content; used for transient status like slideshow delay changes.

## Conventions in this repo

- `ee_conio::cprintln!` is the debug-print macro (colored ANSI via escape codes like `~[c196]`), **not** `println!` — it's a real dependency.
- Exploratory/personal style: large commented-out blocks (kept intentionally), `todo!()` in not-yet-hit branches, `panic!()`/`.expect("reality")` on "can't happen" paths, and heavy column-aligned formatting. **Match the surrounding style; do not reformat or delete dead code unless asked.**
- When touching navigation/indexing, re-derive the 1-based↔0-based conversion each time rather than trusting a nearby line — mistakes here are silent.

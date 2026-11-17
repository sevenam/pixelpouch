# pixelpouch

A desktop screenshot capture, library and editor app written in Rust ([egui](https://github.com/emilk/egui) on wgpu).

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `pixelpouch-core` | Shared types, configuration and error type |
| `pixelpouch-capture` | Screen capture |
| `pixelpouch-library` | Screenshot library / storage |
| `pixelpouch-editor` | Image editing |
| `pixelpouch-app` | The desktop application binary (`pixelpouch`) |

## Building

Install Rust via [rustup](https://rustup.rs). The toolchain is pinned in `rust-toolchain.toml` and is installed automatically on first build.

```sh
cargo run -p pixelpouch-app          # debug build
cargo run -p pixelpouch-app --release
```

### Linux dependencies

Debian/Ubuntu:

```sh
sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev
```

## Checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Conventions

- Libraries return typed errors via `thiserror` (`pixelpouch_core::Error`); the binary uses `anyhow`.
- User settings are TOML in the platform config directory (`%APPDATA%\PixelPouch\config\settings.toml`, `~/Library/Application Support/PixelPouch/settings.toml`, `~/.config/pixelpouch/settings.toml`). A corrupt file is moved to `settings.toml.bak` and defaults are used.
- Logging uses `tracing`; set `RUST_LOG` (e.g. `RUST_LOG=debug`) to change verbosity.
- Rendering uses wgpu with DX12 on Windows (for fast startup). Set `WGPU_BACKEND` (e.g. `vulkan`, `gl`) to override.

## License

MIT — see [LICENSE](LICENSE).

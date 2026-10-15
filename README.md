# family

Code shared by a family of guitar plugins: [Doubler](https://github.com/Iurig/doubler) and [Quiet](https://github.com/Iurig/quiet) so far. The family has no name yet; this repo will be renamed `<name>-core` when it has one.

| Crate | What it holds |
|---|---|
| `family-dsp` | `no_std` DSP: the audited float↔integer conversions and the click statistic |
| `family-ui` | The editor look, Night (`docs/family-identity.md`): the dial, switch, lamp and glow, the bundled Archivo, and the shared stylesheet in a plugin's colour |
| `family-wav` | WAV reading and writing for each plugin's offline `render`, and the `stats`, `split` and `join` commands (also runnable on their own as the `wav` binary) |
| `family-xtask` | `cargo xtask bundle` and `cargo xtask install`: each plugin's `xtask` is a `main` that calls `family_xtask::main` with its package, bundle name and install folder variable |

## Using it from a plugin

Plugins depend on it by git rev, like nih-plug:

```toml
family-dsp = { git = "https://github.com/Iurig/family", rev = "<commit>" }
```

A change here reaches a plugin when that plugin bumps the rev. To work on both at once, with this repo checked out beside the plugin's, build the plugin against this checkout with `.cargo/local.toml`:

```sh
cargo test --workspace --config ../family/.cargo/local.toml
```

While patched, Cargo rewrites the plugin's `Cargo.lock` to point at the local paths; the next build without `--config` points it back at the pinned rev. Push the change here and bump the plugin's rev before committing the plugin.

## Rules

The same as the plugins': `cargo +nightly fmt --all` before every commit, clippy with the workspace lints, and `family-dsp` is `no_std` without `alloc` so nothing in it can allocate on the audio thread. Nothing here may know which plugin it's in: anything plugin-specific is passed in.

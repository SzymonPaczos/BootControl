# Bundled fonts

BootControl GUI v2.1 (Granite direction) uses **Inter** as its UI font and
**JetBrains Mono** for monospace contexts (paths, ETags, CLI snippets).

The Rust side calls `slint::register_font_from_path()` at startup if the
TTF files below are present. If they are missing, the GUI silently falls
back to the system stack declared in `appwindow.slint`'s
`default-font-family`. There is no hard runtime dependency on the
bundled files.

## Files expected

| File                          | Source                                                                | License |
|-------------------------------|-----------------------------------------------------------------------|---------|
| `Inter-VariableFont.ttf`      | https://fonts.google.com/specimen/Inter                               | OFL 1.1 |
| `JetBrainsMono-Regular.ttf`   | https://fonts.google.com/specimen/JetBrains+Mono                      | OFL 1.1 |

(Variable + monospace coverage. Italic variants not used in the GUI.)

## How to add them locally

```bash
# Inter
curl -L 'https://github.com/rsms/inter/releases/download/v4.0/Inter-4.0.zip' -o /tmp/inter.zip
unzip -p /tmp/inter.zip 'Inter Desktop/InterVariable.ttf' > crates/gui/assets/fonts/Inter-VariableFont.ttf

# JetBrains Mono
curl -L 'https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip' -o /tmp/jbm.zip
unzip -p /tmp/jbm.zip 'fonts/ttf/JetBrainsMono-Regular.ttf' > crates/gui/assets/fonts/JetBrainsMono-Regular.ttf
```

After dropping the files, `cargo run -p bootcontrol-gui` picks them up
automatically — no build.rs touch required.

## Why bundled, not system

The Granite visual direction is geometric-sans first; system fallbacks
(macOS SF Pro, GNOME Cantarell, KDE Noto) are all humanist-sans, which
shifts the mood. Bundling Inter keeps the visual identity consistent
across desktops.

## How "bundled" actually works in this PR

Slint 1.8 does not expose a stable runtime font-registration API
(`slint::register_font_from_path` lands in a future minor — tracked by
the upstream issue queue). Until then, the .ttf files dropped into
this directory rely on the **OS font system** finding them. Two ways:

1. **Per-user install** — copy / symlink to:
   - Linux: `~/.local/share/fonts/` then `fc-cache -fv`
   - macOS: `~/Library/Fonts/`
2. **System-wide install** (distro packaging):
   - Linux: `/usr/share/fonts/bootcontrol/`

The `default-font-family: "Inter, Roboto, sans-serif"` declaration on
`AppWindow` resolves through the OS font stack, so once Inter is
installed by either route the GUI picks it up automatically. If
nothing matches, the system's sans-serif is used.

When Slint adds the registration API, `register_bundled_fonts()` in
`crates/gui/src/main.rs` switches to reading directly from this
directory — call-sites stay unchanged.

## License compliance

OFL 1.1 requires the license text alongside the binary distribution.
On `cargo build --release`, the licenses are copied to
`target/release/LICENSES/fonts/` (build.rs hook — TODO; for now ship a
manual `LICENSES.md` at repo root before publishing release artefacts).

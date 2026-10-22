# The `mikage` branch

This branch of rfvp is the FVP runtime of [Mikage](https://github.com/itsCheney/Mikage-Next), an iOS player for visual novels. It differs from upstream in these ways:

- **No Microsoft fonts.** Upstream compiles MS Gothic, MS Mincho, MS PGothic and MS PMincho into the engine. Those fonts may not be redistributed, so they are removed. The host registers stand-ins before it creates the engine (`rfvp_mikage_register_font_face`, `rfvp_mikage_register_fallback_font`, see `crates/rfvp/src/subsystem/resources/host_fonts.rs`). Without a registration, desktop builds take a Japanese font from the system (or `RFVP_DEFAULT_FONT`). The UEFI and console builds have no font until a host registers one.
- **No `reverse/`.** The IDA database of a commercial executable is not carried.
- **Mikage host calls** (`crates/rfvp/src/mikage_host.rs`, declared in `platform/ios/RFVPLauncher/Headers/rfvp.h`): the script to run (a `.bch` patch or one of several `.hcb` files), the folder for saves, a step that reports errors, Windows virtual-key input (Ctrl held skips), a trackpad cursor, right clicks, pausing sounds and writing the global save in the background, the game's resolution, and a log callback. Panics no longer unwind into the host: the C API catches them and reports them through `rfvp_mikage_last_error`.

The rest follows upstream [xmoezzz/rfvp](https://github.com/xmoezzz/rfvp) and keeps its license, MPL-2.0.

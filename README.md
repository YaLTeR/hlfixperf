# hlfixperf

A small library to improve Half-Life's load times on Linux.

## How it works
In Windows file and folder names are case-insensitive, while in Linux they are case-sensitive. To compensate for this, Valve made a wrapper around the filesystem calls in `filesystem_stdio.so` which walks through the given path and looks up if there are any files or folders which differ from the requested in letter cases. This can get quite lengthy since Half-Life tries opening lots of non-existent files, especially if running a mod.

There's an optimization in place which checks if the requested path starts with a path to the Steam folder and removes this initial part from the lookups. Unfortunately, if Half-Life is installed outside of the Steam folder, this optimization is completely skipped.

`hlfixperf` changes this Steam folder path used for the optimization to the Half-Life folder path. This way most of the accesses have to look through only a handful of folders, regardless of where Half-Life is installed.

The performance improvement varies, on my PC it makes the load times almost twice as short when running mods.

## Usage
Put the path to `libhlfixperf.so` into the `LD_PRELOAD` variable, either by putting `LD_PRELOAD=/full/path/to/libhlfixperf.so %command%` into the launch options in Steam, or by editing the launch script you use for BunnymodXT or hl-capture.

## Building
1. Get stable Rust, then do `rustup target add i686-unknown-linux-gnu`
2. Clone the repository and do `cargo build --release`

## License
Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

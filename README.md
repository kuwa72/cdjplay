# cdjplay

A Rekordbox PDB database parser and audio player implemented in Rust.

## Features

- Parse Pioneer Rekordbox `export.pdb` files (DeviceSQL format) exported to USB drives.
- Inspect track metadata, playlists, and cue points.
- Audio playback support via `rodio` and `symphonia` (MP3, AAC, FLAC, AIFF, WAV).
- Cross-platform GUI powered by `eframe` (egui).

## Building

```bash
cargo build --release
```

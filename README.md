# mapgeo2fbx

Converts League of Legends `.mapgeo` map geometry files to ASCII `.fbx` for viewing/editing in
Blender, Maya, 3ds Max, and similar tools.

## Usage

**Double-click** `mapgeo2fbx.exe` for an interactive menu.

**Drag and drop** a `.mapgeo` file, or a folder containing `.mapgeo` files, onto the exe — it
converts in place, writing each `.fbx` next to its source file.

**Command line:**

```
mapgeo2fbx <input.mapgeo>              # convert one file
mapgeo2fbx <input.mapgeo> -o out.fbx   # convert to a specific output path
mapgeo2fbx <folder>                    # recursively convert every .mapgeo in a folder
mapgeo2fbx <input.mapgeo> --info-only  # inspect without converting
mapgeo2fbx <input.mapgeo> --verbose    # include a per-model breakdown
mapgeo2fbx <input.mapgeo> --json       # machine-readable output
```

Run `mapgeo2fbx --help` for the full flag list.

## Scope

- Geometry (positions, normals, primary UVs) and per-submesh material names — no textures are
  extracted or embedded, since `.mapgeo` only stores texture *paths*, not texture data.
- Static geometry only — `.mapgeo` has no skeleton/animation data (that's a different format
  used by character skins).
- ASCII FBX 7.4 output only.
- Currently decodes Position/Normal/UV0 channels stored as 32-bit floats (`XyzFloat32`/
  `XyFloat32`). Some real `.mapgeo` files use packed integer vertex formats for these channels,
  which aren't decoded yet — such files will fail with an `UnsupportedVertexFormat` error rather
  than converting.

## Building

Requires the Rust toolchain pinned in `rust-toolchain.toml` (installs automatically via
`rustup` if you have it).

```
cargo build --release -p mapgeo2fbx-cli
```

The binary is written to `target/release/mapgeo2fbx.exe`.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

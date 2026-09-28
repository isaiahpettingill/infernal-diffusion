# CBOR monster records

`monster.cbor` is the same logical `Monster` record as `monster.pb`. It is a
CBOR map with UTF-8 field-name keys matching `src/proto.rs` (for example
`format_version`, `generation`, `animations`, `projectiles`). Nested messages
are maps, repeated fields are arrays, absent optional messages are CBOR null,
integers are unsigned, and numeric geometry fields are 32-bit floats. The
current `format_version` is 7; it describes the monster schema, not the choice
of protobuf or CBOR. Unknown map keys can be ignored by consumers.

The PNG atlases stay beside the metadata as `sprites.png`, `emission.png`, and
`projectiles.png` when present. Paths inside a monster record are relative to
that monster's package directory. Mounted riders and spawned minions use the
same metadata format in their child package directories.

Generate CBOR packages with `infernal bake3d-cbor "wolf" 42 output/wolf` or
`infernal bake2d-cbor ...`. Rust callers can use `generate_with_format(...,
PackageFormat::Cbor)` or `save_generated_with_format(...)`; the C API exposes
`INFERNAL_FORMAT_CBOR`, and Godot exposes `InfernalDiffusion.FORMAT_CBOR`.
`load_package` and `infernal_validate_package` detect either format. An output
directory contains one metadata file; saving to an existing directory replaces
the other format's metadata so loading cannot pick up an older record.

The `rust`, `go`, `odin`, and `csharp` folders contain complete typed records
and decoding helpers. `fixtures/monster.cbor` is a generated metadata fixture
for consumer tests. To refresh types after editing the canonical Rust schema,
run `python tools/generate_cbor_bindings.py` from the repository root.

Examples:

```rust
let monster = infernal_cbor_types::load("monster.cbor")?;
println!("{}", monster.display_name);
```

```go
monster, err := infernalcbor.Load("monster.cbor")
```

```csharp
Monster monster = Decoder.Load("monster.cbor");
```

```odin
monster, err := infernal_cbor.decode_monster(bytes)
```

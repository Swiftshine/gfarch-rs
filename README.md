# GfArch
Rust crate for handling Good-Feel's [GfArch files](https://swiftshine.github.io/doc/gfa.html).

## Capabilities
You can extract and create archives that use the following compression algorithms:
- None
- Byte Pair Encoding
- LZ10

## Usage
### Archive Creation
```rust
    // "archive_1" is now a GoodFeelArchive
    let archive_1 = gfarch::pack_from_files(
        &files,
        Version::V3,
        CompressionType::BPE,
        GFCPOffset::Default
    );

    // "archive_2" is now also a GoodFeelArchive
    let archive_2 = gfarch::pack_from_bytes(
        &byte_vectors,
        &filenames,
        Version::V3,
        CompressionType::BPE,
        GFCPOffset::Default
    );

```
### Archive Extraction
```rust
    let archive = fs::read("my_file.gfa")?;
    // "files" is now a collection of file data and filenames
    let files = gfarch::extract(&archive)?;
```

## Custom Compression Types
Below is a table of custom compression types that can be used instead of BPE or LZ10. To use one, enable its feature.
| Compression | GFCP Value (Decimal) |
| - | - |
| `zlib` | `10` |

## Notable Dependencies
- [bpe-rs](https://crates.io/crates/bpe-rs/)
    - Byte Pair Encoding
- [nintendo_lz](https://crates.io/crates/nintendo-lz)
    - LZ10/LZ11 compression/decompression


use bpe_rs::bpe;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use nintendo_lz;
use std::io::Cursor;
use thiserror;

#[derive(thiserror::Error, Debug)]
/// Errors for various GfArch problems.
pub enum GfArchError {
    #[error("Archive header was not valid")]
    ArchiveHeaderError,

    #[error("Compression header was not valid")]
    CompressionHeaderError,

    #[error("Unsupported compression type, found type with value {0}")]
    UnsupportedCompressionTypeError(u32),

    #[error("Failed to decompress LZ10")]
    LZ10DecompressError,
}

/// Allows the user to specify a custom GFCP offset.
pub enum GFCPOffset {
    Default,
    Custom(usize),
}

#[derive(PartialEq)]
/// The version of a GfArch archive.
pub enum Version {
    V2,
    V3,
    V3_1,
}

#[derive(PartialEq)]
/// The compression type of a GfArch archive.
pub enum CompressionType {
    None,
    BPE,
    LZ10,

    #[cfg(feature = "zlib")]
    Zlib,
}

#[repr(u32)]
enum RawCompressionType {
    None = 0,
    BPE = 1,
    LZ10 = 3,

    // Arbitrary values
    #[cfg(feature = "zlib")]
    Zlib = 10,
}

struct FileEntry {
    name_offset: usize,
    decompressed_size: usize,
    decompressed_offset: usize,
}

impl FileEntry {
    fn from_bytes(input: &[u8]) -> Self {
        assert_eq!(0x10, input.len());

        let name_offset = (LittleEndian::read_u32(&input[4..8]) & 0x00FFFFFF) as usize;
        let decompressed_size = LittleEndian::read_u32(&input[8..0xC]) as usize;
        let decompressed_offset = LittleEndian::read_u32(&input[0xC..0x10]) as usize;

        Self {
            name_offset,
            decompressed_size,
            decompressed_offset,
        }
    }
}

/// Calculates a checksum from a string, most commonly a filename.
///
/// ### Parameters
/// `input`: The input string.
///
/// ### Returns
/// The output checksum as a `u32`.
pub fn calculate_checksum(input: &str) -> u32 {
    let mut result: u32 = 0;

    for c in input.bytes() {
        result = c as u32 + result.wrapping_mul(137);
    }

    result
}

fn read_string(input: &[u8], offset: usize) -> String {
    let mut result = String::new();

    for &byte in &input[offset..] {
        if byte == 0 {
            break;
        }

        result.push(byte as char);
    }

    result
}

/// Extracts the contents of a GfArch archive.
///
/// ### Parameters
/// `input`: The archive contents to be extracted.
///
/// ### Returns
/// A `Vec<(String, Vec<u8>)>`, containing the contents of the archive.
pub fn extract(input: &[u8]) -> Result<Vec<(String, Vec<u8>)>, GfArchError> {
    if &input[..4] != b"GFAC" {
        return Err(GfArchError::ArchiveHeaderError);
    }

    let file_count = LittleEndian::read_u32(&input[0x2C..0x30]);
    let mut entries = Vec::new();
    let mut filenames = Vec::<String>::new();

    // read file entries

    entries.extend(
        input[0x30..]
            .chunks(0x10)
            .take(file_count as usize)
            .map(FileEntry::from_bytes),
    );

    // read filenames

    filenames.extend(
        entries
            .iter()
            .map(|entry| read_string(input, entry.name_offset)),
    );

    // read compression header

    let gfcp_offset = LittleEndian::read_u32(&input[0x14..0x18]) as usize;

    let is_compressed = input[0x8] != 0;

    let decompressed_chunk = if !is_compressed {
        input[gfcp_offset..].to_vec()
    } else {
        if &input[gfcp_offset..gfcp_offset + 4] != b"GFCP" {
            return Err(GfArchError::CompressionHeaderError);
        }

        // decompress files

        let raw_compression_type =
            LittleEndian::read_u32(&input[gfcp_offset + 0x8..gfcp_offset + 0xC]);
        
        let compression_type = match raw_compression_type {
            val if val == RawCompressionType::None as u32 => CompressionType::None,
            val if val == RawCompressionType::BPE as u32 => CompressionType::BPE,
            val if val == RawCompressionType::LZ10 as u32 => CompressionType::LZ10,

            #[cfg(feature = "zlib")]
            val if val == RawCompressionType::Zlib as u32 => CompressionType::Zlib,
            
            _ => {
                return Err(GfArchError::UnsupportedCompressionTypeError(
                    raw_compression_type,
                ))
            }
        };

        let decompressed_chunk = match compression_type {
            CompressionType::None => unreachable!(), // we already handled it

            CompressionType::BPE => {
                bpe::decode(&input[gfcp_offset + 0x14..], bpe::DEFAULT_STACK_SIZE)
            }

            CompressionType::LZ10 => {
                let decompressed_size =
                    LittleEndian::read_u32(&input[gfcp_offset + 0xC..gfcp_offset + 0x10]);

                // nintendo_lz works with headered chunks but GfArch does not.
                // construct a 4-byte header
                let mut lz_chunk = vec![0x10]; // LZ10
                lz_chunk.extend_from_slice(&decompressed_size.to_le_bytes()[..3]);
                lz_chunk.extend_from_slice(&input[gfcp_offset + 0x14..]);

                let result = nintendo_lz::decompress_arr(&lz_chunk);

                if let Ok(decompressed) = result {
                    decompressed
                } else {
                    return Err(GfArchError::LZ10DecompressError);
                }
            }

            #[cfg(feature = "zlib")]
            CompressionType::Zlib => {
                use flate2::read::ZlibDecoder;
                use std::io::Read;

                let mut decoder = ZlibDecoder::new(&input[gfcp_offset + 0x14..]);
                let mut output = Vec::new();
                decoder.read_to_end(&mut output).unwrap();
                output
            }
        };

        decompressed_chunk
    };

    let files: Vec<(String, Vec<u8>)> = (0..file_count as usize)
        .map(|i| {
            let offset = entries[i].decompressed_offset - gfcp_offset;
            let size = entries[i].decompressed_size;

            (
                filenames[i].clone(),
                decompressed_chunk[offset..offset + size].to_vec(),
            )
        })
        .collect();

    Ok(files)
}

/// Creates a GfArch archive from given files and filenames.
///
/// ### Parameters
/// `input`: The files to be put in the archive.
///
/// `filenames`: The names of each file in the archive.
///
/// `version`: The archive version.
///
/// `compression_type`: The compression type.
///
/// `offset`: An offset for the GFCP header, if specified.
/// For Yoshi's Woolly World, use `0x2000`.
///
/// ### Returns
/// A `Vec<u8>`, containing the archive.
pub fn pack_from_bytes(
    input: &[Vec<u8>],
    filenames: &[String],
    version: Version,
    compression_type: CompressionType,
    offset: GFCPOffset,
) -> Vec<u8> {
    assert_eq!(input.len(), filenames.len());

    let files: Vec<(String, Vec<u8>)> = (0..input.len())
        .map(|i| (filenames[i].clone(), input[i].to_vec()))
        .collect();

    pack_from_files(&files, version, compression_type, offset)
}

/// Creates a GfArch archive from given files.
///
/// ### Parameters
/// `input`: The filenames and contents to be put in the archive,
///
/// `version`: The archive version.
///
/// `compression_type`: The compression type.
///
/// `offset`: An offset for the GFCP header, if specified.
/// For Yoshi's Woolly World, use `0x2000`.
/// ### Returns
/// A `Vec<u8>`, containing the archive.
pub fn pack_from_files(
    input: &[(String, Vec<u8>)],
    version: Version,
    compression_type: CompressionType,
    offset: GFCPOffset,
) -> Vec<u8> {
    // Yoshi's Woolly World is the only known game
    // that consistently picks the same offset

    let file_count = input.len();

    // concatenate all data
    let mut decompressed_chunk = Vec::new();

    for file in input.iter() {
        decompressed_chunk.extend_from_slice(&file.1);
        decompressed_chunk.resize(decompressed_chunk.len().next_multiple_of(0x10), 0);
    }

    // compress all data
    let compressed_chunk = match compression_type {
        CompressionType::None => decompressed_chunk.clone(),

        CompressionType::BPE => bpe::encode(&decompressed_chunk),

        CompressionType::LZ10 => {
            // create a cursor so we can specify LZ10
            let mut compressed: Vec<u8> = Vec::new();
            let mut writer = Cursor::new(&mut compressed);
            nintendo_lz::compress(
                &decompressed_chunk,
                &mut writer,
                nintendo_lz::CompressionLevel::LZ10,
            )
            .unwrap();

            // nintendo_lz works with headered chunks but GfArch does not.
            // the 4-byte header must be removed here

            compressed[4..].to_vec()
        }

        #[cfg(feature = "zlib")]
        CompressionType::Zlib => {
            use flate2::{Compression, write::ZlibEncoder};
            use std::io::Write;

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            let _ = encoder.write_all(&decompressed_chunk);
            encoder.finish().unwrap()
        }
    };

    let mut file_name_section_length = 0usize;

    for file in input.iter() {
        file_name_section_length += file.0.len();
    }

    let is_compressed = !matches!(compression_type, CompressionType::None);

    let compression_header_size = if is_compressed { 0x14 } else { 0 };

    let _archive_size = match offset {
        GFCPOffset::Default => {
            0x30 + // archive header
                (file_count * 0x10) + // file entries
                file_name_section_length.next_multiple_of(0x10) + // filenames
                compression_header_size + // compression header
                compressed_chunk.len() // compressed data
        }

        GFCPOffset::Custom(offs) => offs + compression_header_size + compressed_chunk.len(),
    };

    // write archive header
    // let mut output = vec![0u8; archive_size];
    let mut output = Vec::new();

    // magic
    output.extend(b"GFAC");

    // version
    let _ = output.write_u32::<LittleEndian>(match version {
        Version::V2 => 0x0200,
        Version::V3 => 0x0300,
        Version::V3_1 => 0x0301,
    });

    // is compressed
    let _ = output.write_u8(if matches!(compression_type, CompressionType::None) {
        0
    } else {
        1
    });

    // padding
    for _ in 0..3 {
        output.push(0);
    }

    // file entry offset
    let _ = output.write_u32::<LittleEndian>(0x2C);

    // file info size
    let file_info_size: u32 = 4 + // the actual beginning of the file info
            (file_count * 0x10) as u32 + // file entries
            file_name_section_length as u32 + // length of all strings
            file_count as u32; // (plus null terminators)

    let _ = output.write_u32::<LittleEndian>(file_info_size);

    let file_info_size = file_info_size.next_multiple_of(0x10);

    // gfcp offset
    let gfcp_offset: u32 = match offset {
        GFCPOffset::Default => 0x30 + file_info_size,
        GFCPOffset::Custom(offs) => offs as u32,
    };

    let _ = output.write_u32::<LittleEndian>(gfcp_offset);

    // payload size
    let _ =
        output.write_u32::<LittleEndian>((compression_header_size + compressed_chunk.len()) as u32);

    // more padding
    for _ in 0..0x10 {
        output.push(0);
    }

    // file count
    let _ = output.write_u32::<LittleEndian>(file_count as u32);

    // write file entries
    let mut cur_name_offset = 0x30 + // header size
            (file_count * 0x10); // file entries

    let mut decompressed_offset = gfcp_offset;
    for i in 0..file_count {
        let checksum = calculate_checksum(&input[i].0);
        let name_offset = if i == file_count - 1 {
            // if last entry, apply a flag to indicate so
            cur_name_offset as u32 | 0x80000000
        } else {
            cur_name_offset as u32
        };

        // checksum
        let _ = output.write_u32::<LittleEndian>(checksum);
        // name offset
        let _ = output.write_u32::<LittleEndian>(name_offset);
        // size
        let _ = output.write_u32::<LittleEndian>(input[i].1.len() as u32);
        // offset
        let _ = output.write_u32::<LittleEndian>(decompressed_offset);

        // update offsets
        cur_name_offset += input[i].0.len() + 1;
        decompressed_offset += (input[i].1.len() as u32).next_multiple_of(0x10);
    }

    for file in input.iter() {
        let filename_bytes = file.0.as_bytes();
        output.extend_from_slice(filename_bytes);
        output.push(0); // null terminator
    }

    let gfcp_offset = gfcp_offset as usize;

    // padding
    output.resize(gfcp_offset.next_multiple_of(0x10), 0);

    if !is_compressed {
        output.extend(&compressed_chunk);
    } else {
        // write compression header
        // magic
        output.extend_from_slice(b"GFCP");

        // "version" -- this value is always 1
        let _ = output.write_u32::<LittleEndian>(1);

        // compression type
        let _ = output.write_u32::<LittleEndian>(match compression_type {
            CompressionType::None => RawCompressionType::None,
            CompressionType::BPE => RawCompressionType::BPE,
            CompressionType::LZ10 => RawCompressionType::LZ10,

            #[cfg(feature = "zlib")]
            CompressionType::Zlib => RawCompressionType::Zlib
        } as u32);

        // decompressed size
        let _ = output.write_u32::<LittleEndian>(decompressed_chunk.len() as u32);

        // compressed size
        let _ = output.write_u32::<LittleEndian>(compressed_chunk.len() as u32);

        // write the compressed data
        output.extend(&compressed_chunk);
    }

    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn validate_checksum() {
        let sample = "sea_turtle_01.brres";
        let checksum = super::calculate_checksum(sample);
        assert_eq!(0xCC91B7B8, checksum.swap_bytes());
    }
}

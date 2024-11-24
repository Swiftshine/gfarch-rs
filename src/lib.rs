pub mod gfarch {
    use bpe_rs::bpe;
    use byteorder::{ByteOrder, LittleEndian};
    use thiserror;

    #[derive(thiserror::Error, Debug)]
    /// Errors for various GfArch problems. 
    pub enum GfArchError {
        #[error("Archive header was not valid")]
        ArchiveHeaderError,

        #[error("Compression header was not valid")]
        CompressionHeaderError,

        #[error("Unsupported compression type, found type with value {0}")]
        UnsupportedCompressionTypeError(u32)
    }

    /// The version of a GfArch archive.
    pub enum Version {
        V2_0,
        V3_0,
        V3_1,
    }

    /// The compression type of a GfArch archive.
    pub enum CompressionType {
        BPE,
        LZ77
    }

    struct FileEntry {
        _checksum: u32,
        name_offset: usize,
        decompressed_size: usize,
        decompressed_offset: usize,
    }

    pub struct FileContents {
        pub contents: Vec<u8>,
        pub filename: String
    }


    impl FileEntry {
        fn from_bytes(input: &[u8]) -> Self {
            assert_eq!(0x10, input.len());

            let _checksum = LittleEndian::read_u32(&input[..4]);
            let name_offset = (LittleEndian::read_u32(&input[4..8]) & 0x00FFFFFF) as usize;
            let decompressed_size = LittleEndian::read_u32(&input[8..0xC]) as usize;
            let decompressed_offset = LittleEndian::read_u32(&input[0xC..0x10]) as usize;
            
            Self {
                _checksum,
                name_offset,
                decompressed_size,
                decompressed_offset
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
    /// A `Vec<FileContents>`, containing the contents of the archive.
    pub fn extract(input: &[u8]) -> Result<Vec<FileContents>, GfArchError> {
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
            .map(FileEntry::from_bytes)
        );

        // read filenames
        
        filenames.extend(
            entries.iter().map(|entry|
                read_string(input, entry.name_offset)
            )
        );

        // read compression header

        let gfcp_offset = LittleEndian::read_u32(&input[0x14..0x18]) as usize;

        if &input[gfcp_offset..gfcp_offset + 4] != b"GFCP" {
            return Err(GfArchError::CompressionHeaderError);
        }

        // decompress files

        let raw_compression_type = LittleEndian::read_u32(&input[gfcp_offset + 0x8..gfcp_offset + 0xC]); 
        let compression_type = match raw_compression_type {
            1 => CompressionType::BPE,
            // 3 => CompressionType::LZ77,
            _ => {
                return Err(GfArchError::UnsupportedCompressionTypeError(raw_compression_type))
            }
        };

        let decompressed_chunk = match compression_type {
            CompressionType::BPE => bpe::decode(&input[gfcp_offset + 0x14..], bpe::DEFAULT_STACK_SIZE),
            CompressionType::LZ77 => {
                todo!()
            }
        };

        let files: Vec<FileContents> = (0..file_count as usize)
            .map(|i| {
                let offset = entries[i].decompressed_offset - gfcp_offset;
                let size = entries[i].decompressed_size;

                FileContents {
                    contents: decompressed_chunk[offset..offset + size].to_vec(),
                    filename: filenames[i].clone(),
                }
            }).collect();

        Ok(files)
    }



    /// Creates a GfArch archive from given files.
    /// 
    /// ### Parameters
    /// `input`: The files to be put in the archive.
    /// `version`: The archive version.
    /// `compression_type`: The compression type.
    /// 
    /// ### Returns
    /// A `Vec<u8>`, containing the archive.
    pub fn pack(_input: &[Vec<u8>], _version: Version, _compression_type: CompressionType) -> Vec<u8> {
        todo!()
    }

    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_checksum() {
        let sample = "sea_turtle_01.brres";
        let checksum = gfarch::calculate_checksum(sample);
        assert_eq!(0xCC91B7B8, checksum.swap_bytes());
    }
}

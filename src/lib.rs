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
        BPE,
        LZ77
    }

    struct FileEntry {
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

            let name_offset = (LittleEndian::read_u32(&input[4..8]) & 0x00FFFFFF) as usize;
            let decompressed_size = LittleEndian::read_u32(&input[8..0xC]) as usize;
            let decompressed_offset = LittleEndian::read_u32(&input[0xC..0x10]) as usize;

            Self {
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
    /// `is_yww`: Indicates if the archive is for Yoshi's Woolly World.
    /// This moves the GFCP offset to a consistent `0x2000`.
    /// ### Returns
    /// A `Vec<u8>`, containing the archive.
    pub fn pack_from_bytes(
        input: &[Vec<u8>],
        filenames: &[String],
        version: Version,
        compression_type: CompressionType,
        is_yww: bool
    ) -> Vec<u8> {
        assert_eq!(input.len(), filenames.len());

        let files: Vec::<FileContents> = (0..input.len())
            .map(|i| {
                FileContents {
                    contents: input[i].to_vec(),
                    filename: filenames[i].clone()
                }
            }).collect();


        pack_from_files(&files, version, compression_type, is_yww)
    }
    
    /// Creates a GfArch archive from given files.
    /// 
    /// ### Parameters
    /// `input`: The files to be put in the archive.
    /// 
    /// `version`: The archive version.
    /// 
    /// `compression_type`: The compression type.
    /// 
    /// `is_yww`: Indicates if the archive is for Yoshi's Woolly World.
    /// This moves the GFCP offset to a consistent `0x2000`.
    /// ### Returns
    /// A `Vec<u8>`, containing the archive.
    pub fn pack_from_files(
        input: &[FileContents],
        version: Version,
        compression_type: CompressionType,
        is_yww: bool
    ) -> Vec<u8> {
        // Yoshi's Woolly World is the only known game
        // that consistently picks the same offset

        let file_count = input.len();

        // concatenate all data
        let mut decompressed_chunk = Vec::new();

        for file in input.iter() {
            decompressed_chunk.extend_from_slice(&file.contents);
            if decompressed_chunk.len() % 0x10 != 0 {
                let padding = 0x10 - (decompressed_chunk.len() % 0x10);
                decompressed_chunk.extend(vec![0; padding]);
            }
        }

        // compress all data
        let compressed_chunk = match compression_type {
            CompressionType::BPE => bpe::encode(&decompressed_chunk),
            CompressionType::LZ77 => todo!()
        };

        let mut file_name_section_length = 0usize;

        for file in input.iter() {
            file_name_section_length += file.filename.len();
        }
        
        let archive_size = if is_yww {
            0x2000 + 0x14 + compressed_chunk.len()
        } else {
            0x30 + // archive header
            (0x10 * file_count) + // file entries
            file_name_section_length.next_multiple_of(0x10) + // filenames
            0x14 + // compression header
            compressed_chunk.len() // compressed data   
        };
        
        // write archive header
        let mut output = vec![0u8; archive_size];
        
        // magic
        output[0] = b'G';
        output[1] = b'F';
        output[2] = b'A';
        output[3] = b'C';

        // version
        LittleEndian::write_u32(&mut output[0x4..0x8], match version {
            Version::V2 => 0x0200,
            Version::V3 => 0x0300,
            Version::V3_1 => 0x0301,
        });

        // is compressed
        output[0x8] = 1;

        // file entry offset
        LittleEndian::write_u32(&mut output[0xC..0x10], 0x2C);

        // file info size
        let file_info_size: u32 =
            4 + // the actual begginning of the file info
            (file_count * 0x10) as u32 + // file entries
            file_name_section_length as u32 + // length of all strings
            file_count as u32; // (plus null terminators)


        LittleEndian::write_u32(&mut output[0x10..0x14], file_info_size);

        let file_info_size = file_info_size.next_multiple_of(0x10);

        // gfcp offset
        let gfcp_offset = if is_yww {
            // Yoshi's Woolly World is the only known game
            // that consistently picks the same offset
            0x2000
        } else {
            0x30 + // header size
            file_info_size
        };

        LittleEndian::write_u32(&mut output[0x14..0x18], gfcp_offset);

        // payload size
        LittleEndian::write_u32(
            &mut output[0x18..0x1C],
            {
                0x14 + // gfcp header
                compressed_chunk.len() as u32
            }
        );

        // file count
        LittleEndian::write_u32(&mut output[0x2C..0x30], file_count as u32);

        // write file entries
        let mut cur_name_offset =
            0x30 + // header size
            (file_count * 0x10); // file entries

        let mut decompressed_offset = 0x30 + file_info_size;
        for i in 0..file_count {
            let checksum = calculate_checksum(&input[i].filename);
            let name_offset = if i == file_count - 1 {
                // if last entry, apply a flag to indicate so
                cur_name_offset as u32 | 0x80000000
            } else {
                cur_name_offset as u32
            };

            
            let data_offset = decompressed_offset;
            
            cur_name_offset += input[i].filename.len() + 1;
            decompressed_offset += input[i].contents.len() as u32;

            let offset = 0x30 + (i * 0x10);
            
            
            // checksum
            LittleEndian::write_u32(&mut output[offset..offset + 4], checksum);
            // name offset
            LittleEndian::write_u32(&mut output[offset + 4..offset + 8], name_offset);
            // size
            LittleEndian::write_u32(&mut output[offset + 8..offset + 0xC], input[i].contents.len() as u32);
            // offset
            LittleEndian::write_u32(&mut output[offset + 0xC..offset + 0x10], data_offset);
        }

        // write strings
        let mut name_offs = 0x30 + (file_count * 0x10);

        for file in input.iter() {
            let filename_bytes = file.filename.as_bytes();
            output[name_offs..name_offs + filename_bytes.len()].copy_from_slice(filename_bytes);
            name_offs += filename_bytes.len();
                
            output[name_offs] = 0; // null terminator
            name_offs += 1;
        }

        // write compression header
        // magic

        let gfcp_offset = gfcp_offset as usize;
        output[gfcp_offset] = b'G';
        output[gfcp_offset + 1] = b'F';
        output[gfcp_offset + 2] = b'C';
        output[gfcp_offset + 3] = b'P';

        
        // "version" -- this value is always 1
        LittleEndian::write_u32(&mut output[gfcp_offset + 4..gfcp_offset + 8], 1);

        // write compression type
        LittleEndian::write_u32(
            &mut output[gfcp_offset + 8..gfcp_offset + 0xC],

            match compression_type {
                CompressionType::BPE => 1,
                CompressionType::LZ77 => 3
            }
        );

        // decompressed size
        LittleEndian::write_u32(
            &mut output[gfcp_offset + 0xC..gfcp_offset + 0x10],
            decompressed_chunk.len() as u32
        );

        // compressed size
        LittleEndian::write_u32(
            &mut output[gfcp_offset + 0x10..gfcp_offset + 0x14],
            compressed_chunk.len() as u32
        );

        // write the compressed data
        output[gfcp_offset + 0x14..gfcp_offset + 0x14 + compressed_chunk.len()]
        .copy_from_slice(&compressed_chunk);
        
        output
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

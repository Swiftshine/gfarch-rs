pub mod gfarch {
    pub enum Version {
        V2_0,
        V3_0,
        V3_1,
    }

    pub enum CompressionType {
        BPE,
        LZ77
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
            // if c == 0 {
            //     break;
            // }
            result = c as u32 + result.wrapping_mul(137);
        }

        result
    }

    /// Extracts the contents of a GfArch archive.
    /// 
    /// ### Parameters
    /// `input`: The archive contents to be extracted.
    /// 
    /// ### Returns
    /// A `Vec<Vec<u8>>`, containing the contents of the archive.
    pub fn extract(_input: &[u8]) -> Vec<Vec<u8>> {
        todo!()
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

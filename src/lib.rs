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

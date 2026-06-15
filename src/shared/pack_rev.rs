const PACK_REV_MAGIC_NUMBER: u32 = 0x52494458; // "RIDX" in ASCII

#[allow(dead_code)]
pub struct PackRev {
    pub table: Vec<PackRevIndexPosition>,
    pub packfile_checksum: Vec<u8>, // 20-byte SHA-1 hash of the corresponding packfile
    pub total_checksum: Vec<u8>, // 20-byte SHA-1 hash of the entire pack_rev file (excluding this field)
}

#[allow(dead_code)]
pub struct PackRevIndexPosition(pub u32); // 4-byte integer representing the position of the corresponding object in the packfile (network order)

impl PackRev {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        // Validate magic number
        if bytes.len() < 4 {
            return Err("PackRev data is too short to contain magic number".to_string());
        }

        let magic_number = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        if magic_number != PACK_REV_MAGIC_NUMBER {
            return Err(format!(
                "Invalid magic number: expected 0x{:08X}, found 0x{:08X}",
                PACK_REV_MAGIC_NUMBER, magic_number
            ));
        }

        // Grab 4-byte version identifier (currently unused, but we can use it in the future if we need to make breaking changes to the format)
        if bytes.len() < 8 {
            return Err("PackRev data is too short to contain version number".to_string());
        }

        let _version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());

        // The last 40 bytes are the two 20-byte checksums, so the table data is everything in between
        if bytes.len() < 48 {
            return Err("PackRev data is too short to contain checksums".to_string());
        }

        let table_data = &bytes[8..bytes.len() - 40];
        if table_data.len() % 4 != 0 {
            return Err("PackRev table data length is not a multiple of 4".to_string());
        }

        let mut table = Vec::new();
        for chunk in table_data.chunks(4) {
            let position = PackRevIndexPosition(u32::from_be_bytes(chunk.try_into().unwrap()));
            table.push(position);
        }

        let packfile_checksum = bytes[bytes.len() - 40..bytes.len() - 20].to_vec();
        let total_checksum = bytes[bytes.len() - 20..].to_vec();

        Ok(Self {
            table,
            packfile_checksum,
            total_checksum,
        })
    }
}

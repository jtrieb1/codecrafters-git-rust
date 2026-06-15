#[allow(dead_code)]
pub struct PackIdx {
    pub fan_out_table: [u32; 256],
    pub object_hashes: Vec<[u8; 20]>,
    // pub crc32s: Vec<u32>,
    pub offsets: Vec<u64>,
    pub packfile_checksum: Vec<u8>, // 20-byte SHA-1 hash of the corresponding packfile
    pub total_checksum: Vec<u8>, // 20-byte SHA-1 hash of the entire pack_idx file (excluding this field)
}

impl PackIdx {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        // Magic number is listed as \377tOc in the Git documentation, which is 0xFF 0x74 0x4F 0x63 in hex
        const MAGIC_NUMBER: &[u8; 4] = &[0xFF, 0x74, 0x4F, 0x63];
        if bytes.len() < 8 {
            return Err("Index file too short".to_string());
        }
        if &bytes[0..4] != MAGIC_NUMBER {
            return PackIdx::from_bytes_version_1(bytes);
        }
        let version = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != 2 {
            return Err(format!("Unsupported index version: {}", version));
        }
        let mut fan_out_table = [0u32; 256];
        for i in 0..256 {
            let offset = 8 + i * 4;
            fan_out_table[i] = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
        }
        let object_count = fan_out_table[255] as usize;
        let mut object_hashes = Vec::with_capacity(object_count);
        let mut crc32s = Vec::with_capacity(object_count);
        let mut offsets = Vec::with_capacity(object_count);
        let mut pos = 8 + 256 * 4;
        for _ in 0..object_count {
            let hash: [u8; 20] = bytes[pos..pos + 20]
                .try_into()
                .map_err(|_| "Failed to read object hash".to_string())?;
            object_hashes.push(hash);
            pos += 20;
        }
        for _ in 0..object_count {
            let crc32 =
                u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
            crc32s.push(crc32);
            pos += 4;
        }
        for _ in 0..object_count {
            let offset =
                u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                    as u64;
            offsets.push(offset);
            pos += 4;
        }

        // The last 40 bytes are the two 20-byte checksums
        if bytes.len() < pos + 40 {
            return Err("Index file too short to contain checksums".to_string());
        }
        let packfile_checksum = bytes[pos..pos + 20].to_vec();
        let total_checksum = bytes[pos + 20..pos + 40].to_vec();

        Ok(PackIdx {
            fan_out_table,
            object_hashes,
            // crc32s,
            offsets,
            packfile_checksum,
            total_checksum,
        })
    }

    fn from_bytes_version_1(_bytes: &[u8]) -> Result<Self, String> {
        Err("Unsupported index version: 1".to_string())
    }

    pub fn get_object_location(&self, hash: &[u8; 20]) -> Option<u64> {
        let first_byte = hash[0] as usize;
        let start = if first_byte == 0 {
            0
        } else {
            self.fan_out_table[first_byte - 1] as usize
        };
        let end = self.fan_out_table[first_byte] as usize;
        for i in start..end {
            if &self.object_hashes[i] == hash {
                return Some(self.offsets[i]);
            }
        }
        None
    }
}

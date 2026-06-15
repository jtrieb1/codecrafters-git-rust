const PACK_MTIMES_MAGIC_NUMBER: u32 = 0x4D544D45; // "MTME" in ASCII

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    SHA1,
    SHA256,
}

#[allow(dead_code)]
pub struct PackMTimes {
    pub algo: ChecksumAlgorithm, // 4 bytes indicate the checksum algorithm used for the packfile and total checksums (0x01 for SHA-1, 0x02 for SHA-256)
    pub mtimes: Vec<MTime>, // Vector of MTime structs, one for each object in the packfile, in the same order as the objects
    pub packfile_checksum: Vec<u8>, // 20-byte SHA-1 hash of the corresponding packfile or 32-byte SHA-256 hash of the corresponding packfile, depending on the checksum algorithm used
    pub total_checksum: Vec<u8>, // 20-byte SHA-1 hash or 32-byte SHA-256 hash of the entire pack_mtimes file (excluding this field), depending on the checksum algorithm used
}

#[allow(dead_code)]
pub struct MTime(pub u32); // 4-byte integer representing the mtime of the corresponding object in the packfile (network order)

impl PackMTimes {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        // Match magic number
        if bytes.len() < 4 {
            return Err("PackMTimes data is too short to contain magic number".to_string());
        }

        let magic_number = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        if magic_number != PACK_MTIMES_MAGIC_NUMBER {
            return Err(format!(
                "Invalid magic number: expected 0x{:08X}, found 0x{:08X}",
                PACK_MTIMES_MAGIC_NUMBER, magic_number
            ));
        }

        // Grab 4-byte version identifier (currently unused, but we can use it in the future if we need to make breaking changes to the format)
        if bytes.len() < 8 {
            return Err("PackMTimes data is too short to contain version number".to_string());
        }

        let _version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());

        // Grab the 4 byte checksum algorithm identifier
        if bytes.len() < 12 {
            return Err(
                "PackMTimes data is too short to contain checksum algorithm identifier".to_string(),
            );
        }

        let algo_id = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let algo = match algo_id {
            0x01 => ChecksumAlgorithm::SHA1,
            0x02 => ChecksumAlgorithm::SHA256,
            _ => {
                return Err(format!(
                    "Invalid checksum algorithm identifier: expected 0x01 or 0x02, found 0x{:08X}",
                    algo_id
                ));
            }
        };

        // The last 40 or 64 bytes are the two checksums, so the table data is everything in between
        let checksum_length = match algo {
            ChecksumAlgorithm::SHA1 => 20,
            ChecksumAlgorithm::SHA256 => 32,
        };

        if bytes.len() < 12 + checksum_length * 2 {
            return Err("PackMTimes data is too short to contain checksums".to_string());
        }

        let table_data = &bytes[12..bytes.len() - checksum_length * 2];
        if table_data.len() % 4 != 0 {
            return Err("PackMTimes table data length is not a multiple of 4".to_string());
        }

        let mut mtimes = Vec::new();
        for chunk in table_data.chunks(4) {
            let mtime = MTime(u32::from_be_bytes(chunk.try_into().unwrap()));
            mtimes.push(mtime);
        }

        let packfile_checksum =
            bytes[bytes.len() - checksum_length * 2..bytes.len() - checksum_length].to_vec();
        let total_checksum = bytes[bytes.len() - checksum_length..].to_vec();

        Ok(Self {
            algo,
            mtimes,
            packfile_checksum,
            total_checksum,
        })
    }
}

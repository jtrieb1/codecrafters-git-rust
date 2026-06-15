use std::{collections::HashMap, io::{Read, Seek}};

use sha1::Digest;

use crate::shared::{
    object::{Object, ObjectType},
    pack_idx::PackIdx,
    pack_mtimes::{ChecksumAlgorithm, PackMTimes},
    pack_rev::PackRev,
};

pub fn size_decode(partial_size_byte: u8, whole_size_bytes: &[u8]) -> usize {
    // Taken from the Git documentation:
    // From each byte, the seven least significant bits are used to form the resulting integer.
    // As long as the most significant bit is 1, this process continues; the byte with MSB 0 provides the last seven bits.
    // The seven-bit chunks are concatenated. Later values are more significant.
    let mut size = (partial_size_byte & 0x0F) as usize;
    let mut shift = 4;

    // Only look for more bytes if the MSB of the first byte is 1
    if (partial_size_byte & 0x80) != 0 {
        for &byte in whole_size_bytes {
            size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            
            if (byte & 0x80) == 0 {
                break; // Reached the end of the variable-length integer
            }
        }
    }
    size
}

pub fn offset_decode(offset_bytes: &[u8]) -> (usize, usize) {
    // Taken from the Git documentation:
    // offset encoding:
    // n bytes with MSB set in all but the last one.
    // The offset is then the number constructed by
    // concatenating the lower 7 bit of each byte, and
    // for n >= 2 adding 2^7 + 2^14 + ... + 2^(7*(n-1))
    // to the result.
    // offset is signed.
    let mut offset = 0;
    let mut bytes_consumed = 0;
    for byte in offset_bytes {
        bytes_consumed += 1;
        offset = (offset << 7) | ((byte & 0x7F) as usize);
        if byte & 0x80 == 0 {
            break; // Last byte of the offset encoding
        }
    }
    
    if bytes_consumed >= 2 {
        for i in 2..bytes_consumed + 1 {
            let to_add = 1 << (7 * (i - 1));
            offset += to_add;
        }
    }

    (offset, bytes_consumed)
}

pub fn parse_type_and_size(type_and_size: &[u8]) -> Result<(PackObjectType, usize), String> {
    if type_and_size.is_empty() {
        return Err("Type and size data is empty".to_string());
    }
    let first_byte = type_and_size[0];
    // We're using the same variable-length encoding for the size as Git does in packfiles, which means the first byte contains both the type and part of the size.
    // Be very careful about endian-ness here, since the size is encoded in a variable-length format where the high bit of each byte indicates whether there are more bytes to read for the size.
    // The type takes up the first 3 bits, and the size takes up the remaining 4 bits of the first byte, plus any additional bytes if the high bit is set.
    let type_bits = (first_byte >> 4) & 0x07; // Get the first 3 bits for the type
    let object_type = match type_bits {
        1 => PackObjectType::Commit,
        2 => PackObjectType::Tree,
        3 => PackObjectType::Blob,
        4 => PackObjectType::Tag,
        6 => PackObjectType::OfsDelta,
        7 => PackObjectType::RefDelta,
        _ => return Err(format!("Unknown object type: {}", type_bits)),
    };
    let size = size_decode(first_byte, &type_and_size[1..]);
    Ok((object_type, size))
}

pub fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    // The delta format is:
    // - A variable-length integer encoding the size of the base object
    // - A variable-length integer encoding the size of the resulting object
    // - A series of instructions to transform the base object into the resulting object
    let mut cursor = 0;

    let mut _base_size = 0;
    let mut shift = 0;
    while cursor < delta.len() {
        let byte = delta[cursor];
        cursor += 1;
        _base_size |= ((byte & 0x7F) as usize) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break; // Last byte of the base size encoding
        }
    }

    // Read the result size
    let mut result_size = 0;
    let mut shift = 0;
    while cursor < delta.len() {
        let byte = delta[cursor];
        cursor += 1;
        result_size |= ((byte & 0x7F) as usize) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break; // Last byte of the result size encoding
        }
    }

    let mut result = Vec::with_capacity(result_size);
    while cursor < delta.len() {
        let opcode = delta[cursor];
        cursor += 1;

        if opcode & 0x80 != 0 {
            // Copy instruction
            let mut copy_offset = 0;
            let mut copy_size = 0;

            if opcode & 0x01 != 0 {
                copy_offset |= delta[cursor] as usize;
                cursor += 1;
            }
            if opcode & 0x02 != 0 {
                copy_offset |= (delta[cursor] as usize) << 8;
                cursor += 1;
            }
            if opcode & 0x04 != 0 {
                copy_offset |= (delta[cursor] as usize) << 16;
                cursor += 1;
            }
            if opcode & 0x08 != 0 {
                copy_offset |= (delta[cursor] as usize) << 24;
                cursor += 1;
            }
            if opcode & 0x10 != 0 {
                copy_size |= delta[cursor] as usize;
                cursor += 1;
            }
            if opcode & 0x20 != 0 {
                copy_size |= (delta[cursor] as usize) << 8;
                cursor += 1;
            }
            if opcode & 0x40 != 0 {
                copy_size |= (delta[cursor] as usize) << 16;
                cursor += 1;
            }
            if copy_size == 0 {
                copy_size = 0x10000; // A copy size of 0 means 64KB
            }
            if copy_offset + copy_size > base.len() {
                return Err("Copy instruction exceeds base object size".to_string());
            }
            result.extend_from_slice(&base[copy_offset..copy_offset + copy_size]);
        } else if opcode != 0 {
            // Insert instruction
            let insert_size = opcode as usize;
            if cursor + insert_size > delta.len() {
                return Err("Insert instruction exceeds delta size".to_string());
            }
            result.extend_from_slice(&delta[cursor..cursor + insert_size]);
            cursor += insert_size;
        } else {
            return Err("Invalid delta opcode".to_string());
        }
    }

    if result.len() != result_size {
        return Err("Result size does not match expected size".to_string());
    }
    Ok(result)
}

#[allow(dead_code)]
pub struct PackFileStream {
    pub idx: PackIdx,
    pub rev: Option<PackRev>,
    pub mtimes: Option<PackMTimes>,
    pub object_count: usize,
    pub packfile: std::fs::File,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PackObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
    OfsDelta,
    RefDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackObject {
    pub offset: u64,
    pub object_type: PackObjectType,
    pub data: Vec<u8>,
}

impl PackFileStream {
    pub fn from_files(pack_path: &str) -> Result<Self, String> {
        let idx_path = format!("{}.idx", pack_path.trim_end_matches(".pack"));
        let idx_bytes = std::fs::read(idx_path)
            .map_err(|e| format!("Failed to read pack index file: {}", e))?;
        let idx = PackIdx::from_bytes(&idx_bytes)?;

        let rev_path = format!("{}.rev", pack_path.trim_end_matches(".pack"));
        let rev = if std::path::Path::new(&rev_path).exists() {
            let rev_bytes = std::fs::read(rev_path)
                .map_err(|e| format!("Failed to read pack rev file: {}", e))?;
            Some(PackRev::from_bytes(&rev_bytes)?)
        } else {
            None
        };

        let mtimes_path = format!("{}.mtimes", pack_path.trim_end_matches(".pack"));
        let mtimes = if std::path::Path::new(&mtimes_path).exists() {
            let mtimes_bytes = std::fs::read(mtimes_path)
                .map_err(|e| format!("Failed to read pack mtimes file: {}", e))?;
            Some(PackMTimes::from_bytes(&mtimes_bytes)?)
        } else {
            None
        };

        let mut packfile = std::fs::File::open(pack_path)
            .map_err(|e| format!("Failed to open packfile: {}", e))?;
        // Check header of the packfile
        let mut header = [0u8; 12];
        packfile
            .read_exact(&mut header)
            .map_err(|e| format!("Failed to read packfile header: {}", e))?;
        if &header[0..4] != b"PACK" {
            return Err("Invalid packfile signature".into());
        }
        let version = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
        if version != 2 {
            return Err(format!("Unsupported packfile version: {}", version));
        }
        let object_count =
            u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;

        // Verify the packfile checksum matches the checksum in the index file
        // Remember, the packfile contains its checksum as the last 20 bytes
        let mut packfile_checksum = [0u8; 20];
        packfile
            .seek(std::io::SeekFrom::End(-20))
            .map_err(|e| format!("Failed to seek to packfile checksum: {}", e))?;
        packfile
            .read_exact(&mut packfile_checksum)
            .map_err(|e| format!("Failed to read packfile checksum: {}", e))?;

        if packfile_checksum.to_vec() != idx.packfile_checksum {
            return Err("Packfile checksum does not match index file".to_string());
        }

        // If the .rev file exists, verify that the packfile checksum in the .rev file matches the computed checksum of the packfile
        if let Some(rev) = &rev {
            if rev.packfile_checksum != packfile_checksum {
                return Err("Packfile checksum does not match rev file".to_string());
            }
        }

        // If the .mtimes file exists, verify that the packfile checksum in the .mtimes file matches the computed checksum of the packfile
        if let Some(mtimes) = &mtimes {
            // Check which algorithm the mtimes file is using and compute the appropriate checksum of the packfile
            if mtimes.algo == ChecksumAlgorithm::SHA1 {
                if packfile_checksum.to_vec() != mtimes.packfile_checksum {
                    return Err("Packfile checksum does not match mtimes file".to_string());
                }
            } else if mtimes.algo == ChecksumAlgorithm::SHA256 {
                let mut hasher = sha2::Sha256::new();
                let mut packfile_reader = std::io::BufReader::new(&packfile);
                let mut buffer = [0u8; 8192];
                loop {
                    let bytes_read = packfile_reader.read(&mut buffer).map_err(|e| {
                        format!("Failed to read packfile for checksum verification: {}", e)
                    })?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                let computed_checksum = hasher.finalize().to_vec();
                if computed_checksum != mtimes.packfile_checksum {
                    return Err("Packfile checksum does not match mtimes file".to_string());
                }
            } else {
                return Err("Unknown checksum algorithm in mtimes file".to_string());
            }
        }

        Ok(PackFileStream {
            idx,
            rev,
            mtimes,
            object_count,
            packfile,
        })
    }

    pub fn resolve_delta(&mut self, obj: &PackObject) -> Result<Object, String> {
        // First, check if this is an offset delta or a ref delta
        if let PackObjectType::RefDelta = obj.object_type {
            // Get the referenced base object hash from the first 20 bytes of the data
            if obj.data.len() < 20 {
                return Err("Invalid REF_DELTA object: data too short".to_string());
            }
            let base_hash = &obj.data[0..20];
            // Find the base object in the packfile using the hash
            let base_object = self.stream_object(base_hash.try_into().unwrap())?;
            let delta_data = &obj.data[20..];
            let unpacked_data = apply_delta(&base_object.content(), delta_data)?;
            Ok(Object::new(
                base_object.object_type().clone(),
                unpacked_data,
            ))
        } else {
            // We're in an offset delta, so we need to read the offset from the data
            let mut offset = 0;
            let mut shift = 0;
            for byte in &obj.data {
                offset |= ((byte & 0x7F) as usize) << shift;
                shift += 7;
                if byte & 0x80 == 0 {
                    break; // Last byte of the offset encoding
                }
            }
            // Find the base object in the packfile using the offset
            let base_object = self.stream_object_by_offset(offset as u64)?;
            let delta_data = &obj.data[shift / 7..];
            let unpacked_data = apply_delta(&base_object.content(), delta_data)?;
            Ok(Object::new(
                base_object.object_type().clone(),
                unpacked_data,
            ))
        }
    }

    pub fn stream_object_by_offset(&mut self, offset: u64) -> Result<Object, String> {
        self.packfile
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek to object offset: {}", e))?;
        // Read the type and size bytes
        let mut type_and_size = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            self.packfile
                .read_exact(&mut byte)
                .map_err(|e| format!("Failed to read type and size byte: {}", e))?;
            type_and_size.push(byte[0]);
            if byte[0] & 0x80 == 0 {
                break; // Last byte of the type and size encoding
            }
        }

        // Decode the type and size
        let mut size = 0;
        let mut shift = 0;
        for byte in &type_and_size {
            size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break; // Last byte of the size encoding
            }
        }
        let pack_object_type = match type_and_size[0] & 0x7F {
            1 => PackObjectType::Commit,
            2 => PackObjectType::Tree,
            3 => PackObjectType::Blob,
            4 => PackObjectType::Tag,
            6 => PackObjectType::OfsDelta,
            7 => PackObjectType::RefDelta,
            _ => return Err(format!("Unknown object type: {}", type_and_size[0] & 0x7F)),
        };
        let mut data = vec![0u8; size];
        self.packfile
            .read_exact(&mut data)
            .map_err(|e| format!("Failed to read object data: {}", e))?;

        // Need to decompress the data
        let mut decoder = flate2::read::ZlibDecoder::new(data.as_slice());
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)
            .map_err(|e| format!("Failed to decompress object data: {}", e))?;

        let pack_object = PackObject {
            offset,
            object_type: pack_object_type,
            data: decompressed_data,
        };
        if let PackObjectType::OfsDelta | PackObjectType::RefDelta = pack_object.object_type {
            self.resolve_delta(&pack_object)
        } else {
            Ok(pack_object.into())
        }
    }

    pub fn stream_object(&mut self, hash: &[u8; 20]) -> Result<Object, String> {
        let offset = self
            .idx
            .get_object_location(hash)
            .ok_or("Object not found in pack index".to_string())?;
        self.packfile
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek to object offset: {}", e))?;
        // Read the type and size bytes
        let mut type_and_size = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            self.packfile
                .read_exact(&mut byte)
                .map_err(|e| format!("Failed to read type and size byte: {}", e))?;
            type_and_size.push(byte[0]);
            if byte[0] & 0x80 == 0 {
                break; // Last byte of the type and size encoding
            }
        }

        // Decode the type and size
        let (object_type, size) = parse_type_and_size(&type_and_size)?;
        // Unfortunately, the size is not the size of the compressed data, but the size of the uncompressed data, so we need to read the compressed data and then decompress it to get the actual size of the data we need to read.
        // This is a bit tricky, but we can read the compressed data in chunks until we have enough data to decompress and get the actual size of the uncompressed data.
        let mut decompressed_data = Vec::new();
        let mut decoder = flate2::read::ZlibDecoder::new(&mut self.packfile);
        loop {
            let mut buffer = [0u8; 4096];
            let bytes_read = decoder
                .read(&mut buffer)
                .map_err(|e| format!("Failed to read compressed data: {}", e))?;
            if bytes_read == 0 {
                break; // End of compressed data
            }
            decompressed_data.extend_from_slice(&buffer[..bytes_read]);
            if decompressed_data.len() >= size {
                break; // We have read enough data to get the uncompressed size
            }
        }

        if decompressed_data.len() != size {
            return Err(format!(
                "Decompressed data size {} does not match expected size {}",
                decompressed_data.len(),
                size
            ));
        }

        if let PackObjectType::OfsDelta | PackObjectType::RefDelta = object_type {
            let delta_data = decompressed_data;
            let base_object = self.resolve_delta(&PackObject {
                offset,
                object_type,
                data: delta_data,
            })?;
            Ok(base_object)
        } else {
            Ok(Object::new((&object_type).into(), decompressed_data))
        }
    }

    pub fn stream_all_objects(&mut self) -> Result<Vec<Object>, String> {
        let mut objects = Vec::new();
        let all_hashes = self.idx.object_hashes.clone();
        let mut seen_hashes = std::collections::HashSet::new();
        for hash in &all_hashes {
            if seen_hashes.contains(hash) {
                continue; // Skip already seen objects to avoid infinite loops with circular deltas
            }
            let object = self.stream_object(hash)?;
            seen_hashes.insert(*hash);
            objects.push(object);
        }
        Ok(objects)
    }

    pub fn count_objects(&self) -> usize {
        self.object_count
    }
}

pub struct PackFile {
    pub object_count: usize,
    pub objects: Vec<PackObject>,
}

impl PackFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = std::io::Cursor::new(bytes);
        let mut header = [0u8; 12];
        cursor
            .read_exact(&mut header)
            .map_err(|e| format!("Failed to read packfile header: {}", e))?;
        if &header[0..4] != b"PACK" {
            return Err("Invalid packfile signature".into());
        }
        let version = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
        if version != 2 {
            return Err(format!("Unsupported packfile version: {}", version));
        }
        let object_count =
            u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;

        let mut objects = Vec::with_capacity(object_count);
        for _ in 0..object_count {
            let cursor_start_pos = cursor.position();
            // Read the type and size bytes
            let mut type_and_size = Vec::new();
            loop {
                let mut byte = [0u8; 1];
                cursor
                    .read_exact(&mut byte)
                    .map_err(|e| format!("Failed to read type and size byte: {}", e))?;
                type_and_size.push(byte[0]);
                if byte[0] & 0x80 == 0 {
                    break; // Last byte of the type and size encoding
                }
            }

            // Decode the type and size
            let (object_type, size) = parse_type_and_size(&type_and_size)?;

            let mut decompressed_data;
            if object_type == PackObjectType::RefDelta {
                // First 20 bytes of the data are the base object hash, so we need to read those first before we can read the compressed delta data
                let mut base_hash = [0u8; 20];
                cursor.read_exact(&mut base_hash)
                    .map_err(|e| format!("Failed to read base object hash for REF_DELTA: {}", e))?;
                // Now we can read the compressed delta data
                let mut compressed_delta_data = vec![0u8; size];
                let mut decoder = flate2::bufread::ZlibDecoder::new(&mut cursor);
                decoder.read_exact(&mut compressed_delta_data)
                    .map_err(|e| format!("Failed to read compressed delta data for REF_DELTA: {}", e))?;
                if compressed_delta_data.len() != size {
                    return Err(format!(
                        "Decompressed delta data size {} does not match expected size {} for REF_DELTA",
                        compressed_delta_data.len(),
                        size
                    ));
                }
                let mut delta_data = Vec::with_capacity(20 + compressed_delta_data.len());
                delta_data.extend_from_slice(&base_hash);
                delta_data.extend_from_slice(&compressed_delta_data);
                decompressed_data = delta_data;
            } else if object_type == PackObjectType::OfsDelta {
                // For offset deltas, we need to pull the n-byte offset from the data first, and then read the compressed delta data after that
                let mut offset_bytes = Vec::new();
                loop {
                    let mut byte = [0u8; 1];
                    cursor.read_exact(&mut byte)
                        .map_err(|e| format!("Failed to read offset byte for OFS_DELTA: {}", e))?;
                    offset_bytes.push(byte[0]);
                    if byte[0] & 0x80 == 0 {
                        break; // Last byte of the offset encoding
                    }
                }
                // Now we can read the compressed delta data                
                let mut compressed_delta_data = vec![0u8; size];
                let mut decoder = flate2::bufread::ZlibDecoder::new(&mut cursor);
                decoder.read_exact(&mut compressed_delta_data)
                    .map_err(|e| format!("Failed to read compressed delta data for OFS_DELTA: {}", e))?;
                if compressed_delta_data.len() != size {
                    return Err(format!(
                        "Decompressed delta data size {} does not match expected size {} for OFS_DELTA",
                        compressed_delta_data.len(),
                        size
                    ));
                }
                let mut delta_data = Vec::with_capacity(offset_bytes.len() + compressed_delta_data.len());
                delta_data.extend_from_slice(&offset_bytes);
                delta_data.extend_from_slice(&compressed_delta_data);
                decompressed_data = delta_data;
            } else {
                // For non-delta objects, we can just read the compressed data directly
                decompressed_data = vec![0u8; size];
                let mut decoder = flate2::bufread::ZlibDecoder::new(&mut cursor);

                // Edge case: What if the expected size is zero?
                if size == 0 {
                    // Do a dummy read to advance the cursor past the compressed data.
                    let mut dummy = [0u8; 1];
                    decoder.read(&mut dummy)
                        .map_err(|e| format!("Failed to read compressed data for zero-size object: {}", e))?;
                } else {
                    decoder
                        .read_exact(&mut decompressed_data)
                        .map_err(|e| format!("Failed to read compressed data: {}", e))?;

                    if decompressed_data.len() != size {
                        return Err(format!(
                            "Decompressed data size {} does not match expected size {}",
                            decompressed_data.len(),
                            size
                        ));
                    }
                }
            }

            let obj = PackObject {
                offset: cursor_start_pos,
                object_type,
                data: decompressed_data,
            };

            objects.push(obj);
        }

        Ok(PackFile {
            object_count,
            objects,
        })
    }

    fn unpack_and_cache_object(
        &self,
        obj: &PackObject, 
        cache: &mut HashMap<Vec<u8>, PackObject>
    ) -> Result<PackObject, String> {
        match obj.object_type {
            PackObjectType::OfsDelta => {
                let (offset, bytes_consumed) = offset_decode(&obj.data);
                let pack_obj_offset = obj.offset as usize - offset as usize;
                let base_pack_obj = self.objects.iter()
                    .find(|&o| o.offset as usize == pack_obj_offset)
                    .ok_or_else(|| "Base object missing from packfile stream".to_string())?;
                
                // Recursively resolve the base if it's also a delta
                let resolved_base = self.unpack_and_cache_object(&base_pack_obj, cache)?;
                
                let delta_data = &obj.data[bytes_consumed..];
                let unpacked_data = apply_delta(&resolved_base.data, delta_data)?;
                
                let final_obj = PackObject {
                    offset: obj.offset,
                    object_type: resolved_base.object_type,
                    data: unpacked_data,
                };
                // Cache this newly resolved object's hash so future deltas can target it instantly
                let final_as_obj: Object = (&final_obj).into();
                cache.insert(final_as_obj.get_hash(), final_obj.clone());
                
                Ok(final_obj)
            },
            PackObjectType::RefDelta => {
                let base_hash = &obj.data[0..20].to_vec();
                
                // Look up the base directly from the cache! No linear searching!
                let resolved_base = match cache.get(base_hash) {
                    Some(cached_obj) => cached_obj.clone(),
                    None => {
                        // If it's not in the cache, it means the base is a delta later in the packfile 
                        // (rare but possible). Find it once, resolve it, and cache it.
                        let base_pack_obj = self.objects.iter()
                            .find(|&o| o.object_type != PackObjectType::OfsDelta 
                                && o.object_type != PackObjectType::RefDelta 
                                && Into::<Object>::into(o).get_hash() == *base_hash)
                            // Or find the delta matching it if needed...
                            .ok_or_else(|| "Base object missing from packfile stream".to_string())?;
                        
                        self.unpack_and_cache_object(base_pack_obj, cache)?
                    }
                };

                let delta_data = &obj.data[20..];
                let unpacked_data = apply_delta(&resolved_base.data, delta_data)?;
                
                let final_obj = PackObject {
                    offset: obj.offset,
                    object_type: resolved_base.object_type,
                    data: unpacked_data,
                };

                // Cache this newly resolved object's hash so future deltas can target it instantly
                let final_as_obj: Object = (&final_obj).into();
                cache.insert(final_as_obj.get_hash(), final_obj.clone());

                Ok(final_obj)
            },
            _ => Ok(obj.clone()), // Base objects are already handled
        }
    }

    pub fn unpack_objects(&self) -> Result<Vec<Object>, String> {
        let mut resolved_cache: HashMap<Vec<u8>, PackObject> = HashMap::new();
        let mut unpacked_objects = Vec::with_capacity(self.object_count);

        // First, seed the cache with all non-delta base objects
        for pack_object in &self.objects {
            if pack_object.object_type != PackObjectType::OfsDelta && pack_object.object_type != PackObjectType::RefDelta {
                let obj: Object = pack_object.into();
                resolved_cache.insert(obj.get_hash(), pack_object.clone());
            }
        }

        // 2. Now resolve the deltas using the cache
        for pack_object in &self.objects {
            let resolved = self.unpack_and_cache_object(pack_object, &mut resolved_cache)?;
            unpacked_objects.push(resolved.into());
        }

        Ok(unpacked_objects)
    }
}

impl PackObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackObjectType::Commit => "commit",
            PackObjectType::Tree => "tree",
            PackObjectType::Blob => "blob",
            PackObjectType::Tag => "tag",
            PackObjectType::OfsDelta => "ofs_delta",
            PackObjectType::RefDelta => "ref_delta",
        }
    }
}

impl Into<ObjectType> for &PackObjectType {
    fn into(self) -> ObjectType {
        match self {
            PackObjectType::Commit => ObjectType::Commit,
            PackObjectType::Tree => ObjectType::Tree,
            PackObjectType::Blob => ObjectType::Blob,
            PackObjectType::Tag => ObjectType::Tag,
            PackObjectType::OfsDelta | PackObjectType::RefDelta => {
                panic!("Delta objects cannot be directly converted to ObjectType")
            }
        }
    }
}

impl PackObject {
    pub fn pretty_print(&self) -> String {
        format!(
            "PackObject of type {} with data size {}",
            self.object_type.as_str(),
            self.data.len()
        )
    }

    pub fn unpack(&self, base_object: &Object) -> Result<Object, String> {
        if let PackObjectType::OfsDelta | PackObjectType::RefDelta = self.object_type {
            let delta_data = &self.data;
            let unpacked_data = apply_delta(base_object.content(), delta_data)?;
            Ok(Object::new(
                base_object.object_type().clone(),
                unpacked_data,
            ))
        } else {
            Err("Object is not a delta".to_string())
        }
    }
}

impl Into<Object> for &PackObject {
    fn into(self) -> Object {
        if let PackObjectType::OfsDelta | PackObjectType::RefDelta = self.object_type {
            panic!("Delta objects cannot be directly converted to Object");
        }
        let object_type: ObjectType = (&self.object_type).into();
        Object::new(object_type, self.data.clone()) // Data is already decompressed when we read it from the packfile, so we can just use it directly to create the Object
    }
}

impl Into<Object> for PackObject {
    fn into(self) -> Object {
        if let PackObjectType::OfsDelta | PackObjectType::RefDelta = self.object_type {
            panic!("Delta objects cannot be directly converted to Object");
        }
        let object_type: ObjectType = (&self.object_type).into();
        
        Object::new(object_type, self.data) // Data is already decompressed when we read it from the packfile, so we can just use it directly to create the Object
    }
}

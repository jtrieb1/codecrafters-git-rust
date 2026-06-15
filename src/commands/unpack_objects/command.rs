use super::{errors::UnpackObjectsError, input::UnpackObjectsInput};
use crate::shared::pack::PackFileStream;

pub fn unpack_objects(input: UnpackObjectsInput) -> Result<String, UnpackObjectsError> {
    let mut pkfilestream = PackFileStream::from_files(&input.packfile_path)
        .map_err(|e| UnpackObjectsError::PackfileReadError(e.to_string()))?;

    if input.dry_run {
        let object_count = pkfilestream.count_objects();
        return Ok(format!(
            "Dry run: would unpack {} objects from packfile {}",
            object_count, input.packfile_path
        ));
    }

    let objects = pkfilestream
        .stream_all_objects()
        .map_err(|e| UnpackObjectsError::UnpackError(e.to_string()))?;

    for obj in &objects {
        obj.persist()
            .map_err(|e| UnpackObjectsError::IoError(format!("Failed to persist object: {}", e)))?;
    }

    Ok(format!(
        "Unpacked {} objects from packfile {}",
        objects.len(),
        input.packfile_path
    ))
}

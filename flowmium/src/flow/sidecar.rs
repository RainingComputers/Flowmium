use s3::Bucket;
use std::fs;

use super::errors::FlowError;

pub async fn download_input(
    bucket: Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), FlowError> {
    let Ok(response) = bucket.get_object(store_path).await else {
        return Err(FlowError::UnableToDownloadInput);
    };

    if response.status_code() != 200 {
        return Err(FlowError::UnableToDownloadInput);
    }

    if let std::io::Result::Err(_) = fs::write(local_path, &response.bytes()) {
        return Err(FlowError::UnableToWriteInput);
    }

    return Ok(());
}

pub async fn upload_output(
    bucket: Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), FlowError> {
    let Ok(content) = fs::read(local_path) else {
        return Err(FlowError::UnableToReadOutput);
    };

    let Ok(response) = bucket.put_object(store_path, &content).await else {
        return Err(FlowError::UnableToUploadArtifact);
    };

    if response.status_code() != 200 {
        return Err(FlowError::UnableToSpawnTaskError);
    }

    return Ok(());
}

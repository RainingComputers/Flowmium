use s3::Bucket;

use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

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

    if let std::io::Result::Err(_) = fs::write(local_path, &response.bytes()).await {
        return Err(FlowError::UnableToWriteInput);
    }

    return Ok(());
}

pub async fn upload_output(
    bucket: Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), FlowError> {
    let Ok(content) = fs::read(local_path).await else {
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

async fn wait_for_file(path: &PathBuf, timeout: Duration) -> bool {
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout {
        if fs::metadata(path).await.is_ok() {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    false
}

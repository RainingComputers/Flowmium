use s3::Bucket;

use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

use super::errors::SidecarError;
use super::model::{Input, Output};

pub async fn download_input(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), SidecarError> {
    let Ok(response) = bucket.get_object(store_path).await else {
        return Err(SidecarError::UnableToDownloadInput);
    };

    if response.status_code() != 200 {
        return Err(SidecarError::UnableToDownloadInput);
    }

    if let std::io::Result::Err(_) = fs::write(local_path, &response.bytes()).await {
        return Err(SidecarError::UnableToWriteInput);
    }

    return Ok(());
}

pub async fn upload_output(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), SidecarError> {
    let Ok(content) = fs::read(local_path).await else {
        return Err(SidecarError::UnableToReadOutput);
    };

    let Ok(response) = bucket.put_object(store_path, &content).await else {
        return Err(SidecarError::UnableToUploadArtifact);
    };

    if response.status_code() != 200 {
        return Err(SidecarError::UnableToUploadArtifact);
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

fn get_store_path(flow_id: usize, output_name: &str) -> String {
    return flow_id.to_string() + "/" + output_name;
}

async fn download_all_inputs(
    bucket: Bucket,
    flow_id: usize,
    inputs: Vec<Input>,
) -> Result<(), SidecarError> {
    for input in inputs {
        let store_path = get_store_path(flow_id, &input.from);

        // TODO: Log on error
        download_input(&bucket, input.path, store_path).await?;
    }

    return Ok(());
}

async fn upload_all_output(
    bucket: Bucket,
    flow_id: usize,
    outputs: Vec<Output>,
) -> Result<(), SidecarError> {
    for output in outputs {
        let store_path = get_store_path(flow_id, &output.name);

        // TODO: Log on error
        upload_output(&bucket, output.path, store_path).await?;
    }

    return Ok(());
}

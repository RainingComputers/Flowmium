use s3::Bucket;

use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

use super::errors::SidecarError;
use super::model::{Input, Output};

#[tracing::instrument(skip(bucket))]
pub async fn download_input(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), SidecarError> {
    let response = match bucket.get_object(store_path).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!("Could not download input because {}", error);
            return Err(SidecarError::UnableToDownloadInput);
        }
    };

    if response.status_code() != 200 {
        tracing::error!(
            "Response was non ok code {} while downloading input",
            response.status_code()
        );
        return Err(SidecarError::UnableToDownloadInput);
    }

    if let std::io::Result::Err(error) = fs::write(local_path, &response.bytes()).await {
        tracing::error!(%error, "File error while downloading input");
        return Err(SidecarError::UnableToWriteInput);
    }

    return Ok(());
}

#[tracing::instrument(skip(bucket))]
pub async fn upload_output(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), SidecarError> {
    let content = match fs::read(local_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(%error, "File error while uploading output");
            return Err(SidecarError::UnableToReadOutput);
        }
    };

    let response = match bucket.put_object(store_path, &content).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!("Could not upload output because {}", error);
            return Err(SidecarError::UnableToUploadArtifact);
        }
    };

    if response.status_code() != 200 {
        tracing::error!(
            "Response was non ok code {} while uploading output",
            response.status_code()
        );
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
        upload_output(&bucket, output.path, store_path).await?;
    }

    return Ok(());
}

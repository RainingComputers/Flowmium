use s3::{creds::Credentials, Bucket, Region};
use tokio::fs;

use super::errors::ArtefactError;

pub fn get_bucket(
    access_key: &String,
    secret_key: &String,
    bucket_name: &String,
    store_url: String,
) -> Result<Bucket, ArtefactError> {
    let bucket_creds =
        match Credentials::new(Some(&access_key), Some(&secret_key), None, None, None) {
            Ok(creds) => creds,
            Err(error) => {
                tracing::error!(%error, "Unable to create creds for bucket");
                return Err(ArtefactError::UnableToOpenBucketError);
            }
        };

    let bucket_region = Region::Custom {
        region: "custom".to_owned(),
        endpoint: store_url,
    };

    let bucket = match Bucket::new(&bucket_name, bucket_region, bucket_creds) {
        Ok(bucket) => bucket.with_path_style(),
        Err(error) => {
            tracing::error!(%error, "Unable to open bucket");
            return Err(ArtefactError::UnableToOpenBucketError);
        }
    };

    return Ok(bucket);
}

#[tracing::instrument(skip(bucket))]
pub async fn download_input(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), ArtefactError> {
    let response = match bucket.get_object(store_path).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "Could not download input");
            return Err(ArtefactError::UnableToDownloadInput);
        }
    };

    if response.status_code() != 200 {
        tracing::error!(
            "Response was non ok code {} while downloading input",
            response.status_code()
        );
        return Err(ArtefactError::UnableToDownloadInput);
    }

    if let std::io::Result::Err(error) = fs::write(local_path, &response.bytes()).await {
        tracing::error!(%error, "File error while downloading input");
        return Err(ArtefactError::UnableToWriteInput);
    }

    return Ok(());
}

#[tracing::instrument(skip(bucket))]
pub async fn upload_output(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), ArtefactError> {
    let content = match fs::read(local_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(%error, "File error while uploading output");
            return Err(ArtefactError::UnableToReadOutput);
        }
    };

    let response = match bucket.put_object(store_path, &content).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "Could not upload output");
            return Err(ArtefactError::UnableToUploadArtifact);
        }
    };

    if response.status_code() != 200 {
        tracing::error!(
            "Response was non ok code {} while uploading output",
            response.status_code()
        );
        return Err(ArtefactError::UnableToUploadArtifact);
    }

    return Ok(());
}

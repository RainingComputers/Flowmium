use s3::{creds::Credentials, request_trait::ResponseData, Bucket, Region};

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
                return Err(ArtefactError::UnableToOpenBucket(
                    s3::error::S3Error::Credentials(error),
                ));
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
            return Err(ArtefactError::UnableToOpenBucket(error));
        }
    };

    return Ok(bucket);
}

pub async fn create_parent_directories(local_path: &String) -> tokio::io::Result<()> {
    let path = std::path::Path::new(&local_path);
    let prefix = match path.parent() {
        Some(parent) => parent,
        None => {
            return Ok(());
        }
    };

    tokio::fs::create_dir_all(prefix).await
}

pub async fn get_artefact(
    bucket: &Bucket,
    store_path: String,
) -> Result<ResponseData, ArtefactError> {
    let response = match bucket.get_object(&store_path).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "Could not download artefact");
            return Err(ArtefactError::UnableToDownloadInput(error));
        }
    };

    let status_code = response.status_code();

    if status_code == 404 {
        tracing::error!("Got 404 response while downloading artefact");
        return Err(ArtefactError::ArtefactDoesNotExist(store_path));
    }

    if status_code != 200 {
        tracing::error!(
            "Response was non ok code {} while downloading artefact",
            status_code
        );
        return Err(ArtefactError::UnableToDownloadInputApi(status_code));
    }

    return Ok(response);
}

#[tracing::instrument(skip(bucket))]
pub async fn download_input(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), ArtefactError> {
    tracing::info!("Downloading input");

    let response = get_artefact(bucket, store_path).await?;

    if let Err(error) = create_parent_directories(&local_path).await {
        tracing::error!(%error, "Unable to create parent directories for input");
        return Err(ArtefactError::UnableToWriteInput(error));
    }

    if let std::io::Result::Err(error) = tokio::fs::write(local_path, &response.bytes()).await {
        tracing::error!(%error, "File error while downloading input");
        return Err(ArtefactError::UnableToWriteInput(error));
    }

    return Ok(());
}

#[tracing::instrument(skip(bucket))]
pub async fn upload_output(
    bucket: &Bucket,
    local_path: String,
    store_path: String,
) -> Result<(), ArtefactError> {
    tracing::info!("Uploading output");

    let content = match tokio::fs::read(local_path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(%error, "File error while uploading output");
            return Err(ArtefactError::UnableToReadOutput(error));
        }
    };

    let response = match bucket.put_object(store_path, &content).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "Could not upload output");
            return Err(ArtefactError::UnableToUploadArtifact(error));
        }
    };

    let status_cost = response.status_code();

    if status_cost != 200 {
        tracing::error!(
            "Response was non ok code {} while uploading output",
            status_cost
        );
        return Err(ArtefactError::UnableToUploadArtifactApi(status_cost));
    }

    return Ok(());
}

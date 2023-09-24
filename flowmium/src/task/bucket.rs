use s3::{creds::Credentials, request_trait::ResponseData, Bucket, BucketConfiguration, Region};

use super::errors::ArtefactError;

pub async fn bucket_exists(bucket: &Bucket) -> Result<bool, ArtefactError> {
    match bucket
        .list_page("/".to_string(), None, None, None, None)
        .await
    {
        Ok(_) => Ok(true),
        Err(error) => match error {
            s3::error::S3Error::Http(404, _) => Ok(false),
            _ => {
                tracing::error!(%error, "Unable to check if bucket exists");
                Err(ArtefactError::UnableToCheckExistence(error))
            }
        },
    }
}

#[tracing::instrument(skip(access_key, secret_key))]
pub async fn get_bucket(
    access_key: &str,
    secret_key: &str,
    bucket_name: &str,
    store_url: String,
) -> Result<Bucket, ArtefactError> {
    let bucket_creds = match Credentials::new(Some(access_key), Some(secret_key), None, None, None)
    {
        Ok(creds) => creds,
        Err(error) => {
            tracing::error!(%error, "Unable to create creds for bucket");
            return Err(ArtefactError::UnableToExistingOpenBucket(
                s3::error::S3Error::Credentials(error),
            ));
        }
    };

    let bucket_region = Region::Custom {
        region: "custom".to_owned(),
        endpoint: store_url,
    };

    let bucket = match Bucket::new(bucket_name, bucket_region.clone(), bucket_creds.clone()) {
        Ok(bucket) => bucket.with_path_style(),
        Err(error) => {
            tracing::error!(%error, "Unable to open bucket");
            return Err(ArtefactError::UnableToExistingOpenBucket(error));
        }
    };

    match bucket_exists(&bucket).await? {
        true => {
            tracing::info!("Using existing bucket");
            Ok(bucket)
        }
        false => match Bucket::create_with_path_style(
            bucket_name,
            bucket_region,
            bucket_creds,
            BucketConfiguration::public(),
        )
        .await
        {
            Ok(response) => match response.success() {
                true => {
                    tracing::info!("Created a new bucket");
                    Ok(response.bucket)
                }
                false => {
                    tracing::error!(
                        "Unable to create bucket, got failure response {}",
                        response.response_text
                    );
                    Err(ArtefactError::UnableToCreateBucketFailResponse(
                        response.response_text,
                    ))
                }
            },
            Err(error) => {
                tracing::error!(%error, "Unable to create bucket");
                Err(ArtefactError::UnableToCreateBucket(error))
            }
        },
    }
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
        Err(error) => match error {
            s3::error::S3Error::Http(404, _) => {
                tracing::error!("Got 404 response while downloading artefact");
                return Err(ArtefactError::ArtefactDoesNotExist(store_path));
            }
            error => {
                tracing::error!(%error, "Could not download artefact");
                return Err(ArtefactError::UnableToDownloadInput(error));
            }
        },
    };

    let status_code = response.status_code();

    if status_code != 200 {
        tracing::error!(
            "Response was non ok code {} while downloading artefact",
            status_code
        );
        return Err(ArtefactError::UnableToDownloadInputApi(status_code));
    }

    Ok(response)
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

    Ok(())
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

    Ok(())
}

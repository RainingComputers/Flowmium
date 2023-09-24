use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArtefactError {
    #[error("unable to upload artefact: {0}")]
    UnableToUploadArtifact(s3::error::S3Error),
    #[error("unable to read output: {0}")]
    UnableToReadOutput(std::io::Error),
    #[error("unable to download input: {0}")]
    UnableToDownloadInput(s3::error::S3Error),
    #[error("unable to download input api errored with status {0}")]
    UnableToDownloadInputApi(u16),
    #[error("unable to write input: {0}")]
    UnableToWriteInput(std::io::Error),
    #[error("unable to check for existence of bucket: {0}")]
    UnableToCheckExistence(s3::error::S3Error),
    #[error("unable to create bucket: {0}")]
    UnableToCreateBucket(s3::error::S3Error),
    #[error("unable to create bucket response was not ok: {0}")]
    UnableToCreateBucketFailResponse(String),
    #[error("unable to open bucket: {0}")]
    UnableToExistingOpenBucket(s3::error::S3Error),
    #[error("unable to upload output api errored with status {0}")]
    UnableToUploadArtifactApi(u16),
    #[error("artefact {0} does not exist")]
    ArtefactDoesNotExist(String),
}

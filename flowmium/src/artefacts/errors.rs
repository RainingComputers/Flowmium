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
    UnableToDownloadInputApiError(u16),
    #[error("unable to write input: {0}")]
    UnableToWriteInput(std::io::Error),
    #[error("unable to open bucket: {0}")]
    UnableToOpenBucketError(s3::error::S3Error),
    #[error("unable to upload output api errored with status {0}")]
    UnableToUploadArtifactApiError(u16),
    #[error("artefact {0} does not exist")]
    ArtefactDoesNotExistError(String),
}

#[derive(Debug, PartialEq)]
pub enum ArtefactError {
    UnableToUploadArtifact,
    UnableToReadOutput,
    UnableToDownloadInput,
    UnableToWriteInput,
    UnableToOpenBucketError,
}

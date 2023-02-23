use std::fmt;

#[derive(Debug, PartialEq)]
pub enum FlowError {
    CyclicDependenciesError,
    DependentTaskDoesNotExistError,
    FlowDoesNotExistError,
    StageDoesNotExistError,
    UnableToSpawnTaskError,
    UnableToConnectToKubernetesError,
    UnexpectedRunnerStateError,
    OutputDoesNotExistError,
    OutputNotFromParentError,
    OutputNotUniqueError,
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq)]
pub enum SidecarError {
    UnableToUploadArtifact,
    UnableToReadOutput,
    UnableToDownloadInput,
    UnableToWriteInput,
}

impl fmt::Display for SidecarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

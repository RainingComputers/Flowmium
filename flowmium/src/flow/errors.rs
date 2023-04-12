use std::fmt;

#[derive(Debug, PartialEq)]
pub enum FlowError {
    CyclicDependenciesError,
    DependentTaskDoesNotExistError,
    UnableToSpawnTaskError,
    UnableToConnectToKubernetesError,
    UnexpectedRunnerStateError,
    OutputDoesNotExistError,
    OutputNotFromParentError,
    OutputNotUniqueError,
    InvalidTaskDefinitionError,
    InvalidStoredValueError,
    StoreError,
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

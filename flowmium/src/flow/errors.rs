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
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{:?}", self)
    }
}

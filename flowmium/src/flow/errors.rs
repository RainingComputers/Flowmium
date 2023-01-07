#[derive(PartialEq, Debug)]
pub enum FlowError {
    CyclicDependenciesError,
    DependentTaskDoesNotExistError,
    FlowDoesNotExistError,
    StageDoesNotExistError,
    TaskDoesNotExistError,
    UnableToSpawnTaskError,
    CorruptedTaskError,
}

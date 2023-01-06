#[derive(PartialEq, Debug)]
pub enum FlowError {
    CyclicDependenciesError,
    DependentTaskDoesNotExistError,
    FlowDoesNotExist,
    StageDoesNotExist,
    TaskDoesNotExist,
}

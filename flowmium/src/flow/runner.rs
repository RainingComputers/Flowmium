use super::model::Task;

pub struct KubernetesRunner {}

pub struct RunnerStatus {
    pub pending: Vec<(usize, usize)>,
    pub running: Vec<(usize, usize)>,
    pub finished: Vec<(usize, usize)>,
    pub failed: Vec<(usize, usize)>,
}

impl KubernetesRunner {
    pub fn run_tasks(&self, tasks: Vec<(usize, &Task)>) {}
    pub fn status(&self) -> RunnerStatus {
        return RunnerStatus {
            pending: vec![],
            running: vec![],
            finished: vec![],
            failed: vec![],
        };
    }
}

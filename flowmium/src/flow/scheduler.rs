use std::collections::BTreeSet;

use super::{errors::FlowError, model::Task};

#[derive(Debug)]
enum FlowStatus {
    Running,
    Success,
    Failed,
}

#[derive(Debug)]
pub struct FlowState {
    id: usize,
    plan: Vec<BTreeSet<usize>>,
    current_stage: usize,
    running_tasks: BTreeSet<usize>,
    finished_tasks: BTreeSet<usize>,
    failed_tasks: BTreeSet<usize>,
    task_definitions: Vec<Task>,
    flow_name: String,
    status: FlowStatus,
}

impl FlowState {
    fn create_flow_state(
        id: usize,
        flow_name: String,
        plan: Vec<BTreeSet<usize>>,
        task_definitions: Vec<Task>,
    ) -> FlowState {
        FlowState {
            id,
            plan,
            current_stage: 0,
            running_tasks: BTreeSet::new(),
            finished_tasks: BTreeSet::new(),
            failed_tasks: BTreeSet::new(),
            task_definitions,
            flow_name,
            status: FlowStatus::Running,
        }
    }
}

#[derive(Debug)]
pub struct Scheduler {
    pub flow_runs: Vec<FlowState>,
}

impl Scheduler {
    pub fn create_flow(
        &mut self,
        flow_name: String,
        plan: Vec<BTreeSet<usize>>,
        task_definitions: Vec<Task>,
    ) -> (usize, Vec<(usize, &Task)>) {
        let id = self.flow_runs.len();

        self.flow_runs.push(FlowState::create_flow_state(
            id,
            flow_name,
            plan,
            task_definitions,
        ));

        let flow = self.flow_runs.last_mut().unwrap();

        let tasks = Scheduler::stage_to_tasks(&flow.plan[0], &flow.task_definitions);

        return (id, tasks);
    }

    pub fn mark_task_running(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        (*flow).running_tasks.insert(task_id);

        return Ok(());
    }

    fn stage_to_tasks<'a>(
        stage: &'a BTreeSet<usize>,
        task_definitions: &'a Vec<Task>,
    ) -> Vec<(usize, &'a Task)> {
        stage
            .iter()
            .map(|id| (*id, &task_definitions[*id]))
            .collect()
    }

    pub fn schedule_next_stage(
        &mut self,
        flow_id: usize,
    ) -> Result<Option<Vec<(usize, &Task)>>, FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        let Some(stage) = flow.plan.get(flow.current_stage) else {
            return Err(FlowError::StageDoesNotExistError);
        };

        if !stage.is_subset(&flow.finished_tasks) {
            return Ok(None);
        }

        (*flow).current_stage += 1;
        let Some(next_stage) = flow.plan.get(flow.current_stage) else {
            return Ok(None);
        };

        let tasks = Scheduler::stage_to_tasks(next_stage, &flow.task_definitions);

        return Ok(Some(tasks));
    }

    pub fn mark_task_finished(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        (*flow).running_tasks.remove(&task_id);
        (*flow).finished_tasks.insert(task_id);

        if flow.finished_tasks.len() == flow.task_definitions.len() {
            flow.status = FlowStatus::Success;
        }

        return Ok(());
    }

    pub fn mark_task_failed(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        (*flow).running_tasks.remove(&task_id);
        (*flow).failed_tasks.insert(task_id);

        (*flow).status = FlowStatus::Failed;

        return Ok(());
    }
}

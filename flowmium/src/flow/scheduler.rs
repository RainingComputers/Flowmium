use std::collections::{BTreeSet};

use super::{errors::FlowError, model::Task};

#[derive(Debug, PartialEq)]
enum FlowStatus {
    Pending,
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
            status: FlowStatus::Pending,
        }
    }

    pub fn running_tasks(&self) -> BTreeSet<usize> {
        return self.running_tasks.clone();
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
    ) -> usize {
        let id = self.flow_runs.len();

        self.flow_runs.push(FlowState::create_flow_state(
            id,
            flow_name,
            plan,
            task_definitions,
        ));

        return id;
    }

    pub fn mark_task_running(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        (*flow).running_tasks.insert(task_id);

        return Ok(());
    }

    fn stage_to_tasks(stage: &BTreeSet<usize>, task_definitions: &Vec<Task>) -> Vec<(usize, Task)> {
        stage
            .iter()
            .map(|id| (*id, task_definitions[*id].clone()))
            .collect()
    }

    pub fn schedule_tasks(
        &mut self,
        flow_id: usize,
    ) -> Result<Option<Vec<(usize, Task)>>, FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExistError);
        };

        if flow.status != FlowStatus::Running && flow.status != FlowStatus::Pending {
            return Ok(None);
        }

        let Some(stage) = flow.plan.get(flow.current_stage) else {
            return Err(FlowError::StageDoesNotExistError);
        };

        if !stage.is_subset(&flow.finished_tasks) && flow.status == FlowStatus::Running {
            return Ok(None);
        }

        if flow.status == FlowStatus::Pending {
            (*flow).status = FlowStatus::Running;
        } else {
            (*flow).current_stage += 1;
        }

        let Some(next_stage) = flow.plan.get(flow.current_stage) else {
            return Err(FlowError::StageDoesNotExistError);
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

    pub fn get_running_or_pending_flows(&self) -> Vec<(usize, BTreeSet<usize>)> {
        return self
            .flow_runs
            .iter()
            .filter(|flow| flow.status == FlowStatus::Running || flow.status == FlowStatus::Pending)
            .map(|flow| (flow.id, flow.running_tasks()))
            .collect();
    }
}

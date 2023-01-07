use std::collections::BTreeSet;

use super::{errors::FlowError, model::Task};

enum FlowStatus {
    Running,
    Success,
    Failed,
}

// TODO: Add task status
struct FlowState {
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

pub struct Scheduler {
    flow_runs: Vec<FlowState>,
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
            return Err(FlowError::FlowDoesNotExist);
        };

        flow.running_tasks.insert(task_id);

        return Ok(());
    }

    pub fn schedule_next_stage(
        &self,
        flow_id: usize,
    ) -> Result<Option<Vec<(usize, &Task)>>, FlowError> {
        let Some(flow) = self.flow_runs.get(flow_id) else {
            return Err(FlowError::FlowDoesNotExist);
        };

        let Some(stage) = flow.plan.get(flow.current_stage) else {
            return Err(FlowError::StageDoesNotExist);
        };

        if !flow.finished_tasks.is_disjoint(&stage) {
            return Ok(None);
        }

        let Some(next_stage) = flow.plan.get(flow.current_stage+1) else {
            return Ok(None);
        };

        let tasks = next_stage
            .iter()
            .map(|id| (*id, &flow.task_definitions[*id]))
            .collect();

        return Ok(Some(tasks));
    }

    pub fn mark_task_finished(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExist);
        };

        flow.running_tasks.remove(&task_id);
        flow.finished_tasks.insert(task_id);

        if flow.finished_tasks.len() == flow.task_definitions.len() {
            flow.status = FlowStatus::Success;
        }

        return Ok(());
    }

    pub fn mark_task_failed(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExist);
        };

        flow.running_tasks.remove(&task_id);
        flow.failed_tasks.insert(task_id);

        flow.status = FlowStatus::Failed;

        return Ok(());
    }

    pub fn mark_flow_failed(&mut self, flow_id: usize) -> Result<(), FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(FlowError::FlowDoesNotExist);
        };

        flow.status = FlowStatus::Failed;

        return Ok(());
    }
}

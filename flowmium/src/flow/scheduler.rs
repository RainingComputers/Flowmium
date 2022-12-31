use super::model::Task;

enum FlowStatus {
    Running,
    Success,
    Failed,
}

struct FlowState {
    id: usize,
    plan: Vec<Vec<usize>>,
    current_stage: usize,
    running_tasks: Vec<usize>,
    finished_tasks: Vec<usize>,
    failed_tasks: Vec<usize>,
    task_definitions: Vec<Task>,
    flow_name: String,
    status: FlowStatus,
}

impl FlowState {
    fn create_flow_state(
        id: usize,
        flow_name: String,
        plan: Vec<Vec<usize>>,
        task_definitions: Vec<Task>,
    ) -> FlowState {
        FlowState {
            id,
            plan,
            current_stage: 0,
            running_tasks: vec![],
            finished_tasks: vec![],
            failed_tasks: vec![],
            task_definitions,
            flow_name,
            status: FlowStatus::Running,
        }
    }
}

enum SchedulerError {
    FlowDoesNotExist,
    StageDoesNotExist,
    TaskDoesNotExist,
}

struct Scheduler {
    flow_runs: Vec<FlowState>,
}

impl Scheduler {
    fn create_flow(
        &mut self,
        flow_name: String,
        plan: Vec<Vec<usize>>,
        task_definitions: Vec<Task>,
    ) -> usize {
        let id = self.flow_runs.len();

        self.flow_runs.push(FlowState::create_flow_state(
            id,
            flow_name,
            plan,
            task_definitions,
        ));

        return 0;
    }

    fn mark_task_running(&mut self, flow_id: usize, task_id: usize) -> Result<(), SchedulerError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(SchedulerError::FlowDoesNotExist);
        };

        flow.running_tasks.push(task_id);

        return Ok(());
    }

    fn schedule_next_stage(&self, flow_id: usize) -> Result<Option<Vec<&Task>>, SchedulerError> {
        let Some(flow) = self.flow_runs.get(flow_id) else {
            return Err(SchedulerError::FlowDoesNotExist);
        };

        let Some(stage) = flow.plan.get(flow.current_stage) else {
            return Err(SchedulerError::StageDoesNotExist);
        };

        for task_id in stage {
            match flow.finished_tasks.get(*task_id) {
                None => {
                    return Ok(None);
                }
                _ => (),
            }
        }

        let mut stage_tasks: Vec<&Task> = vec![];

        for option_task in stage.into_iter().map(|id| flow.task_definitions.get(*id)) {
            match option_task {
                None => {
                    return Err(SchedulerError::TaskDoesNotExist);
                }
                Some(task) => stage_tasks.push(task),
            }
        }

        return Ok(Some(stage_tasks));
    }

    fn mark_task_finished(&mut self, flow_id: usize, task_id: usize) -> Result<(), SchedulerError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(SchedulerError::FlowDoesNotExist);
        };

        flow.running_tasks.retain(|id| *id != task_id);

        flow.finished_tasks.push(task_id);

        if flow.running_tasks.len() == 0 {
            flow.status = FlowStatus::Success;
        }

        return Ok(());
    }

    fn mark_task_failed(&mut self, flow_id: usize, task_id: usize) -> Result<(), SchedulerError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(SchedulerError::FlowDoesNotExist);
        };

        flow.running_tasks.retain(|id| *id != task_id);

        flow.failed_tasks.push(task_id);

        flow.status = FlowStatus::Failed;

        return Ok(());
    }

    fn mark_flow_failed(&mut self, flow_id: usize) -> Result<(), SchedulerError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            return Err(SchedulerError::FlowDoesNotExist);
        };

        flow.status = FlowStatus::Failed;

        return Ok(());
    }
}

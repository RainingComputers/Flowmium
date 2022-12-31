use std::{
    collections::{binary_heap::Iter, BTreeMap},
    iter::Filter,
};

use super::model::{Flow, Task};

struct FlowState {
    id: usize,
    plan: Vec<Vec<usize>>,
    running_tasks: Vec<usize>,
    finished_tasks: Vec<usize>,
    task_definitions: Vec<Task>,
    flow_name: String,
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
            running_tasks: vec![],
            finished_tasks: vec![],
            task_definitions,
            flow_name,
        }
    }
}

struct ExecutorState {
    flow_runs: Vec<FlowState>,
}

impl ExecutorState {
    fn create_flow(
        self,
        flow_name: String,
        plan: Vec<Vec<i32>>,
        task_definitions: Vec<Task>,
    ) -> usize {
        // TODO: generate random unique id and insert into map

        return 0;
    }

    fn mark_task_finished(self, flow_id: usize, task_id: usize) {
        // TODO: Move the task from running to finished
    }

    fn schedule_next_task(self, flow_id: usize) -> Option<Task> {
        // TODO: Fetch flow, look at plan, running tasks and finished tasks, return next task to run if any

        None
    }

    fn get_running_flows(self) -> Vec<usize> {
        self.flow_runs
            .into_iter()
            .filter(|flow_state| flow_state.running_tasks.len() != 0)
            .map(|flow_state| flow_state.id)
            .collect()
    }
}

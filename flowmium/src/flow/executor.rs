use super::errors::FlowError;
use super::model::ContainerDAGFlow;
use super::model::Task;
use super::planner::construct_plan;
use super::scheduler::Scheduler;

use k8s_openapi::{api::batch::v1::Job, serde_json};
use kube::{api::PostParams, Api, Client};

pub struct RunnerStatus {
    pub pending: Vec<(usize, usize)>,
    pub running: Vec<(usize, usize)>,
    pub finished: Vec<(usize, usize)>,
    pub failed: Vec<(usize, usize)>,
}

async fn spawn_task(flow_id: usize, task_id: usize, task: &Task) -> Result<Job, FlowError> {
    let Ok(client) = Client::try_default().await else {
        // TODO logging
        return Err(FlowError::UnableToSpawnTaskError)
    };

    let jobs: Api<Job> = Api::default_namespaced(client);

    let data = serde_json::from_value(serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": task.name,
        },
        "spec": {
            "template": {
                "metadata": {
                    "name": task.name,
                    "annotations": {
                        "flowmium.io/flow-id": flow_id.to_string(),
                        "flowmium.io/task-id": task_id.to_string()
                    }
                },
                "spec": {
                    "containers": [{
                        "name": task.name,
                        "image": "alpine:latest",
                        "command": ["sleep", "5"],
                    }],
                    "restartPolicy": "Never",
                }
            }
        }
    }))
    .unwrap();

    match jobs.create(&PostParams::default(), &data).await {
        Ok(job) => Ok(job),
        // TODO logging
        Err(error) => Err(FlowError::UnableToSpawnTaskError),
    }
}

pub async fn run_tasks(flow_id: usize, tasks: Vec<(usize, &Task)>) -> Result<(), FlowError> {
    for (task_id, task) in tasks {
        if let Err(err) = spawn_task(flow_id, task_id, task).await {
            // TODO logging and continue
            return Err(err);
        };
    }

    return Ok(());
}

pub fn status() -> RunnerStatus {
    return RunnerStatus {
        pending: vec![],
        running: vec![],
        finished: vec![],
        failed: vec![],
    };
}

async fn instantiate_flow(
    flow: ContainerDAGFlow,
    sched: &mut Scheduler,
) -> Result<usize, FlowError> {
    // TODO logging ?
    let plan = construct_plan(&flow.tasks)?;

    let flow_id = sched.create_flow(flow.name, plan, flow.tasks);

    if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
        // TODO logging ?
        run_tasks(flow_id, tasks).await?;
    };

    return Ok(flow_id);
}

async fn schedule_and_run_tasks(
    status: RunnerStatus,
    sched: &mut Scheduler,
) -> Result<(), FlowError> {
    // TODO log errors and continue on error

    for (flow_id, task_id) in status.finished {
        sched.mark_task_finished(flow_id, task_id)?;

        if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
            run_tasks(flow_id, tasks).await?;
        }
    }

    for (flow_id, task_id) in status.failed {
        sched.mark_task_failed(flow_id, task_id)?
    }

    return Ok(());
}

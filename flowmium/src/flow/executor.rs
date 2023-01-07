use super::errors::FlowError;
use super::model::ContainerDAGFlow;
use super::model::Task;
use super::planner::construct_plan;
use super::scheduler::Scheduler;

use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::{api::batch::v1::Job, serde_json};
use kube::api::ListParams;
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
        return Err(FlowError::UnableToConnectToKubernetes)
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
        Err(error) => {
            println!("Unable to spawn job because {:?}", error);
            Err(FlowError::UnableToSpawnTaskError)
        }
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

pub fn get_flow_task_id(pod: &Pod) -> Result<(usize, usize), FlowError> {
    // TODO: find a way to reduce boiler plate?

    let Some(annotations) = &pod.metadata.annotations else {
        return Err(FlowError::InvalidTaskInstanceError);
    };

    let Some(flow_id_string) = annotations.get("flowmium.io/flow-id") else {
        return Err(FlowError::InvalidTaskInstanceError);
    };

    let Some(task_id_string) = annotations.get("flowmium.io/task-id") else {
        return Err(FlowError::InvalidTaskInstanceError);
    };

    let Ok(flow_id) = flow_id_string.parse::<usize>() else {
        return Err(FlowError::InvalidTaskInstanceError);
    };

    let Ok(task_id) = task_id_string.parse::<usize>() else {
        return Err(FlowError::InvalidTaskInstanceError);
    };

    Ok((flow_id, task_id))
}

pub async fn get_status() -> Result<RunnerStatus, FlowError> {
    let mut runner_status = RunnerStatus {
        pending: vec![],
        running: vec![],
        finished: vec![],
        failed: vec![],
    };

    let Ok(client) = Client::try_default().await else {
        // TODO logging
        return Err(FlowError::UnableToConnectToKubernetes)
    };

    let pods: Api<Pod> = Api::default_namespaced(client);
    let Ok(pod_list) = pods.list(&ListParams::default()).await else {
        return Err(FlowError::UnableToConnectToKubernetes);
    };

    for pod in pod_list {
        let (flow_id, task_id) = match get_flow_task_id(&pod) {
            Ok(id) => id,
            Err(_) => {
                println!("Ignoring invalid pod {:?}", pod.metadata.name);
                continue;
            }
        };

        // TODO log errors and continue on error
        let Some(pod_status) = &pod.status else {
            return Err(FlowError::InvalidTaskInstanceError);
        };

        let Some(phase) = &pod_status.phase else {
            return Err(FlowError::InvalidTaskInstanceError);
        };

        match &phase[..] {
            "Pending" => runner_status.pending.push((flow_id, task_id)),
            "Running" => runner_status.running.push((flow_id, task_id)),
            "Succeeded" => runner_status.finished.push((flow_id, task_id)),
            "Failed" => runner_status.failed.push((flow_id, task_id)),
            _ => {
                return {
                    print!(
                        "Unrecognized phase {:?} for pod {:?}",
                        phase, pod.metadata.name
                    );
                    Err(FlowError::InvalidTaskInstanceError)
                }
            }
        }
    }

    return Ok(runner_status);
}

pub async fn instantiate_flow(
    flow: ContainerDAGFlow,
    sched: &mut Scheduler,
) -> Result<usize, FlowError> {
    // TODO logging ?
    let plan = construct_plan(&flow.tasks)?;

    let (flow_id, tasks) = sched.create_flow(flow.name, plan, flow.tasks);

    // TODO logging ?
    run_tasks(flow_id, tasks).await?;

    return Ok(flow_id);
}

pub async fn schedule_and_run_tasks(sched: &mut Scheduler) -> Result<(), FlowError> {
    // TODO log errors and continue on error

    let status = get_status().await?;

    for (flow_id, task_id) in status.finished {
        sched.mark_task_finished(flow_id, task_id)?;

        if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
            println!("Running tasks {:?}", tasks);
            run_tasks(flow_id, tasks).await?;
        }
    }

    for (flow_id, task_id) in status.failed {
        sched.mark_task_failed(flow_id, task_id)?
    }

    return Ok(());
}

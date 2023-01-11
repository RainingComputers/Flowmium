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

async fn get_kubernetes_client() -> Result<Client, FlowError> {
    match Client::try_default().await {
        Ok(client) => Ok(client),
        Err(error) => {
            tracing::error!(%error, "Unable to connect to kubernetes");
            return Err(FlowError::UnableToConnectToKubernetes);
        }
    }
}

#[tracing::instrument]
async fn spawn_task(flow_id: usize, task_id: usize, task: &Task) -> Result<Job, FlowError> {
    tracing::info!("Spawning task");

    let client = get_kubernetes_client().await?;

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
            tracing::error!(%error, "Unable to spawn job");
            Err(FlowError::UnableToSpawnTaskError)
        }
    }
}

#[tracing::instrument(skip(tasks))]
pub async fn run_tasks(flow_id: usize, tasks: Vec<(usize, &Task)>) {
    // TODO Propagate error

    for (task_id, task) in tasks {
        let _ = spawn_task(flow_id, task_id, task).await;
    }
}

#[tracing::instrument]
pub fn get_flow_task_id(pod: &Pod) -> Option<(usize, usize)> {
    // TODO: make this optional
    // TODO make flowmium.io/flow-id an env variable
    // TODO make flowmium.io/task-id an env variable

    let Some(annotations) = &pod.metadata.annotations else {
        tracing::warn!("Invalid pod with missing annotations");
        return None;
    };

    let Some(flow_id_string) = annotations.get("flowmium.io/flow-id") else {
        
        tracing::warn!("Invalid pod with missing 'flowmium.io/flow-id' annotation"); 
        return None;
    };

    let Some(task_id_string) = annotations.get("flowmium.io/task-id") else {
        tracing::warn!("Invalid pod with missing 'flowmium.io/task-id' annotation"); 
        return None;
    };

    let Ok(flow_id) = flow_id_string.parse::<usize>() else {
        tracing::warn!("Invalid pod with non integer value for 'flowmium.io/flow-id' annotation"); 
        return None;
    };

    let Ok(task_id) = task_id_string.parse::<usize>() else {
        tracing::warn!("Invalid pod with non integer value for 'flowmium.io/task-id' annotation"); 
        return None;
    };

    Some((flow_id, task_id))
}

#[tracing::instrument]
pub async fn get_status() -> Result<RunnerStatus, FlowError> {
    let mut runner_status = RunnerStatus {
        pending: vec![],
        running: vec![],
        finished: vec![],
        failed: vec![],
    };

    let client = get_kubernetes_client().await?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    let Ok(pod_list) = pods.list(&ListParams::default()).await else {
        tracing::error!("Unable to list pods");
        return Err(FlowError::UnableToConnectToKubernetes);
    };

    for pod in pod_list {
        // TODO: propagate errors

        let pod_name = &pod.metadata.name;

        let Some((flow_id, task_id)) = get_flow_task_id(&pod) else {
            continue;
        };

        let Some(pod_status) = &pod.status else {
            tracing::warn!(pod_name = pod_name, "Pod with invalid status");
            continue;
        };

        let Some(phase) = &pod_status.phase else {
            tracing::warn!(pod_name=pod_name, "Pod with invalid pah");
            continue;
        };

        match &phase[..] {
            "Pending" => runner_status.pending.push((flow_id, task_id)),
            "Running" => runner_status.running.push((flow_id, task_id)),
            "Succeeded" => runner_status.finished.push((flow_id, task_id)),
            "Failed" => runner_status.failed.push((flow_id, task_id)),
            _ => { 
                tracing::warn!(pod_name=pod_name, "Pod with invalid phase");
                continue;
            }
        }
    }

    return Ok(runner_status);
}

#[tracing::instrument(skip(sched, flow))]
pub async fn instantiate_flow(
    flow: ContainerDAGFlow,
    sched: &mut Scheduler,
) -> Result<usize, FlowError> {
    // TODO logging ?
    let plan = construct_plan(&flow.tasks)?;

    tracing::info!(flow_name = flow.name, plan = ?plan, "Creating flow");
    let (flow_id, tasks) = sched.create_flow(flow.name, plan, flow.tasks);

    run_tasks(flow_id, tasks).await;

    return Ok(flow_id);
}

#[tracing::instrument(skip(sched))]
pub async fn schedule_and_run_tasks(sched: &mut Scheduler) -> Result<(), FlowError> {
    // TODO log errors and continue on error

    let status = get_status().await?;

    for (flow_id, task_id) in status.finished {
        sched.mark_task_finished(flow_id, task_id)?;
        tracing::debug!(flow_id = flow_id, task_id = task_id, "Finished task");

        if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
            run_tasks(flow_id, tasks).await;
        }
    }

    for (flow_id, task_id) in status.failed {
        sched.mark_task_failed(flow_id, task_id)?;
        tracing::warn!(flow_id = flow_id, task_id = task_id, "Task failed");
    }

    return Ok(());
}

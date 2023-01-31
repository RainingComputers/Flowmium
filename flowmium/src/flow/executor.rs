use super::errors::FlowError;
use super::model::ContainerDAGFlow;
use super::model::Task;
use super::planner::construct_plan;
use super::scheduler::Scheduler;

use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::{api::batch::v1::Job, serde_json};
use kube::api::ListParams;
use kube::core::ObjectList;
use kube::{api::PostParams, Api, Client};

#[derive(Debug, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Finished,
    Failed,
    Unknown,
}

async fn get_kubernetes_client() -> Result<Client, FlowError> {
    match Client::try_default().await {
        Ok(client) => Ok(client),
        Err(error) => {
            tracing::error!(%error, "Unable to connect to kubernetes");
            return Err(FlowError::UnableToConnectToKubernetesError);
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
                    "labels": {
                        "flowmium.io/flow-id": flow_id.to_string(),
                        "flowmium.io/task-id": task_id.to_string()
                    }
                },
                "spec": {
                    "containers": [{
                        "name": task.name,
                        "image": "alpine:latest",
                        "command": task.cmd,
                    }],
                    "restartPolicy": "Never",
                }
            },
            "backoffLimit": 0,
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

#[tracing::instrument]
async fn list_pods(flow_id: usize, task_id: usize) -> Result<ObjectList<Pod>, FlowError> {
    let client = get_kubernetes_client().await?;

    let pods_api: Api<Pod> = Api::namespaced(client, "default"); // TODO: Env variable for namespace

    let label_selector = format!(
        "flowmium.io/flow-id={},flowmium.io/task-id={}",
        flow_id, task_id
    );

    let mut list_params = ListParams::default();
    list_params = list_params.labels(&label_selector);

    let pod_list = match pods_api.list(&list_params).await {
        Ok(list) => list,
        Err(error) => {
            tracing::error!(%error, "Unable to list pods");
            return Err(FlowError::UnableToConnectToKubernetesError);
        }
    };

    return Ok(pod_list);
}

fn get_pod_phase(pod: Pod) -> Option<String> {
    let pod_status = pod.status?;
    let phase = pod_status.phase?;

    return Some(phase);
}

fn phase_to_task_status(phase: String) -> TaskStatus {
    match &phase[..] {
        "Pending" => TaskStatus::Pending,
        "Running" => TaskStatus::Running,
        "Succeeded" => TaskStatus::Finished,
        "Failed" => TaskStatus::Failed,
        "StartError" => TaskStatus::Failed,
        _ => TaskStatus::Unknown,
    }
}

#[tracing::instrument]
async fn get_task_status(flow_id: usize, task_id: usize) -> Result<TaskStatus, FlowError> {
    let pod_list = list_pods(flow_id, task_id).await?;
    let mut pod_iter = pod_list.iter();

    let Some(pod) = pod_iter.next() else {
        tracing::error!("Cannot find corresponding pod for task");
        return Err(FlowError::UnexpectedRunnerStateError);
    };

    if pod_iter.peekable().peek().is_some() {
        tracing::error!("Found duplicate pod for task");
        return Err(FlowError::UnexpectedRunnerStateError);
    }

    let Some(phase) = get_pod_phase(pod.to_owned()) else {
        tracing::error!("Unable to fetch status for pod");
        return Err(FlowError::UnexpectedRunnerStateError)
    };

    let status = phase_to_task_status(phase);
    return Ok(status);
}

#[tracing::instrument(skip(sched, flow))]
pub async fn instantiate_flow(
    flow: ContainerDAGFlow,
    sched: &mut Scheduler,
) -> Result<usize, FlowError> {
    let plan = construct_plan(&flow.tasks)?;

    tracing::info!(flow_name = flow.name, plan = ?plan, "Creating flow");
    let flow_id = sched.create_flow(flow.name, plan, flow.tasks);

    return Ok(flow_id);
}

#[tracing::instrument(skip(sched))]
async fn sched_pending_tasks(sched: &mut Scheduler, flow_id: usize) -> Result<bool, FlowError> {
    let option_tasks = sched.schedule_tasks(flow_id)?;

    if let Some(tasks) = option_tasks {
        for (task_id, task) in tasks {
            match spawn_task(flow_id, task_id, &task).await {
                Ok(_) => sched.mark_task_running(flow_id, task_id)?,
                Err(_) => break,
            }
        }

        return Ok(true);
    }

    return Ok(false);
}

#[tracing::instrument(skip(sched))]
async fn mark_running_tasks(
    sched: &mut Scheduler,
    flow_id: usize,
    task_id: usize,
) -> Result<(), FlowError> {
    let status = match get_task_status(flow_id, task_id).await {
        Ok(status) => status,
        Err(_) => return sched.mark_task_failed(flow_id, task_id),
    };

    return match status {
        TaskStatus::Pending | TaskStatus::Running => Ok(()),
        TaskStatus::Finished => sched.mark_task_finished(flow_id, task_id),
        TaskStatus::Failed | TaskStatus::Unknown => sched.mark_task_failed(flow_id, task_id),
    };
}

#[tracing::instrument(skip(sched))]
pub async fn schedule_and_run_tasks(sched: &mut Scheduler) {
    for (flow_id, running_tasks) in sched.get_running_or_pending_flows() {
        match sched_pending_tasks(sched, flow_id).await {
            Ok(true) => continue,
            Ok(false) => (),
            Err(_) => break,
        }

        for task_id in running_tasks {
            if let Err(_) = mark_running_tasks(sched, flow_id, task_id).await {
                break;
            };
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use kube::api::DeleteParams;
    use serial_test::serial;

    use super::*;

    async fn delete_all_pods() {
        let client = get_kubernetes_client().await.unwrap();

        let pods_api: Api<Pod> = Api::namespaced(client, "default");

        pods_api
            .delete_collection(&DeleteParams::default(), &ListParams::default())
            .await
            .unwrap();
    }

    async fn delete_all_jobs() {
        let client = get_kubernetes_client().await.unwrap();

        let jobs_api: Api<Job> = Api::namespaced(client, "default");

        jobs_api
            .delete_collection(&DeleteParams::default(), &ListParams::default())
            .await
            .unwrap();
    }

    fn test_flow() -> ContainerDAGFlow {
        ContainerDAGFlow {
            name: "hello-world".to_owned(),
            schedule: Some("".to_owned()),
            tasks: vec![
                Task {
                    name: "task-e".to_string(),
                    image: "".to_string(),
                    depends: vec![],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-b".to_string(),
                    image: "".to_string(),
                    depends: vec!["task-d".to_string()],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-a".to_string(),
                    image: "".to_string(),
                    depends: vec![
                        "task-b".to_string(),
                        "task-c".to_string(),
                        "task-d".to_string(),
                        "task-e".to_string(),
                    ],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-d".to_string(),
                    image: "".to_string(),
                    depends: vec!["task-e".to_string()],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-c".to_string(),
                    image: "".to_string(),
                    depends: vec!["task-d".to_string()],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
            ],
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_schedule_and_run_tasks() {
        delete_all_pods().await;
        delete_all_jobs().await;

        let mut sched = Scheduler { flow_runs: vec![] };

        let flow_id = instantiate_flow(test_flow(), &mut sched).await.unwrap();

        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&mut sched).await;
        }

        for task_id in 0..5 {
            assert_eq!(
                get_task_status(flow_id, task_id).await.unwrap(),
                TaskStatus::Finished
            )
        }
    }

    fn test_flow_fail() -> ContainerDAGFlow {
        ContainerDAGFlow {
            name: "hello-world".to_owned(),
            schedule: Some("".to_owned()),
            tasks: vec![
                Task {
                    name: "task-one".to_string(),
                    image: "".to_string(),
                    depends: vec!["task-two".to_string()],
                    cmd: vec!["exit".to_string(), "1".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-zero".to_string(),
                    image: "".to_string(),
                    depends: vec!["task-one".to_string()],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-two".to_string(),
                    image: "".to_string(),
                    depends: vec![],
                    cmd: vec!["sleep".to_string(), "0.5".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
            ],
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_schedule_and_run_tasks_fail() {
        delete_all_pods().await;
        delete_all_jobs().await;

        let mut sched = Scheduler { flow_runs: vec![] };

        let flow_id = instantiate_flow(test_flow_fail(), &mut sched)
            .await
            .unwrap();

        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&mut sched).await;
        }

        assert_eq!(
            get_task_status(flow_id, 2).await.unwrap(),
            TaskStatus::Finished
        );

        assert_eq!(
            get_task_status(flow_id, 0).await.unwrap(),
            TaskStatus::Failed
        );

        assert_eq!(
            get_task_status(flow_id, 1).await,
            Err(FlowError::UnexpectedRunnerStateError)
        );
    }
}

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
use serde::Deserialize;

#[derive(Debug, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Finished,
    Failed,
    Unknown,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct TaskPodConfig {
    store_url: String,
    bucket_name: String,
    access_key: String,
    secret_key: String,
    flowmium_image: String,
}

#[derive(Debug, PartialEq)]
pub struct ExecutorConfig {
    namespace: String,
    flow_id_label: String,
    task_id_label: String,
    pod_config: TaskPodConfig,
}

impl ExecutorConfig {
    pub fn create_default_config(task_pod_config: TaskPodConfig) -> ExecutorConfig {
        ExecutorConfig {
            namespace: "default".to_owned(),
            flow_id_label: "flowmium.io/flow-id".to_owned(),
            task_id_label: "flowmium.io/task-id".to_owned(),
            pod_config: task_pod_config,
        }
    }
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

fn get_task_cmd(task: &Task) -> Vec<&str> {
    let mut task_cmd = vec!["/var/run/flowmium", "task"];
    task_cmd.extend(task.cmd.iter().map(|elem| &elem[..]));

    return task_cmd;
}

#[tracing::instrument(skip(config))]
async fn spawn_task(
    flow_id: usize,
    task_id: usize,
    task: &Task,
    config: &ExecutorConfig,
) -> Result<Job, FlowError> {
    tracing::info!("Spawning task");

    let client = get_kubernetes_client().await?;

    let jobs: Api<Job> = Api::default_namespaced(client);

    let input_json = match serde_json::to_string(&task.inputs) {
        Ok(string) => string,
        Err(error) => {
            tracing::error!(%error, "Unable to serailize input");
            return Err(FlowError::InvalidTaskDefinition);
        }
    };

    let output_json = match serde_json::to_string(&task.outputs) {
        Ok(string) => string,
        Err(error) => {
            tracing::error!(%error, "Unable to serailize output");
            return Err(FlowError::InvalidTaskDefinition);
        }
    };

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
                        &config.flow_id_label: flow_id.to_string(),
                        &config.task_id_label: task_id.to_string()
                    }
                },
                "spec": {
                    "initContainers": [
                        {
                            "name": "init",
                            "image": &config.pod_config.flowmium_image,
                            "command": ["/flowmium", "init", "/flowmium", "/var/run/flowmium"],
                            "volumeMounts": [
                                {
                                    "name": "executable",
                                    "mountPath": "/var/run",
                                }
                            ]
                        }
                    ],
                    "containers": [{
                        "name": task.name,
                        "image": task.image,
                        "command": get_task_cmd(&task),
                        "env": [
                            {
                                "name": "INPUT_JSON",
                                "value": input_json
                            },
                            {
                                "name": "OUTPUT_JSON",
                                "value": output_json,
                              },
                            {
                                "name": "FLOW_ID",
                                "value": flow_id.to_string(),
                            },
                            {
                                "name": "ACCESS_KEY",
                                "value": config.pod_config.access_key,
                            },
                            {
                                "name": "SECRET_KEY",
                                "value": config.pod_config.secret_key,
                            },
                            {
                                "name": "BUCKET_NAME",
                                "value": config.pod_config.bucket_name,
                            },
                            {
                                "name": "STORE_URL",
                                "value": config.pod_config.store_url,
                            }
                        ],
                        "volumeMounts": [
                            {
                                "name": "executable",
                                "mountPath": "/var/run",
                            }
                        ]
                    }],
                    "restartPolicy": "Never",
                    "volumes": [
                        {
                            "name": "executable",
                            "emptyDir": {
                                "medium": "Memory",
                            }
                        }
                    ],
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

#[tracing::instrument(skip(config))]
async fn list_pods(
    flow_id: usize,
    task_id: usize,
    config: &ExecutorConfig,
) -> Result<ObjectList<Pod>, FlowError> {
    let client = get_kubernetes_client().await?;

    let pods_api: Api<Pod> = Api::namespaced(client, &config.namespace);

    let label_selector = format!(
        "{}={},{}={}",
        config.flow_id_label, flow_id, config.task_id_label, task_id
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

#[tracing::instrument(skip(config))]
async fn get_task_status(
    flow_id: usize,
    task_id: usize,
    config: &ExecutorConfig,
) -> Result<TaskStatus, FlowError> {
    let pod_list = list_pods(flow_id, task_id, config).await?;
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

#[tracing::instrument(skip(sched, config))]
async fn sched_pending_tasks(
    sched: &mut Scheduler,
    flow_id: usize,
    config: &ExecutorConfig,
) -> Result<bool, FlowError> {
    let option_tasks = sched.schedule_tasks(flow_id)?;

    if let Some(tasks) = option_tasks {
        for (task_id, task) in tasks {
            match spawn_task(flow_id, task_id, &task, &config).await {
                Ok(_) => sched.mark_task_running(flow_id, task_id)?,
                Err(_) => break,
            }
        }

        return Ok(true);
    }

    return Ok(false);
}

#[tracing::instrument(skip(sched, config))]
async fn mark_running_tasks(
    sched: &mut Scheduler,
    flow_id: usize,
    task_id: usize,
    config: &ExecutorConfig,
) -> Result<(), FlowError> {
    let status = match get_task_status(flow_id, task_id, config).await {
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
pub async fn schedule_and_run_tasks(sched: &mut Scheduler, config: &ExecutorConfig) {
    for (flow_id, running_tasks) in sched.get_running_or_pending_flows() {
        match sched_pending_tasks(sched, flow_id, &config).await {
            Ok(true) => continue,
            Ok(false) => (),
            Err(_) => break,
        }

        for task_id in running_tasks {
            if let Err(_) = mark_running_tasks(sched, flow_id, task_id, &config).await {
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

    use crate::flow::model::{Input, Output};

    use super::*;

    fn test_pod_config() -> TaskPodConfig {
        TaskPodConfig {
            store_url: "http://172.16.238.4:9000".to_owned(),
            bucket_name: "flowmium-test".to_owned(),
            access_key: "minio".to_owned(),
            secret_key: "password".to_owned(),
            flowmium_image: "registry:5000/flowmium-debug".to_owned(),
        }
    }

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
                    image: "ubuntu:latest".to_string(),
                    depends: vec![],
                    cmd: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "echo 'Greetings foobar' >> /greetings-foobar".to_string(),
                    ],
                    env: vec![],
                    inputs: None,
                    outputs: Some(vec![Output {
                        name: "OutputFromTaskE".to_string(),
                        path: "/greetings-foobar".to_string(),
                    }]),
                },
                Task {
                    name: "task-b".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec!["task-d".to_string()],
                    cmd: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "cat /task-d-output | sed 's/\\bfoobar\\b/world/g' > /hello-world"
                            .to_string(),
                    ],
                    env: vec![],
                    inputs: Some(vec![Input {
                        from: "OutputFromTaskD".to_string(),
                        path: "/task-d-output".to_string(),
                    }]),
                    outputs: Some(vec![Output {
                        name: "OutputFromTaskB".to_string(),
                        path: "/hello-world".to_string(),
                    }]),
                },
                Task {
                    name: "task-a".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec![
                        "task-b".to_string(),
                        "task-c".to_string(),
                        "task-d".to_string(),
                        "task-e".to_string(),
                    ],
                    cmd: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "echo `cat /task-b-output` `cat /task-c-output` `cat /task-d-output` `cat /task-e-output` > /concat-all"
                            .to_string(),
                    ],
                    env: vec![],
                    inputs: Some(vec![
                        Input {
                            from: "OutputFromTaskB".to_string(),
                            path: "/task-b-output".to_string(),
                        },
                        Input {
                            from: "OutputFromTaskC".to_string(),
                            path: "/task-c-output".to_string(),
                        },
                        Input {
                            from: "OutputFromTaskD".to_string(),
                            path: "/task-d-output".to_string(),
                        },
                        Input {
                            from: "OutputFromTaskE".to_string(),
                            path: "/task-e-output".to_string(),
                        },
                    ]),
                    outputs: Some(vec![Output {
                        name: "OutputFromTaskA".to_string(),
                        path: "/concat-all".to_string(),
                    }]),
                },
                Task {
                    name: "task-d".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec!["task-e".to_string()],
                    cmd: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "cat /task-e-output | sed 's/\\bGreetings\\b/Hello/g' > /hello-foobar"
                            .to_string(),
                    ],
                    env: vec![],
                    inputs: Some(vec![Input {
                        from: "OutputFromTaskE".to_string(),
                        path: "/task-e-output".to_string(),
                    }]),
                    outputs: Some(vec![Output {
                        name: "OutputFromTaskD".to_string(),
                        path: "/hello-foobar".to_string(),
                    }]),
                },
                Task {
                    name: "task-c".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec!["task-d".to_string()],
                    cmd: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "cat /task-d-output | sed 's/\\bfoobar\\b/mars/g' > /hello-mars"
                            .to_string(),
                    ],
                    env: vec![],
                    inputs: Some(vec![Input {
                        from: "OutputFromTaskD".to_string(),
                        path: "/task-d-output".to_string(),
                    }]),
                    outputs: Some(vec![Output {
                        name: "OutputFromTaskC".to_string(),
                        path: "/hello-mars".to_string(),
                    }]),
                },
            ],
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_schedule_and_run_tasks() {
        delete_all_pods().await;
        delete_all_jobs().await;

        let config = ExecutorConfig::create_default_config(test_pod_config());
        let mut sched = Scheduler { flow_runs: vec![] };

        let flow_id = instantiate_flow(test_flow(), &mut sched).await.unwrap();

        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&mut sched, &config).await;
        }

        for task_id in 0..5 {
            assert_eq!(
                get_task_status(flow_id, task_id, &config).await.unwrap(),
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
                    image: "ubuntu:latest".to_string(),
                    depends: vec!["task-two".to_string()],
                    cmd: vec!["exit".to_string(), "1".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-zero".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec!["task-one".to_string()],
                    cmd: vec!["sleep".to_string(), "0.01".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
                Task {
                    name: "task-two".to_string(),
                    image: "ubuntu:latest".to_string(),
                    depends: vec![],
                    cmd: vec!["sleep".to_string(), "0.01".to_string()],
                    env: vec![],
                    inputs: None,
                    outputs: None,
                },
            ],
        }
    }

    // #[tokio::test]
    // #[serial]
    // async fn test_schedule_and_run_tasks_fail() {
    //     delete_all_pods().await;
    //     delete_all_jobs().await;

    //     let config = ExecutorConfig::create_default_config(test_pod_config());
    //     let mut sched = Scheduler { flow_runs: vec![] };

    //     let flow_id = instantiate_flow(test_flow_fail(), &mut sched)
    //         .await
    //         .unwrap();

    //     for _ in 0..30 {
    //         tokio::time::sleep(Duration::from_millis(1000)).await;
    //         schedule_and_run_tasks(&mut sched, &config).await;
    //     }

    //     assert_eq!(
    //         get_task_status(flow_id, 2, &config).await.unwrap(),
    //         TaskStatus::Finished
    //     );

    //     assert_eq!(
    //         get_task_status(flow_id, 0, &config).await.unwrap(),
    //         TaskStatus::Failed
    //     );

    //     assert_eq!(
    //         get_task_status(flow_id, 1, &config).await,
    //         Err(FlowError::UnexpectedRunnerStateError)
    //     );
    // }
}

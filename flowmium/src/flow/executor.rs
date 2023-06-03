use super::model::EnvVar;
use super::model::Flow;
use super::model::KeyValuePair;
use super::model::SecretRef;
use super::model::Task;
use super::planner::construct_plan;
use super::planner::PlannerError;
use super::scheduler::Scheduler;
use super::scheduler::SchedulerError;

use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::{api::batch::v1::Job, serde_json};
use kube::api::ListParams;
use kube::core::ObjectList;
use kube::{api::PostParams, Api, Client};
use serde::Deserialize;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("unable to spawn task {0}")]
    UnableToSpawnTaskError(#[source] kube::error::Error),
    #[error("unable connect to kubernetes {0}")]
    UnableToConnectToKubernetesError(#[source] kube::error::Error),
    #[error("unexpected runner state for flow {0} task {1}")]
    UnexpectedRunnerStateError(i32, i32),
    #[error("invalid task definition {0}")]
    InvalidTaskDefinitionError(#[source] serde_json::Error),
    #[error("unable to construct plan")]
    UnableToConstructPlanError(
        #[from]
        #[source]
        PlannerError,
    ),
    #[error("unable to create flow error")]
    UnableToCreateFlowOrMarkTaskError(
        #[from]
        #[source]
        SchedulerError,
    ),
}

#[derive(Debug, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Finished,
    Failed,
    Unknown,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct TaskPodConfig {
    store_url: String,
    bucket_name: String,
    access_key: String,
    secret_key: String,
    executor_image: String,
}

#[derive(Debug, PartialEq, Clone)]
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

async fn get_kubernetes_client() -> Result<Client, ExecutorError> {
    match Client::try_default().await {
        Ok(client) => Ok(client),
        Err(error) => {
            tracing::error!(%error, "Unable to connect to kubernetes");
            return Err(ExecutorError::UnableToConnectToKubernetesError(error));
        }
    }
}

fn get_task_cmd(task: &Task) -> Vec<&str> {
    let mut task_cmd = vec!["/var/run/flowmium", "task"];
    task_cmd.extend(task.cmd.iter().map(|elem| &elem[..]));

    return task_cmd;
}

fn get_task_envs<'a>(
    task: &'a Task,
    input_json: String,
    output_json: String,
    flow_id: i32,
    config: &'a ExecutorConfig,
) -> Vec<serde_json::Value> {
    let mut task_envs: Vec<serde_json::Value> = vec![
        serde_json::json! ({
            "name": "FLOWMIUM_INPUT_JSON",
            "value": input_json,
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_OUTPUT_JSON",
            "value": output_json,
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_FLOW_ID",
            "value": flow_id.to_string(),
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_ACCESS_KEY",
            "value": config.pod_config.access_key,
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_SECRET_KEY",
            "value": config.pod_config.secret_key,
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_BUCKET_NAME",
            "value": config.pod_config.bucket_name,
        }),
        serde_json::json!( {
            "name": "FLOWMIUM_STORE_URL",
            "value": config.pod_config.store_url,
        }),
    ];

    task_envs.extend(task.env.iter().map(|env| match env {
        EnvVar::KeyValuePair(KeyValuePair { name, value }) => {
            serde_json::json! ({"name": name, "value": value})
        }
        EnvVar::SecretRef(SecretRef { name, from_secret }) => {
            serde_json::json! ({"name": name, "value": from_secret}) // TODO: Actually fetch secret
        }
    }));

    return task_envs;
}

#[tracing::instrument(skip(config))]
async fn spawn_task(
    flow_id: i32,
    task_id: i32,
    task: &Task,
    config: &ExecutorConfig,
) -> Result<Job, ExecutorError> {
    tracing::info!("Spawning task");

    let client = get_kubernetes_client().await?;

    let jobs: Api<Job> = Api::default_namespaced(client);

    let input_json = match serde_json::to_string(&task.inputs) {
        Ok(string) => string,
        Err(error) => {
            tracing::error!(%error, "Unable to serialize input");
            return Err(ExecutorError::InvalidTaskDefinitionError(error));
        }
    };

    let output_json = match serde_json::to_string(&task.outputs) {
        Ok(string) => string,
        Err(error) => {
            tracing::error!(%error, "Unable to serialize output");
            return Err(ExecutorError::InvalidTaskDefinitionError(error));
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
                            "image": &config.pod_config.executor_image,
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
                        "env": get_task_envs(task, input_json, output_json, flow_id, config),
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
            Err(ExecutorError::UnableToSpawnTaskError(error))
        }
    }
}

#[tracing::instrument(skip(config))]
async fn list_pods(
    flow_id: i32,
    task_id: i32,
    config: &ExecutorConfig,
) -> Result<ObjectList<Pod>, ExecutorError> {
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
            return Err(ExecutorError::UnableToConnectToKubernetesError(error));
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

// TODO: Batch these requests
#[tracing::instrument(skip(config))]
async fn get_task_status(
    flow_id: i32,
    task_id: i32,
    config: &ExecutorConfig,
) -> Result<TaskStatus, ExecutorError> {
    let pod_list = list_pods(flow_id, task_id, config).await?;
    let mut pod_iter = pod_list.iter();

    let Some(pod) = pod_iter.next() else {
        tracing::error!("Cannot find corresponding pod for task");
        return Err(ExecutorError::UnexpectedRunnerStateError(flow_id, task_id));
    };

    if pod_iter.peekable().peek().is_some() {
        tracing::error!("Found duplicate pod for task");
        return Err(ExecutorError::UnexpectedRunnerStateError(flow_id, task_id));
    }

    let Some(phase) = get_pod_phase(pod.to_owned()) else {
        tracing::error!("Unable to fetch status for pod");
        return Err(ExecutorError::UnexpectedRunnerStateError(flow_id, task_id))
    };

    let status = phase_to_task_status(phase);
    return Ok(status);
}

#[tracing::instrument(skip(sched, flow))]
pub async fn instantiate_flow(flow: Flow, sched: &Scheduler) -> Result<i32, ExecutorError> {
    let plan = construct_plan(&flow.tasks)?;

    tracing::info!(flow_name = flow.name, plan = ?plan, "Creating flow");
    let flow_id = sched.create_flow(flow.name, plan, flow.tasks).await?;

    return Ok(flow_id);
}

#[tracing::instrument(skip(sched, config))]
async fn sched_pending_tasks(
    sched: &Scheduler,
    flow_id: i32,
    config: &ExecutorConfig,
) -> Result<bool, ExecutorError> {
    let option_tasks = sched.schedule_tasks(flow_id).await?;

    if let Some(tasks) = option_tasks {
        for (task_id, task) in tasks {
            match spawn_task(flow_id, task_id, &task, &config).await {
                Ok(_) => sched.mark_task_running(flow_id, task_id).await?,
                Err(_) => break,
            }
        }

        return Ok(true);
    }

    return Ok(false);
}

#[tracing::instrument(skip(sched, config))]
async fn mark_running_tasks(
    sched: &Scheduler,
    flow_id: i32,
    task_id: i32,
    config: &ExecutorConfig,
) -> Result<(), SchedulerError> {
    let status = match get_task_status(flow_id, task_id, config).await {
        Ok(status) => status,
        Err(_) => return sched.mark_task_failed(flow_id, task_id).await,
    };

    return match status {
        TaskStatus::Pending | TaskStatus::Running => Ok(()),
        TaskStatus::Finished => sched.mark_task_finished(flow_id, task_id).await,
        TaskStatus::Failed | TaskStatus::Unknown => sched.mark_task_failed(flow_id, task_id).await,
    };
}

#[tracing::instrument(skip(sched))]
pub async fn schedule_and_run_tasks(sched: &Scheduler, config: &ExecutorConfig) {
    if let Ok(tasks_to_schedule) = sched.get_running_or_pending_flows().await {
        for (flow_id, running_tasks) in tasks_to_schedule {
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
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use kube::api::DeleteParams;
    use s3::Bucket;
    use serial_test::serial;
    use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

    use crate::{
        artefacts::bucket::get_bucket,
        flow::model::{Input, Output},
    };

    use super::*;

    async fn get_test_pool() -> Pool<Postgres> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://flowmium:flowmium@localhost/flowmium")
            .await
            .unwrap();

        return pool;
    }

    fn test_pod_config() -> TaskPodConfig {
        TaskPodConfig {
            store_url: "http://172.16.238.4:9000".to_owned(),
            bucket_name: "flowmium-test".to_owned(),
            access_key: "minio".to_owned(),
            secret_key: "password".to_owned(),
            executor_image: "registry:5000/flowmium-debug".to_owned(),
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

    async fn delete_all_objects(config: &ExecutorConfig) -> Bucket {
        let bucket = get_bucket(
            &config.pod_config.access_key,
            &config.pod_config.secret_key,
            &config.pod_config.bucket_name,
            config.pod_config.store_url.clone(),
        )
        .unwrap();

        let object_list = bucket
            .list("".to_string(), None)
            .await
            .unwrap()
            .get(0)
            .unwrap()
            .contents
            .clone();

        for obj in object_list {
            bucket.delete_object(obj.key).await.unwrap();
        }

        return bucket;
    }

    async fn delete_all_jobs() {
        let client = get_kubernetes_client().await.unwrap();

        let jobs_api: Api<Job> = Api::namespaced(client, "default");

        jobs_api
            .delete_collection(&DeleteParams::default(), &ListParams::default())
            .await
            .unwrap();
    }

    async fn get_contents(bucket: &Bucket, path: String) -> String {
        let response_data = bucket.get_object(path).await.unwrap();

        std::str::from_utf8(response_data.bytes())
            .unwrap()
            .to_owned()
    }

    fn test_flow() -> Flow {
        Flow {
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
                        "cat /inputs/testing/task-e-output | sed 's/\\bGreetings\\b/Hello/g' > /hello-foobar"
                            .to_string(),
                    ],
                    env: vec![],
                    inputs: Some(vec![Input {
                        from: "OutputFromTaskE".to_string(),
                        path: "/inputs/testing/task-e-output".to_string(),
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
        let pool = get_test_pool().await;
        let config = ExecutorConfig::create_default_config(test_pod_config());
        let sched = Scheduler { pool };

        delete_all_pods().await;
        delete_all_jobs().await;
        let bucket = delete_all_objects(&config).await;

        let flow_id = instantiate_flow(test_flow(), &sched).await.unwrap();

        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&sched, &config).await;
        }

        for task_id in 0..5 {
            assert_eq!(
                get_task_status(flow_id, task_id, &config).await.unwrap(),
                TaskStatus::Finished
            )
        }

        assert_eq!(
            get_contents(&bucket, format!("{}/OutputFromTaskA", flow_id)).await,
            "Hello world Hello mars Hello foobar Greetings foobar\n"
        );
        assert_eq!(
            get_contents(&bucket, format!("{}/OutputFromTaskB", flow_id)).await,
            "Hello world\n"
        );
        assert_eq!(
            get_contents(&bucket, format!("{}/OutputFromTaskC", flow_id)).await,
            "Hello mars\n"
        );
        assert_eq!(
            get_contents(&bucket, format!("{}/OutputFromTaskD", flow_id)).await,
            "Hello foobar\n"
        );
        assert_eq!(
            get_contents(&bucket, format!("{}/OutputFromTaskE", flow_id)).await,
            "Greetings foobar\n"
        );
    }

    fn test_flow_fail() -> Flow {
        Flow {
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

    #[tokio::test]
    #[serial]
    async fn test_schedule_and_run_tasks_fail() {
        delete_all_pods().await;
        delete_all_jobs().await;

        let pool = get_test_pool().await;
        let config = ExecutorConfig::create_default_config(test_pod_config());
        let sched = Scheduler { pool };

        let flow_id = instantiate_flow(test_flow_fail(), &sched).await.unwrap();

        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&sched, &config).await;
        }

        assert_eq!(
            get_task_status(flow_id, 2, &config).await.unwrap(),
            TaskStatus::Finished
        );

        assert_eq!(
            get_task_status(flow_id, 0, &config).await.unwrap(),
            TaskStatus::Failed
        );

        match get_task_status(flow_id, 1, &config).await {
            Err(ExecutorError::UnexpectedRunnerStateError(..)) => (),
            _ => panic!(),
        }
    }
}

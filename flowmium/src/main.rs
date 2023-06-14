mod api;
mod args;
mod artefacts;
mod flow;
mod pool;
mod secrets;

use api::start_server;
use artefacts::{
    bucket::get_bucket,
    errors::ArtefactError,
    init::do_init,
    task::{run_task, SidecarConfig},
};
use pool::{init_db_and_get_pool, PostgresConfig};
use s3::Bucket;
use secrets::SecretsCrud;
use sqlx::{Pool, Postgres};
use std::{process::ExitCode, time::Duration};
use tokio::fs;

use args::{ExecuteOpts, ServerOpts, TaskOpts};
use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks, ExecutorConfig, TaskPodConfig},
    model::Flow,
    scheduler::Scheduler,
};

async fn get_pool() -> Option<Pool<Postgres>> {
    let database_config: PostgresConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for database");
            return None;
        }
    };

    let Some(pool) = init_db_and_get_pool(database_config).await else {
        tracing::error!("Unable to initialize database");
        return None;
    };

    return Some(pool);
}

async fn get_executor_config() -> Option<ExecutorConfig> {
    let config: TaskPodConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for executor");
            return None;
        }
    };

    let executor_config = ExecutorConfig::create_default_config(config);

    return Some(executor_config);
}

fn get_bucket_from_executor_config(
    executor_config: &ExecutorConfig,
) -> Result<Bucket, ArtefactError> {
    let pod_config = &executor_config.pod_config;

    return get_bucket(
        &pod_config.access_key,
        &pod_config.secret_key,
        &pod_config.bucket_name,
        pod_config.store_url.clone(),
    );
}

#[tracing::instrument]
async fn execute_main(execute_opts: ExecuteOpts) -> ExitCode {
    let Some(pool) = get_pool().await else {
        return  ExitCode::FAILURE;
    };

    let Some(executor_config) = get_executor_config().await else {
        return  ExitCode::FAILURE;
    };

    let sched = Scheduler { pool: pool.clone() };
    let secrets = SecretsCrud { pool };

    for dag_file_path in execute_opts.files.iter() {
        let contents = match fs::read_to_string(dag_file_path).await {
            Ok(contents) => contents,
            Err(error) => {
                tracing::error!(%error, "Unable to read file {}", dag_file_path);
                return ExitCode::FAILURE;
            }
        };

        let flow = match serde_yaml::from_str::<Flow>(&contents) {
            Ok(flow) => flow,
            Err(error) => {
                tracing::error!(%error, "Unable to parse YAML in file {}", dag_file_path);
                return ExitCode::FAILURE;
            }
        };

        match instantiate_flow(flow, &sched).await {
            Ok(_) => (),
            Err(error) => {
                tracing::error!(%error, "Error instantiating flow");
                tracing::warn!("Skipping flow file {}", dag_file_path);
            }
        };
    }

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        schedule_and_run_tasks(&sched, &executor_config, &secrets).await;
    }
}

#[tracing::instrument]
async fn run_server(server_opts: ServerOpts) -> ExitCode {
    let Some(pool) = get_pool().await else {
        return  ExitCode::FAILURE;
    };

    let Some(executor_config) = get_executor_config().await else {
        return  ExitCode::FAILURE;
    };

    let Ok(bucket) = get_bucket_from_executor_config(&executor_config) else {
        return ExitCode::FAILURE;
    };

    let pool_loop = pool.clone();
    let executor_config_loop = executor_config.clone();

    tracing::info!("Starting scheduler loop");

    tokio::spawn(async move {
        let sched = Scheduler {
            pool: pool_loop.clone(),
        };
        let secrets = SecretsCrud {
            pool: pool_loop.clone(),
        };

        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&sched, &executor_config_loop, &secrets).await;
        }
    });

    tracing::info!("Starting API server");

    if let Err(error) = start_server(server_opts, pool.clone(), executor_config, bucket).await {
        tracing::error!(%error, "Unable to start server");
        return ExitCode::FAILURE;
    }

    return ExitCode::SUCCESS;
}

#[tracing::instrument]
async fn task_main(task_opts: TaskOpts) -> ExitCode {
    let config: SidecarConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for task");
            return ExitCode::FAILURE;
        }
    };

    return run_task(config, task_opts.cmd).await;
}

#[tokio::main]
async fn main() -> ExitCode {
    let subscriber = tracing_subscriber::fmt().with_line_number(true).finish();
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(()) => (),
        Err(_) => {
            eprintln!("Cannot initialize logger");
            return ExitCode::FAILURE;
        }
    };

    let args: args::FlowmiumOptions = argh::from_env();

    match args.command {
        args::Command::Init(init_opts) => do_init(init_opts.src, init_opts.dest).await,
        args::Command::Task(task_opts) => task_main(task_opts).await,
        args::Command::Execute(execute_opts) => execute_main(execute_opts).await,
        args::Command::Server(server_opts) => run_server(server_opts).await,
    }
}

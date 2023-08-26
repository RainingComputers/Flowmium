use crate::api::start_server;
use crate::artefacts::{
    bucket::get_bucket,
    errors::ArtefactError,
    init::do_init,
    task::{run_task, SidecarConfig},
};
use crate::pool::{init_db_and_get_pool, PostgresConfig};
use crate::secrets::SecretsCrud;
use s3::Bucket;
use sqlx::{Pool, Postgres};
use std::{process::ExitCode, time::Duration};

use crate::args::{self, ServerOpts, TaskOpts};
use crate::flow::{
    executor::{schedule_and_run_tasks, ExecutorConfig, TaskPodConfig},
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

    Some(pool)
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

    Some(executor_config)
}

fn get_bucket_from_executor_config(
    executor_config: &ExecutorConfig,
) -> Result<Bucket, ArtefactError> {
    let pod_config = &executor_config.pod_config;

    get_bucket(
        &pod_config.access_key,
        &pod_config.secret_key,
        &pod_config.bucket_name,
        pod_config.store_url.clone(),
    )
}

fn spawn_executor(pool: &Pool<Postgres>, sched: &Scheduler, executor_config: &ExecutorConfig) {
    let pool_loop = pool.clone();
    let sched_loop = sched.clone();
    let executor_config_loop = executor_config.clone();

    tracing::info!("Starting scheduler loop");

    tokio::spawn(async move {
        let secrets = SecretsCrud { pool: pool_loop };

        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&sched_loop, &executor_config_loop, &secrets).await;
        }
    });
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

    let sched = Scheduler::new(pool.clone());

    spawn_executor(&pool, &sched, &executor_config);

    tracing::info!("Starting API server");

    if let Err(error) =
        start_server(server_opts, pool.clone(), &sched, executor_config, bucket).await
    {
        tracing::error!(%error, "Unable to start server");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
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

    run_task(config, task_opts.cmd).await
}

pub async fn run() -> ExitCode {
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
        args::Command::Server(server_opts) => run_server(server_opts).await,
    }
}

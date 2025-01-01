use s3::Bucket;
use sqlx::{Pool, Postgres};
use std::{process::ExitCode, time::Duration};
use tokio::task::JoinHandle;

use crate::server::{
    api::start_server,
    args,
    executor::{schedule_and_run_tasks, ExecutorConfig},
    scheduler::Scheduler,
};
use crate::{
    retry::with_exp_backoff_retry,
    server::secrets::SecretsCrud,
    task::{
        bucket::get_bucket,
        driver::{run_task, SidecarConfig},
        errors::ArtefactError,
    },
};

use super::args::TaskOpts;
use super::pool::{init_db_and_get_pool, PostgresConfig};

/// Create a postgres connection pool object, create a table to store secrets and flow statuses and perform migration.
/// The tables are `flows` and `secrets` respectively. An environment variable named `FLOWMIUM_POSTGRES_URL` with value as an
/// URL to a postgres database is expected to be set.
pub async fn get_default_postgres_pool() -> Option<Pool<Postgres>> {
    let database_config: PostgresConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for database");
            return None;
        }
    };

    init_db_and_get_pool(database_config).await
}

/// Constructor executor config from environment variables. Environment variables that are expected to be set
/// are fields of [`crate::executor::ExecutorConfig`] but in all caps prefixed with `FLOWMIUM_`.
pub async fn get_default_executor_config() -> Option<ExecutorConfig> {
    let executor_config: ExecutorConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for executor");
            return None;
        }
    };

    Some(executor_config)
}

async fn get_bucket_from_executor_config(
    executor_config: &ExecutorConfig,
) -> Result<Box<Bucket>, ArtefactError> {
    get_bucket(
        &executor_config.access_key,
        &executor_config.secret_key,
        &executor_config.bucket_name,
        executor_config.store_url.clone(),
    )
    .await
}

/// Spawn a tokio task that periodically calls [`crate::executor::schedule_and_run_tasks`] every second
/// and makes progress on pending flows.
pub fn spawn_executor(
    pool: &Pool<Postgres>,
    sched: &Scheduler,
    executor_config: &ExecutorConfig,
) -> JoinHandle<()> {
    let pool_loop = pool.clone();
    let sched_loop = sched.clone();
    let executor_config_loop = executor_config.clone();

    tracing::info!("Starting scheduler loop");

    tokio::spawn(async move {
        let secrets = SecretsCrud::new(pool_loop);

        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            schedule_and_run_tasks(&sched_loop, &executor_config_loop, &secrets).await;
        }
    })
}

/// Run API server. This function does not return unless there is an error.
#[tracing::instrument(skip(pool, sched, executor_config))]
pub async fn run_api_server(
    pool: &Pool<Postgres>,
    sched: &Scheduler,
    executor_config: &ExecutorConfig,
    port: u16,
) -> ExitCode {
    tracing::info!("Starting API server");

    let Some(bucket) = with_exp_backoff_retry(
        || async { get_bucket_from_executor_config(executor_config).await.ok() },
        "Unable to create or open bucket",
        8,
    )
    .await
    else {
        return ExitCode::FAILURE;
    };

    if let Err(error) = start_server(port, pool.clone(), sched, bucket).await {
        tracing::error!(%error, "Unable to start server");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn server_main(port: u16) -> ExitCode {
    let Some(pool) = with_exp_backoff_retry(
        get_default_postgres_pool,
        "Unable to connect to database",
        8,
    )
    .await
    else {
        return ExitCode::FAILURE;
    };

    let Some(executor_config) = get_default_executor_config().await else {
        return ExitCode::FAILURE;
    };

    let sched = Scheduler::new(pool.clone());

    spawn_executor(&pool, &sched, &executor_config);

    run_api_server(&pool, &sched, &executor_config, port).await
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

#[tracing::instrument]
async fn init_main(src: String, dest: String) -> ExitCode {
    if let Err(error) = tokio::fs::copy(src, dest).await {
        tracing::error!(%error, "Unable to copy flowmium executable");
        return ExitCode::FAILURE;
    }

    tracing::info!("Copied flowmium executable into volume successfully");

    ExitCode::SUCCESS
}

/// Parse CLI arguments and environment variables and run `flowmium` CLI.
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
        args::Command::Init(init_opts) => init_main(init_opts.src, init_opts.dest).await,
        args::Command::Task(task_opts) => task_main(task_opts).await,
        args::Command::Server(server_opts) => server_main(server_opts.port).await,
    }
}

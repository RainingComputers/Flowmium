mod api;
mod args;
mod artefacts;
mod flow;
mod pool;

use api::start_server;
use artefacts::{
    init::do_init,
    task::{run_task, SidecarConfig},
};
use pool::{init_db_and_get_pool, PostgresConfig};
use std::{process::ExitCode, time::Duration};
use tokio::fs;

use args::{ExecuteOpts, FlowmiumOptions, ServerOpts, TaskOpts};
use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks, ExecutorConfig, TaskPodConfig},
    model::Flow,
    scheduler::Scheduler,
};
use gumdrop::Options;

async fn get_scheduler() -> Option<Scheduler> {
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

    let sched = Scheduler { pool };

    return Some(sched);
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

#[tracing::instrument]
async fn execute_main(execute_opts: ExecuteOpts) -> ExitCode {
    let Some(sched) = get_scheduler().await else {
        return  ExitCode::FAILURE;
    };

    let Some(executor_config) = get_executor_config().await else {
        return  ExitCode::FAILURE;
    };

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

        schedule_and_run_tasks(&sched, &executor_config).await;
    }
}

async fn run_server(server_opts: ServerOpts) -> ExitCode {
    let Some(sched) = get_scheduler().await else {
        return  ExitCode::FAILURE;
    };

    let Some(executor_config) = get_executor_config().await else {
        return  ExitCode::FAILURE;
    };

    if let Err(error) = start_server(server_opts, sched.clone(), executor_config.clone()).await {
        tracing::error!(%error, "Unable to start server");
        return ExitCode::FAILURE;
    }

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;

            schedule_and_run_tasks(&sched, &executor_config).await;
        }
    });

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

    let args: Vec<String> = std::env::args().collect();

    let opts = match FlowmiumOptions::parse_args(&args[1..], gumdrop::ParsingStyle::StopAtFirstFree)
    {
        Ok(opts) => opts,
        Err(error) => {
            print!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let Some(opt) = opts.command else {
        eprint!("Invalid sub command");
        return  ExitCode::FAILURE;
    };

    match opt {
        args::Command::Init(init_opts) => do_init(init_opts.src, init_opts.dest).await,
        args::Command::Task(task_opts) => task_main(task_opts).await,
        args::Command::Execute(execute_opts) => execute_main(execute_opts).await,
        args::Command::Server(server_opts) => run_server(server_opts).await,
    }
}

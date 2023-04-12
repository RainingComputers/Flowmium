mod args;
mod artefacts;
mod flow;

use artefacts::{
    init::do_init,
    task::{run_task, SidecarConfig},
};
use sqlx::postgres::PgPoolOptions;
use std::{process::ExitCode, time::Duration};
use tokio::fs;

use args::{ExecuteOpts, FlowmiumOptions, TaskOpts};
use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks, ExecutorConfig, TaskPodConfig},
    model::ContainerDAGFlow,
    scheduler::Scheduler,
};
use gumdrop::Options;

#[tracing::instrument]
async fn execute_main(execute_opts: ExecuteOpts) -> ExitCode {
    let config: TaskPodConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for executor");
            return ExitCode::FAILURE;
        }
    };

    // TODO: Clean DB code

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://flowmium:flowmium@localhost/flowmium")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // --- //

    let executor_config = ExecutorConfig::create_default_config(config);
    let mut sched = Scheduler { pool };

    for dag_file_path in execute_opts.files.iter() {
        let contents = match fs::read_to_string(dag_file_path).await {
            Ok(contents) => contents,
            Err(error) => {
                tracing::error!(%error, "Unable to read file {}", dag_file_path);
                return ExitCode::FAILURE;
            }
        };

        let flow = match serde_yaml::from_str::<ContainerDAGFlow>(&contents) {
            Ok(flow) => flow,
            Err(error) => {
                tracing::error!(%error, "Unable to parse YAML in file {}", dag_file_path);
                return ExitCode::FAILURE;
            }
        };

        match instantiate_flow(flow, &mut sched).await {
            Ok(_) => (),
            Err(error) => {
                tracing::error!(%error, "Error instantiating flow");
                tracing::warn!("Skipping flow file {}", dag_file_path);
            }
        };
    }

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        schedule_and_run_tasks(&mut sched, &executor_config).await;
    }
}

#[tracing::instrument]
async fn task_main(taks_opts: TaskOpts) -> ExitCode {
    let config: SidecarConfig = match envy::prefixed("FLOWMIUM_").from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "Invalid env config for task");
            return ExitCode::FAILURE;
        }
    };

    return run_task(config, taks_opts.cmd).await;
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
    }
}

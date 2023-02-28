mod args;
mod artefacts;
mod flow;

use std::{process::ExitCode, time::Duration};
use tokio::fs;

use args::{ExecuteOpts, FlowmiumOptions, SidecarOpts};
use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks, ExecutorConfig},
    model::{ContainerDAGFlow, Task},
    scheduler::Scheduler,
};
use gumdrop::Options;

async fn execute_main(executeOpts: ExecuteOpts) -> ExitCode {
    let config = ExecutorConfig::create_default_config();
    let mut sched = Scheduler { flow_runs: vec![] };

    for dag_file_path in executeOpts.files.iter() {
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
            Err(_) => {
                return ExitCode::FAILURE;
            }
        };
    }

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        schedule_and_run_tasks(&mut sched, &config).await;
    }
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

    let opts = FlowmiumOptions::parse_args_default_or_exit();

    let Some(opt) = opts.command else {
        eprint!("Invalid sub command");
        return  ExitCode::FAILURE;
    };

    match opt {
        args::Command::Sidecar(sidecarOpts) => ExitCode::FAILURE,
        args::Command::Main(mainOpts) => ExitCode::FAILURE,
        args::Command::Execute(executeOpts) => execute_main(executeOpts).await,
    }
}

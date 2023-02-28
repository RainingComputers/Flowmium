mod flow;
mod artefacts;

use std::{process::ExitCode, time::Duration};

use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks, ExecutorConfig},
    model::{ContainerDAGFlow, Task},
    scheduler::Scheduler,
};

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

    let flow = ContainerDAGFlow {
        name: "hello-world".to_owned(),
        schedule: Some("".to_owned()),
        tasks: vec![
            Task {
                name: "task-e".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "task-b".to_string(),
                image: "".to_string(),
                depends: vec!["task-d".to_string()],
                cmd: vec![],
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
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "task-d".to_string(),
                image: "".to_string(),
                depends: vec!["task-e".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "task-c".to_string(),
                image: "".to_string(),
                depends: vec!["task-d".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
        ],
    };

    let config = ExecutorConfig::create_default_config();
    let mut sched = Scheduler { flow_runs: vec![] };

    match instantiate_flow(flow, &mut sched).await {
        Ok(_) => (),
        Err(_) => {
            return ExitCode::FAILURE;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        schedule_and_run_tasks(&mut sched, &config).await;
    }
}

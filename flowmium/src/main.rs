mod flow;

use std::{process::ExitCode, thread::sleep, time::Duration};

use flow::{
    executor::{instantiate_flow, schedule_and_run_tasks},
    model::{ContainerDAGFlow, Flow, Task},
    scheduler::Scheduler,
};

#[tokio::main]
async fn main() -> ExitCode {
    let flow = ContainerDAGFlow {
        name: "hello-world".to_owned(),
        schedule: Some("".to_owned()),
        tasks: vec![
            Task {
                name: "E".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["D".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "A".to_string(),
                image: "".to_string(),
                depends: vec![
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                    "E".to_string(),
                ],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "D".to_string(),
                image: "".to_string(),
                depends: vec!["E".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "C".to_string(),
                image: "".to_string(),
                depends: vec!["D".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
        ],
    };

    let mut sched = Scheduler { flow_runs: vec![] };

    match instantiate_flow(flow, &mut sched).await {
        Ok(_) => (),
        Err(_) => return ExitCode::FAILURE,
    };

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        match schedule_and_run_tasks(&mut sched).await {
            Ok(()) => (),
            Err(_) => return ExitCode::FAILURE,
        };
    }
}

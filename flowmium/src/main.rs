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

    let mut sched = Scheduler { flow_runs: vec![] };

    match instantiate_flow(flow, &mut sched).await {
        Ok(_) => (),
        Err(err) => {
            println!("Error {:?}", err);
            return ExitCode::FAILURE;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        println!("Scheduler state is {:?}", sched.flow_runs);

        match schedule_and_run_tasks(&mut sched).await {
            Ok(()) => (),
            Err(err) => {
                println!("Error {:?}", err);
                return ExitCode::FAILURE;
            }
        };
    }
}

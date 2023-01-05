mod flow;

use flow::model::Task;
use flow::planner::construct_plan;
use std::process::ExitCode;

fn main() -> ExitCode {
    let tasks: Vec<Task> = vec![];
    construct_plan(&tasks);
    return ExitCode::SUCCESS;
}

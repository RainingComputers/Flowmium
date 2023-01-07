use super::errors::FlowError;
use super::model::ContainerDAGFlow;
use super::planner::construct_plan;
use super::runner::{KubernetesRunner, RunnerStatus};
use super::scheduler::Scheduler;

fn instantiate_flow(
    flow: ContainerDAGFlow,
    sched: &mut Scheduler,
    runner: &KubernetesRunner,
) -> Result<usize, FlowError> {
    let plan = construct_plan(&flow.tasks)?;

    let flow_id = sched.create_flow(flow.name, plan, flow.tasks);

    if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
        runner.run_tasks(tasks);
    };

    return Ok(flow_id);
}

fn schedule_tasks(
    status: RunnerStatus,
    sched: &mut Scheduler,
    runner: &KubernetesRunner,
) -> Result<(), FlowError> {
    // TODO log errors and continue on error

    for (flow_id, task_id) in status.finished {
        sched.mark_task_finished(flow_id, task_id)?;

        if let Some(tasks) = sched.schedule_next_stage(flow_id)? {
            runner.run_tasks(tasks);
        }
    }

    for (flow_id, task_id) in status.failed {
        sched.mark_task_failed(flow_id, task_id)?
    }

    return Ok(());
}

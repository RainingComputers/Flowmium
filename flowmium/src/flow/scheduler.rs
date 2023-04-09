use std::collections::BTreeSet;

use sqlx::{Pool, Postgres};

use super::{errors::FlowError, model::Task};

#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "flow_status")]
#[sqlx(rename_all = "lowercase")]
enum FlowStatus {
    Pending,
    Running,
    Success,
    Failed,
}

#[derive(Debug)]
pub struct FlowState {
    id: usize,
    plan: Vec<BTreeSet<usize>>,
    current_stage: usize,
    running_tasks: BTreeSet<usize>,
    finished_tasks: BTreeSet<usize>,
    failed_tasks: BTreeSet<usize>,
    task_definitions: Vec<Task>,
    flow_name: String,
    status: FlowStatus,
}

impl FlowState {
    fn create_flow_state(
        id: usize,
        flow_name: String,
        plan: Vec<BTreeSet<usize>>,
        task_definitions: Vec<Task>,
    ) -> FlowState {
        FlowState {
            id,
            plan,
            current_stage: 0,
            running_tasks: BTreeSet::new(),
            finished_tasks: BTreeSet::new(),
            failed_tasks: BTreeSet::new(),
            task_definitions,
            flow_name,
            status: FlowStatus::Pending,
        }
    }

    pub fn running_tasks(&self) -> BTreeSet<usize> {
        return self.running_tasks.clone();
    }
}

#[derive(Debug)]
pub struct Scheduler {
    pub flow_runs: Vec<FlowState>,
    pub pool: Pool<Postgres>,
}

impl Scheduler {
    pub fn create_flow(
        &mut self,
        flow_name: String,
        plan: Vec<BTreeSet<usize>>,
        task_definitions: Vec<Task>,
    ) -> usize {
        let id = self.flow_runs.len();

        self.flow_runs.push(FlowState::create_flow_state(
            id,
            flow_name,
            plan,
            task_definitions,
        ));

        // INSERT INTO flows2 (
        //     plan,
        //     current_stage,
        //     running_tasks,
        //     finished_tasks,
        //     failed_tasks,
        //     task_definitions,
        //     flow_name,
        //     status
        // ) VALUES (
        //     '[[1,2],[3,4, 5],[6,7]]',
        //     1,
        //     '{1,2}',
        //     '{3,4}',
        //     '{}',
        //     '{"task1":{"type":"download","url":"http://example.com"},"task2":{"type":"process","input":"task1"}}',
        //     'example_flow',
        //     'pending'
        // );

        return id;
    }

    #[tracing::instrument]
    fn get_flow(&mut self, flow_id: usize) -> Result<&mut FlowState, FlowError> {
        let Some(flow) = self.flow_runs.get_mut(flow_id) else {
            tracing::error!("Flow does not exist");
            return Err(FlowError::FlowDoesNotExistError);
        };

        return Ok(flow);
    }

    pub fn mark_task_running(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let flow = self.get_flow(flow_id)?;

        (*flow).running_tasks.insert(task_id);

        // UPDATE flows2
        // SET running_tasks = running_tasks || 6
        // WHERE id = 1;

        return Ok(());
    }

    pub fn mark_task_finished(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let flow = self.get_flow(flow_id)?;

        (*flow).running_tasks.remove(&task_id);
        (*flow).finished_tasks.insert(task_id);

        if flow.finished_tasks.len() == flow.task_definitions.len() {
            flow.status = FlowStatus::Success;
        }

        return Ok(());

        // UPDATE flows2
        // SET running_tasks = array_remove(running_tasks, 5),
        // finished_tasks = finished_tasks || 5,
        // status =
        //         case
        //             when json_array_length(task_definitions) - 1 = cardinality(finished_tasks)  then 'success'::flow_status
        //             else status
        //         end
        // WHERE id = 1;
    }

    pub fn mark_task_failed(&mut self, flow_id: usize, task_id: usize) -> Result<(), FlowError> {
        let flow = self.get_flow(flow_id)?;

        (*flow).running_tasks.remove(&task_id);
        (*flow).failed_tasks.insert(task_id);

        (*flow).status = FlowStatus::Failed;

        // UPDATE flows
        // SET running_tasks = array_remove(running_tasks, 5),
        //     failed_tasks = failed_tasks || 5,
        //     status       = 'failed'::flow_status
        // WHERE id = 1;

        return Ok(());
    }

    pub fn get_running_or_pending_flows(&self) -> Vec<(usize, BTreeSet<usize>)> {
        return self
            .flow_runs
            .iter()
            .filter(|flow| flow.status == FlowStatus::Running || flow.status == FlowStatus::Pending)
            .map(|flow| (flow.id, flow.running_tasks()))
            .collect();

        // SELECT id, running_tasks
        // FROM flows
        // WHERE status IN ('running', 'pending');
    }

    fn stage_to_tasks(stage: &BTreeSet<usize>, task_definitions: &Vec<Task>) -> Vec<(usize, Task)> {
        stage
            .iter()
            .map(|id| (*id, task_definitions[*id].clone()))
            .collect()
    }

    #[tracing::instrument]
    pub async fn schedule_tasks<'a>(
        &'a mut self,
        flow_id: usize,
    ) -> Result<Option<Vec<(usize, Task)>>, FlowError> {
        let flow = self.get_flow(flow_id)?;

        if flow.status != FlowStatus::Running && flow.status != FlowStatus::Pending {
            return Ok(None);
        }

        let Some(stage) = flow.plan.get(flow.current_stage) else {
            tracing::error!("Stage {} does not exist", flow.current_stage);
            return Err(FlowError::StageDoesNotExistError);
        };

        if !stage.is_subset(&flow.finished_tasks) && flow.status == FlowStatus::Running {
            return Ok(None);
        }

        if flow.status == FlowStatus::Pending {
            (*flow).status = FlowStatus::Running;
        } else {
            (*flow).current_stage += 1;
        }

        let Some(next_stage) = flow.plan.get(flow.current_stage) else {
            tracing::error!("Stage {} does not exist", flow.current_stage);
            return Err(FlowError::StageDoesNotExistError);
        };

        let tasks = Scheduler::stage_to_tasks(next_stage, &flow.task_definitions);

        // TODO: flow_status type
        let rows = sqlx::query!(r#"
            WITH updated AS (
                UPDATE flows
                SET 
                    current_stage = 
                        CASE 
                            WHEN status = 'running'::flow_status THEN current_stage + 1
                            ELSE current_stage 
                        END,
                    status =
                        CASE 
                            WHEN status = 'pending'::flow_status THEN 'running'::flow_status
                            ELSE status 
                        END
                WHERE (finished_tasks @> array(SELECT json_array_elements_text((plan -> current_stage)::json) :: integer) OR status = 'pending')
                AND current_stage < json_array_length(plan) - 1
                AND id = 1
                AND status IN ('running', 'pending')
                RETURNING  *
            ) SELECT plan -> current_stage AS task_id_list, status::text FROM updated;
        "#).fetch_all(&self.pool).await.unwrap();

        tracing::info!("RECORD IS {:?}", rows);

        return Ok(Some(tasks));
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use crate::flow::model::Task;
    use std::collections::BTreeSet;

    use super::*;

    fn create_fake_task(task_name: &str) -> Task {
        Task {
            name: task_name.to_string(),
            image: "".to_string(),
            depends: vec![], // No need to fill because of forced fake plan
            cmd: vec![],
            env: vec![],
            inputs: None,
            outputs: None,
        }
    }

    #[tokio::test]
    async fn test_scheduler() {
        // TODO: Clean DB code
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://flowmium:flowmium@localhost/flowmium")
            .await
            .unwrap();

        let test_tasks_0 = vec![
            create_fake_task("flow-0-task-0"),
            create_fake_task("flow-0-task-1"),
            create_fake_task("flow-0-task-2"),
            create_fake_task("flow-0-task-3"),
        ];

        let test_plan_0 = vec![
            BTreeSet::from([0]),
            BTreeSet::from([1, 2]),
            BTreeSet::from([3]),
        ];

        let test_tasks_1 = vec![
            create_fake_task("flow-1-task-0"),
            create_fake_task("flow-1-task-1"),
            create_fake_task("flow-1-task-2"),
        ];

        let test_plan_1 = vec![
            BTreeSet::from([0]),
            BTreeSet::from([1]),
            BTreeSet::from([2]),
        ];

        let mut scheduler = Scheduler {
            flow_runs: vec![],
            pool,
        };

        let flow_id_0 = scheduler.create_flow("flow-0".to_string(), test_plan_0, test_tasks_0);
        let flow_id_1 = scheduler.create_flow("flow-1".to_string(), test_plan_1, test_tasks_1);

        assert_eq!(
            scheduler.get_running_or_pending_flows(),
            vec![
                (flow_id_0, BTreeSet::from([])),
                (flow_id_1, BTreeSet::from([]))
            ],
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![(0, create_fake_task("flow-0-task-0"))])),
        );

        scheduler.mark_task_running(0, 0).unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        scheduler.mark_task_finished(0, 0).unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![
                (1, create_fake_task("flow-0-task-1")),
                (2, create_fake_task("flow-0-task-2"))
            ])),
        );

        scheduler.mark_task_running(0, 1).unwrap();
        scheduler.mark_task_running(0, 2).unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flows(),
            vec![
                (flow_id_0, BTreeSet::from([1, 2])),
                (flow_id_1, BTreeSet::from([]))
            ],
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        scheduler.mark_task_finished(0, 1).unwrap();
        scheduler.mark_task_finished(0, 2).unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![(3, create_fake_task("flow-0-task-3")),])),
        );

        scheduler.mark_task_finished(0, 3).unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        assert_eq!(
            scheduler.get_running_or_pending_flows(),
            vec![(flow_id_1, BTreeSet::from([]))],
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_1).await,
            Ok(Some(vec![(0, create_fake_task("flow-1-task-0"))])),
        );

        scheduler.mark_task_running(1, 0).unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flows(),
            vec![(flow_id_1, BTreeSet::from([0]))],
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await, Ok(None));

        scheduler.mark_task_failed(1, 0).unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await, Ok(None));

        assert_eq!(scheduler.get_running_or_pending_flows(), vec![]);
    }

    #[tokio::test]
    async fn test_scheduler_flow_does_not_exist() {
        // TODO: Clean DB code
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://flowmium:flowmium@localhost/flowmium")
            .await
            .unwrap();

        let test_tasks_0 = vec![
            create_fake_task("flow-0-task-0"),
            create_fake_task("flow-0-task-1"),
        ];

        let test_plan_0 = vec![BTreeSet::from([0]), BTreeSet::from([1])];

        let test_tasks_1 = vec![
            create_fake_task("flow-1-task-0"),
            create_fake_task("flow-1-task-1"),
        ];

        let test_plan_1 = vec![BTreeSet::from([0]), BTreeSet::from([1])];

        let mut scheduler = Scheduler {
            flow_runs: vec![],
            pool,
        };

        scheduler.create_flow("flow-0".to_string(), test_plan_0, test_tasks_0);
        scheduler.create_flow("flow-1".to_string(), test_plan_1, test_tasks_1);

        assert_eq!(
            scheduler.mark_task_running(1000, 0),
            Err(FlowError::FlowDoesNotExistError)
        );

        assert_eq!(
            scheduler.mark_task_finished(1000, 0),
            Err(FlowError::FlowDoesNotExistError)
        );

        assert_eq!(
            scheduler.mark_task_failed(1000, 0),
            Err(FlowError::FlowDoesNotExistError)
        );

        assert_eq!(
            scheduler.schedule_tasks(1000).await,
            Err(FlowError::FlowDoesNotExistError)
        );
    }
}

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::collections::BTreeSet;

use crate::{server::record::FlowListRecord, server::record::FlowRecord};
use tokio::sync::broadcast;

use super::{model::Task, planner::Plan, pool::check_rows_updated, record::TaskStatus};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("invalid stored value error for flow: {0}")]
    InvalidStoredValue(i32),
    #[error("database query error: {0}")]
    DatabaseQuery(#[source] sqlx::error::Error),
    #[error("flow {0} does not exist error")]
    FlowDoesNotExist(i32),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum SchedulerEvent {
    TaskStatusUpdateEvent {
        flow_id: i32,
        task_id: i32,
        status: TaskStatus,
    },
    FlowCreatedEvent {
        flow_id: i32,
    },
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    pool: Pool<Postgres>,
    tx: broadcast::Sender<SchedulerEvent>,
}

impl Scheduler {
    pub fn new(pool: Pool<Postgres>) -> Self {
        let (tx, _rx) = broadcast::channel(1024);

        Self { pool, tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.tx.subscribe()
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn create_flow(
        &self,
        flow_name: String,
        plan: Plan,
        task_definitions: Vec<Task>,
    ) -> Result<i32, SchedulerError> {
        // Task does not have custom impl of Serialize or a key that is not a string
        let task_definitions =
            serde_json::to_value(task_definitions).expect("Failed to serialize task");

        // Plan does not have custom impl of Serialize or a key that is not a string
        let plan = serde_json::to_value(plan).expect("Failed to serialize plan");

        let query = r#"
        INSERT INTO flows (
            plan,
            current_stage, running_tasks, finished_tasks, failed_tasks,
            task_definitions, flow_name, status
        ) VALUES (
            $1,
            0, '{}', '{}', '{}',
            $2, $3, 'pending'
        ) RETURNING id;
        "#;

        let id: i32 = match sqlx::query_as(query)
            .bind(plan)
            .bind(task_definitions)
            .bind(flow_name)
            .fetch_one(&self.pool)
            .await
            .map(|record: (i32,)| record.0)
        {
            Ok(id) => id,
            Err(error) => {
                tracing::error!(%error, "Error creating flow in database");
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        let _ = self
            .tx
            .send(SchedulerEvent::FlowCreatedEvent { flow_id: id });

        Ok(id)
    }

    async fn run_mark_query(
        &self,
        flow_id: i32,
        task_id: i32,
        status: TaskStatus,
        query: &'static str,
    ) -> Result<(), SchedulerError> {
        let rows_updated = match sqlx::query(query)
            .bind(task_id)
            .bind(flow_id)
            .execute(&self.pool)
            .await
        {
            Ok(result) => result.rows_affected(),
            Err(error) => {
                tracing::error!(%error, "Unable to mark flow {} task {} as {} in database", flow_id, task_id, status);
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        check_rows_updated(rows_updated, SchedulerError::FlowDoesNotExist(flow_id))?;

        let _ = self.tx.send(SchedulerEvent::TaskStatusUpdateEvent {
            flow_id,
            task_id,
            status,
        });

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn mark_task_running(
        &self,
        flow_id: i32,
        task_id: i32,
    ) -> Result<(), SchedulerError> {
        let query = r#"
        UPDATE flows
        SET 
            running_tasks = array_append(running_tasks, $1),
            status       = 'running'::flow_status
        WHERE id = $2;
        "#;

        self.run_mark_query(flow_id, task_id, TaskStatus::Running, query)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn mark_task_finished(
        &self,
        flow_id: i32,
        task_id: i32,
    ) -> Result<(), SchedulerError> {
        let query = r#"
        UPDATE flows
        SET running_tasks = array_remove(running_tasks, $1),
            finished_tasks = array_append(finished_tasks, $1),
        status =
                case
                    when json_array_length(task_definitions) - 1 = cardinality(finished_tasks)  then 'success'::flow_status
                    else status
                end
        WHERE id = $2;
        "#;

        self.run_mark_query(flow_id, task_id, TaskStatus::Finished, query)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn mark_task_failed(
        &self,
        flow_id: i32,
        task_id: i32,
    ) -> Result<(), SchedulerError> {
        let query = r#"
        UPDATE flows
        SET running_tasks = array_remove(running_tasks, $1),
            failed_tasks = array_append(failed_tasks, $1),
            status       = 'failed'::flow_status
        WHERE id = $2;
        "#;

        self.run_mark_query(flow_id, task_id, TaskStatus::Failed, query)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_flows(&self) -> Result<Vec<FlowListRecord>, SchedulerError> {
        let query = r#"
        SELECT 
            id, flow_name, status, 
            array_length(running_tasks, 1) AS num_running, 
            array_length(finished_tasks, 1) AS num_finished, 
            array_length(failed_tasks, 1) AS num_failed,
            json_array_length(task_definitions) AS num_total
        FROM flows
        ORDER BY id ASC
        LIMIT 1000;
        "#;

        let flows: Vec<FlowListRecord> = match sqlx::query_as(query).fetch_all(&self.pool).await {
            Ok(flows) => flows,
            Err(error) => {
                tracing::error!(%error, "Unable to list flows on database");
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        Ok(flows)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_terminated_flows(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<FlowListRecord>, SchedulerError> {
        let query = r#"
        SELECT 
            id, flow_name, status, 
            array_length(running_tasks, 1) AS num_running, 
            array_length(finished_tasks, 1) AS num_finished, 
            array_length(failed_tasks, 1) AS num_failed,
            json_array_length(task_definitions) AS num_total
        FROM flows
        WHERE status IN ('success', 'failed')
        ORDER BY id ASC
        OFFSET $1
        LIMIT $2;
        "#;

        let flows: Vec<FlowListRecord> = match sqlx::query_as(query)
            .bind(offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        {
            Ok(flows) => flows,
            Err(error) => {
                tracing::error!(%error, "Unable to fetch terminated flows from database");
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        Ok(flows)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_flow(&self, id: i32) -> Result<FlowRecord, SchedulerError> {
        let query = r#"
        SELECT 
            id, plan, current_stage, running_tasks, finished_tasks, failed_tasks,
            task_definitions, flow_name, status
        FROM flows
        WHERE id = $1
        "#;

        let flow_optional: Option<FlowRecord> = match sqlx::query_as(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(flows) => flows,
            Err(error) => {
                tracing::error!(%error, "Unable to fetch terminated flows from database");
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        match flow_optional {
            None => Err(SchedulerError::FlowDoesNotExist(id)),
            Some(flow) => Ok(flow),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_running_or_pending_flow_ids(
        &self,
    ) -> Result<Vec<(i32, Vec<i32>)>, SchedulerError> {
        let query = r#"
        SELECT id, running_tasks
        FROM flows
        WHERE status IN ('running', 'pending')
        ORDER BY id ASC
        LIMIT 1000;
        "#;

        let flows: Vec<(i32, Vec<i32>)> = match sqlx::query_as(query).fetch_all(&self.pool).await {
            Ok(flows) => flows,
            Err(error) => {
                tracing::error!(%error, "Unable to fetch running or pending flows from database");
                return Err(SchedulerError::DatabaseQuery(error));
            }
        };

        Ok(flows)
    }

    fn record_to_tasks(
        task_id_list: Option<serde_json::Value>,
        tasks: serde_json::Value,
    ) -> Option<Vec<(i32, Task)>> {
        let Ok(task_ids) = serde_json::from_value::<BTreeSet<i32>>(task_id_list?) else {
            return None;
        };

        let Ok(task_definitions) = serde_json::from_value::<Vec<Task>>(tasks) else {
            return None;
        };

        let task_defs_filtered = task_definitions
            .into_iter()
            .enumerate()
            .map(|(i, task)| (i as i32, task))
            .filter(|(i, _)| task_ids.contains(i))
            .collect();

        Some(task_defs_filtered)
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn schedule_tasks<'a>(
        &'a self,
        flow_id: i32,
    ) -> Result<Option<Vec<(i32, Task)>>, SchedulerError> {
        let query = r#"
        WITH updated AS (
            UPDATE flows
            SET current_stage = 
                    CASE 
                        WHEN status = 'running'::flow_status THEN current_stage + 1
                        ELSE current_stage 
                    END
            WHERE (finished_tasks @> array(SELECT json_array_elements_text((plan -> current_stage)::json) :: integer) OR status = 'pending')
            AND current_stage < json_array_length(plan) - 1
            AND id = $1
            AND status IN ('running', 'pending')
            RETURNING  *
        ) SELECT plan -> current_stage AS "task_id_list", task_definitions AS "tasks" FROM updated;
        "#;

        let record: Option<(Option<serde_json::Value>, serde_json::Value)> =
            match sqlx::query_as(query)
                .bind(flow_id)
                .fetch_optional(&self.pool)
                .await
            {
                Ok(tasks) => tasks,
                Err(error) => {
                    tracing::error!(%error, "Unable to fetch next stage from database");
                    return Err(SchedulerError::DatabaseQuery(error));
                }
            };

        let Some(record) = record else {
            return Ok(None);
        };

        let tasks = Scheduler::record_to_tasks(record.0, record.1);

        let Some(tasks) = tasks else {
            tracing::error!("Invalid record in database for flow {}", flow_id);
            return Err(SchedulerError::InvalidStoredValue(flow_id));
        };

        Ok(Some(tasks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{model::Task, pool::get_test_pool, record::FlowStatus};
    use serial_test::serial;
    use std::collections::BTreeSet;

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

    fn create_task_status_update_event(
        flow_id: i32,
        task_id: i32,
        status: TaskStatus,
    ) -> SchedulerEvent {
        SchedulerEvent::TaskStatusUpdateEvent {
            flow_id,
            task_id,
            status,
        }
    }

    async fn setup_mock_data(scheduler: &Scheduler) -> (i32, i32) {
        let test_tasks_0 = vec![
            create_fake_task("flow-0-task-0"),
            create_fake_task("flow-0-task-1"),
            create_fake_task("flow-0-task-2"),
            create_fake_task("flow-0-task-3"),
        ];

        let test_plan_0 = Plan(vec![
            BTreeSet::from([0]),
            BTreeSet::from([1, 2]),
            BTreeSet::from([3]),
        ]);

        let test_tasks_1 = vec![
            create_fake_task("flow-1-task-0"),
            create_fake_task("flow-1-task-1"),
            create_fake_task("flow-1-task-2"),
        ];

        let test_plan_1 = Plan(vec![
            BTreeSet::from([0]),
            BTreeSet::from([1]),
            BTreeSet::from([2]),
        ]);

        let flow_id_0 = scheduler
            .create_flow("flow-0".to_string(), test_plan_0, test_tasks_0)
            .await
            .unwrap();

        let flow_id_1 = scheduler
            .create_flow("flow-1".to_string(), test_plan_1, test_tasks_1)
            .await
            .unwrap();

        (flow_id_0, flow_id_1)
    }

    #[tokio::test]
    #[serial]
    async fn test_scheduler() {
        let pool = get_test_pool(&["flows"]).await;
        let scheduler = Scheduler::new(pool);
        let mut rx = scheduler.subscribe();

        let (flow_id_0, flow_id_1) = setup_mock_data(&scheduler).await;

        assert_eq!(
            scheduler.get_running_or_pending_flow_ids().await.unwrap(),
            vec![(flow_id_0, vec![]), (flow_id_1, vec![])],
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await.unwrap(),
            Some(vec![(0, create_fake_task("flow-0-task-0"))]),
        );

        scheduler.mark_task_running(flow_id_0, 0).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await.unwrap(), None);

        scheduler.mark_task_finished(flow_id_0, 0).await.unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await.unwrap(),
            Some(vec![
                (1, create_fake_task("flow-0-task-1")),
                (2, create_fake_task("flow-0-task-2"))
            ]),
        );

        scheduler.mark_task_running(flow_id_0, 1).await.unwrap();
        scheduler.mark_task_running(flow_id_0, 2).await.unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flow_ids().await.unwrap(),
            vec![(flow_id_0, vec![1, 2]), (flow_id_1, vec![])],
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await.unwrap(), None);

        scheduler.mark_task_finished(flow_id_0, 1).await.unwrap();
        scheduler.mark_task_finished(flow_id_0, 2).await.unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await.unwrap(),
            Some(vec![(3, create_fake_task("flow-0-task-3")),]),
        );

        scheduler.mark_task_running(flow_id_0, 3).await.unwrap();
        scheduler.mark_task_finished(flow_id_0, 3).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await.unwrap(), None);

        assert_eq!(
            scheduler.get_running_or_pending_flow_ids().await.unwrap(),
            vec![(flow_id_1, vec![])],
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_1).await.unwrap(),
            Some(vec![(0, create_fake_task("flow-1-task-0"))]),
        );

        scheduler.mark_task_running(flow_id_1, 0).await.unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flow_ids().await.unwrap(),
            vec![(flow_id_1, vec![0])],
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await.unwrap(), None);

        scheduler.mark_task_failed(flow_id_1, 0).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await.unwrap(), None);

        assert_eq!(
            scheduler.get_running_or_pending_flow_ids().await.unwrap(),
            vec![]
        );

        let expected_events = vec![
            SchedulerEvent::FlowCreatedEvent { flow_id: flow_id_0 },
            SchedulerEvent::FlowCreatedEvent { flow_id: flow_id_1 },
            create_task_status_update_event(flow_id_0, 0, TaskStatus::Running),
            create_task_status_update_event(flow_id_0, 0, TaskStatus::Finished),
            create_task_status_update_event(flow_id_0, 1, TaskStatus::Running),
            create_task_status_update_event(flow_id_0, 2, TaskStatus::Running),
            create_task_status_update_event(flow_id_0, 1, TaskStatus::Finished),
            create_task_status_update_event(flow_id_0, 2, TaskStatus::Finished),
            create_task_status_update_event(flow_id_0, 3, TaskStatus::Running),
            create_task_status_update_event(flow_id_0, 3, TaskStatus::Finished),
            create_task_status_update_event(flow_id_1, 0, TaskStatus::Running),
            create_task_status_update_event(flow_id_1, 0, TaskStatus::Failed),
        ];

        for event in expected_events {
            assert_eq!(rx.recv().await.unwrap(), event);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_scheduler_flow_does_not_exist() {
        let pool = get_test_pool(&["flows"]).await;
        let scheduler = Scheduler::new(pool);

        let (flow_id_0, _flow_id_1) = setup_mock_data(&scheduler).await;

        let does_not_exist_id = flow_id_0 + 1000;

        fn assert_flow_does_not_exist_error<T>(result: Result<T, SchedulerError>, flow_id: i32) {
            assert!(match result {
                Err(SchedulerError::FlowDoesNotExist(id)) => id == flow_id,
                _ => false,
            })
        }

        assert_flow_does_not_exist_error(
            scheduler.mark_task_running(does_not_exist_id, 0).await,
            does_not_exist_id,
        );

        assert_flow_does_not_exist_error(
            scheduler.mark_task_finished(does_not_exist_id, 0).await,
            does_not_exist_id,
        );

        assert_flow_does_not_exist_error(
            scheduler.mark_task_failed(does_not_exist_id, 0).await,
            does_not_exist_id,
        );

        assert_flow_does_not_exist_error(
            scheduler.get_flow(does_not_exist_id).await,
            does_not_exist_id,
        );

        assert_eq!(
            scheduler.schedule_tasks(does_not_exist_id).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_scheduler_get() {
        let pool = get_test_pool(&["flows"]).await;
        let scheduler = Scheduler::new(pool);

        assert_eq!(scheduler.list_flows().await.unwrap(), vec![]);
        assert_eq!(
            scheduler.list_terminated_flows(0, 1000).await.unwrap(),
            vec![]
        );

        let (flow_id_0, flow_id_1) = setup_mock_data(&scheduler).await;

        assert_eq!(
            scheduler.list_flows().await.unwrap(),
            vec![
                FlowListRecord {
                    id: flow_id_0,
                    flow_name: "flow-0".to_string(),
                    status: FlowStatus::Pending,
                    num_running: None,
                    num_finished: None,
                    num_failed: None,
                    num_total: Some(4),
                },
                FlowListRecord {
                    id: flow_id_1,
                    flow_name: "flow-1".to_string(),
                    status: FlowStatus::Pending,
                    num_running: None,
                    num_finished: None,
                    num_failed: None,
                    num_total: Some(3),
                }
            ]
        );

        scheduler.mark_task_running(flow_id_0, 0).await.unwrap();
        scheduler.mark_task_failed(flow_id_1, 0).await.unwrap();

        assert_eq!(
            scheduler.get_flow(flow_id_1).await.unwrap(),
            FlowRecord {
                id: flow_id_1,
                flow_name: "flow-1".to_string(),
                status: FlowStatus::Failed,
                plan: serde_json::json!([[0], [1], [2]]),
                current_stage: 0,
                running_tasks: vec![],
                finished_tasks: vec![],
                failed_tasks: vec![0],
                task_definitions: serde_json::to_value(vec![
                    create_fake_task("flow-1-task-0"),
                    create_fake_task("flow-1-task-1"),
                    create_fake_task("flow-1-task-2"),
                ])
                .unwrap(),
            }
        );

        assert_eq!(
            scheduler.get_flow(flow_id_0).await.unwrap(),
            FlowRecord {
                id: flow_id_0,
                flow_name: "flow-0".to_string(),
                status: FlowStatus::Running,
                plan: serde_json::json!([[0], [1, 2], [3]]),
                current_stage: 0,
                running_tasks: vec![0],
                finished_tasks: vec![],
                failed_tasks: vec![],
                task_definitions: serde_json::to_value(vec![
                    create_fake_task("flow-0-task-0"),
                    create_fake_task("flow-0-task-1"),
                    create_fake_task("flow-0-task-2"),
                    create_fake_task("flow-0-task-3"),
                ])
                .unwrap(),
            }
        );

        assert_eq!(
            scheduler.list_terminated_flows(0, 1000).await.unwrap(),
            vec![FlowListRecord {
                id: flow_id_1,
                flow_name: "flow-1".to_string(),
                status: FlowStatus::Failed,
                num_running: None,
                num_finished: None,
                num_failed: Some(1),
                num_total: Some(3),
            }]
        );
    }
}

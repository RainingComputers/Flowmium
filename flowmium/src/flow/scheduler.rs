use std::collections::BTreeSet;

use serde_json::Value;
use sqlx::{Pool, Postgres};

use super::{errors::FlowError, model::Task};

#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(rename_all = "snake_case")]
enum FlowStatus {
    Pending,
    Running,
    Success,
    Failed,
}

// #[derive(Debug)]
// pub struct FlowState {
//     id: usize,
//     plan: Vec<BTreeSet<usize>>,
//     current_stage: usize,
//     running_tasks: BTreeSet<usize>,
//     finished_tasks: BTreeSet<usize>,
//     failed_tasks: BTreeSet<usize>,
//     task_definitions: Vec<Task>,
//     flow_name: String,
//     status: FlowStatus,
// }

// TODO: table as env variable

#[derive(Debug)]
pub struct Scheduler {
    pub pool: Pool<Postgres>,
}

impl Scheduler {
    pub async fn create_flow(
        &mut self,
        flow_name: String,
        plan: Vec<BTreeSet<usize>>,
        task_definitions: Vec<Task>,
    ) -> Result<i32, FlowError> {
        let Ok(task_definitions_json) = serde_json::to_value(task_definitions) else {
            return Err(FlowError::StoreError); // TODO log and diff error
        };

        let Ok(plan_json) = serde_json::to_value(plan) else {
            return Err(FlowError::StoreError); // TODO log and diff error
        };

        let Ok(id) = sqlx::query!(
            r#"
            INSERT INTO flows (
                plan,
                current_stage,
                running_tasks,
                finished_tasks,
                failed_tasks,
                task_definitions,
                flow_name,
                status
            ) VALUES (
                $1,
                0,
                '{}',
                '{}',
                '{}',
                $2,
                $3,
                'pending'
            ) RETURNING id;
            "#,
            plan_json,
            task_definitions_json,
            flow_name
        ).map(|record| record.id)
        .fetch_one(&self.pool)
        .await else {
            return Err(FlowError::StoreError); // TODO log
        };

        return Ok(id);
    }

    pub async fn mark_task_running(&mut self, flow_id: i32, task_id: i32) -> Result<(), FlowError> {
        if let Err(_) = sqlx::query!(
            r#"
            UPDATE flows
            SET running_tasks = array_append(running_tasks, $1)
            WHERE id = $2;
            "#,
            task_id,
            flow_id
        )
        .execute(&self.pool)
        .await
        {
            return Err(FlowError::StoreError); // TODO log
        };

        return Ok(());
    }

    pub async fn mark_task_finished(
        &mut self,
        flow_id: i32,
        task_id: i32,
    ) -> Result<(), FlowError> {
        if let Err(_) = sqlx::query!(
            r#"
            UPDATE flows
            SET running_tasks = array_remove(running_tasks, $1),
            finished_tasks = array_append(finished_tasks, $1),
            status =
                    case
                        when json_array_length(task_definitions) - 1 = cardinality(finished_tasks)  then 'success'::flow_status
                        else status
                    end
            WHERE id = $2;
            "#,
            task_id,
            flow_id
        )
        .execute(&self.pool)
        .await
        {
            return Err(FlowError::StoreError); // TODO log
        }; // TODO: check if one row was updated

        return Ok(());
    }

    pub async fn mark_task_failed(&mut self, flow_id: i32, task_id: i32) -> Result<(), FlowError> {
        if let Err(_) = sqlx::query!(
            r#"
            UPDATE flows
            SET running_tasks = array_remove(running_tasks, $1),
                failed_tasks = array_append(failed_tasks, $1),
                status       = 'failed'::flow_status
            WHERE id = $2;
            "#,
            task_id,
            flow_id
        )
        .execute(&self.pool)
        .await
        {
            return Err(FlowError::StoreError); // TODO log
        };

        return Ok(());
    }

    pub async fn get_running_or_pending_flows(&self) -> Result<Vec<(i32, Vec<i32>)>, FlowError> {
        struct RunningPendingSelectQueryRecord {
            id: i32,
            running_tasks: Vec<i32>,
        }

        let Ok(flows) = sqlx::query_as!(
            RunningPendingSelectQueryRecord,
            r#"
            SELECT id, running_tasks
            FROM flows
            WHERE status IN ('running', 'pending')
            ORDER BY id ASC;
            "#
        ).map(|record| (record.id, record.running_tasks))
        .fetch_all(&self.pool)
        .await else {
            return Err(FlowError::StoreError); // TODO log
        };

        return Ok(flows);
    }

    fn record_to_tasks(task_id_list: Option<Value>, tasks: Value) -> Option<Vec<(i32, Task)>> {
        let Ok(task_ids) = serde_json::from_value::<BTreeSet<i32>>(task_id_list?) else {
            return  None;
        };

        let Ok(task_definitions) = serde_json::from_value::<Vec<Task>>(tasks) else {
            return  None;
        };

        let task_defs_filtered = task_definitions
            .into_iter()
            .enumerate()
            .map(|(i, task)| (i as i32, task))
            .filter(|(i, _)| task_ids.contains(i))
            .collect();

        return Some(task_defs_filtered);
    }

    #[tracing::instrument]
    pub async fn schedule_tasks<'a>(
        &'a mut self,
        flow_id: i32,
    ) -> Result<Option<Vec<(i32, Task)>>, FlowError> {
        let record_optional = match sqlx::query!(
            r#"
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
                AND id = $1
                AND status IN ('running', 'pending')
                RETURNING  *
            ) SELECT plan -> current_stage AS "task_id_list", task_definitions AS "tasks" FROM updated;
            "#, 
            flow_id
        ).map(|record| Scheduler::record_to_tasks(record.task_id_list, record.tasks))
        .fetch_optional(&self.pool)
        .await {
            Ok(tasks) => tasks,
            Err(err)  => {
                return Err(FlowError::StoreError); // TODO: log
            }
        };

        let Some(stage_tasks_optional) = record_optional else {
            return Ok(None); 
        };

        let Some(stage_tasks) = stage_tasks_optional else {
            return Err(FlowError::InvalidStoredValueError); // TODO: log
        };

        return Ok(Some(stage_tasks));
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
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
    #[serial]
    async fn test_scheduler() {
        // TODO: Clean DB code
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://flowmium:flowmium@localhost/flowmium")
            .await
            .unwrap();

        sqlx::query!("DELETE from flows;").execute(&pool).await.unwrap();

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

        let mut scheduler = Scheduler { pool };

        let flow_id_0 = scheduler
            .create_flow("flow-0".to_string(), test_plan_0, test_tasks_0)
            .await
            .unwrap();
        let flow_id_1 = scheduler
            .create_flow("flow-1".to_string(), test_plan_1, test_tasks_1)
            .await
            .unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flows().await,
            Ok(vec![(flow_id_0, vec![]), (flow_id_1, vec![])]),
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![(0, create_fake_task("flow-0-task-0"))])),
        );

        scheduler.mark_task_running(flow_id_0, 0).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        scheduler.mark_task_finished(flow_id_0, 0).await.unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![
                (1, create_fake_task("flow-0-task-1")),
                (2, create_fake_task("flow-0-task-2"))
            ])),
        );

        scheduler.mark_task_running(flow_id_0, 1).await.unwrap();
        scheduler.mark_task_running(flow_id_0, 2).await.unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flows().await,
            Ok(vec![(flow_id_0, vec![1, 2]), (flow_id_1, vec![])]),
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        scheduler.mark_task_finished(flow_id_0, 1).await.unwrap();
        scheduler.mark_task_finished(flow_id_0, 2).await.unwrap();

        assert_eq!(
            scheduler.schedule_tasks(flow_id_0).await,
            Ok(Some(vec![(3, create_fake_task("flow-0-task-3")),])),
        );

        scheduler.mark_task_finished(flow_id_0, 3).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_0).await, Ok(None));

        assert_eq!(
            scheduler.get_running_or_pending_flows().await,
            Ok(vec![(flow_id_1, vec![])]),
        );

        assert_eq!(
            scheduler.schedule_tasks(flow_id_1).await,
            Ok(Some(vec![(0, create_fake_task("flow-1-task-0"))])),
        );

        scheduler.mark_task_running(flow_id_1, 0).await.unwrap();

        assert_eq!(
            scheduler.get_running_or_pending_flows().await,
            Ok(vec![(flow_id_1, vec![0])]),
        );

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await, Ok(None));

        scheduler.mark_task_failed(flow_id_1, 0).await.unwrap();

        assert_eq!(scheduler.schedule_tasks(flow_id_1).await, Ok(None));

        assert_eq!(scheduler.get_running_or_pending_flows().await, Ok(vec![]));
    }

    // #[tokio::test]
    // async fn test_scheduler_flow_does_not_exist() {
    //     // TODO: Clean DB code
    //     let pool = PgPoolOptions::new()
    //         .max_connections(5)
    //         .connect("postgres://flowmium:flowmium@localhost/flowmium")
    //         .await
    //         .unwrap();

    //     let test_tasks_0 = vec![
    //         create_fake_task("flow-0-task-0"),
    //         create_fake_task("flow-0-task-1"),
    //     ];

    //     let test_plan_0 = vec![BTreeSet::from([0]), BTreeSet::from([1])];

    //     let test_tasks_1 = vec![
    //         create_fake_task("flow-1-task-0"),
    //         create_fake_task("flow-1-task-1"),
    //     ];

    //     let test_plan_1 = vec![BTreeSet::from([0]), BTreeSet::from([1])];

    //     let mut scheduler = Scheduler { pool };

    //     scheduler.create_flow("flow-0".to_string(), test_plan_0, test_tasks_0);
    //     scheduler.create_flow("flow-1".to_string(), test_plan_1, test_tasks_1);

    //     assert_eq!(
    //         scheduler.mark_task_running(1000, 0).await,
    //         Err(FlowError::FlowDoesNotExistError)
    //     );

    //     assert_eq!(
    //         scheduler.mark_task_finished(1000, 0).await,
    //         Err(FlowError::FlowDoesNotExistError)
    //     );

    //     assert_eq!(
    //         scheduler.mark_task_failed(1000, 0).await,
    //         Err(FlowError::FlowDoesNotExistError)
    //     );

    //     assert_eq!(
    //         scheduler.schedule_tasks(1000).await,
    //         Err(FlowError::FlowDoesNotExistError)
    //     );
    // }
}

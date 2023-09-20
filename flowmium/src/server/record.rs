use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sqlx::{database::HasValueRef, Database};

use super::planner::Plan;

#[derive(sqlx::Type, Debug, PartialEq, Serialize, Deserialize, Clone)]
#[sqlx(rename_all = "snake_case")]
pub enum FlowStatus {
    Pending,
    Running,
    Success,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Failed,
    Finished,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]

pub struct FlowRecord {
    pub id: i32,
    pub flow_name: String,
    pub status: FlowStatus,
    pub plan: Plan,
    pub current_stage: i32,
    pub running_tasks: Vec<i32>,
    pub finished_tasks: Vec<i32>,
    pub failed_tasks: Vec<i32>,
    pub task_definitions: serde_json::Value,
}

impl<'r, DB: Database> sqlx::Decode<'r, DB> for Plan
where
    &'r str: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<Plan, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let json_value = <&str as sqlx::Decode<DB>>::decode(value)?;

        let stages: Vec<BTreeSet<usize>> = serde_json::from_str(json_value)?;

        Ok(Plan(stages))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FlowListRecord {
    pub id: i32,
    pub flow_name: String,
    pub status: FlowStatus,
    pub num_running: Option<i32>,
    pub num_finished: Option<i32>,
    pub num_failed: Option<i32>,
    pub num_total: Option<i32>,
}

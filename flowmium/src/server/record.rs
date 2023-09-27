use core::fmt;

use serde::{Deserialize, Serialize};

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

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Finished => write!(f, "finished"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, sqlx::FromRow)]
pub struct FlowRecord {
    pub id: i32,
    pub flow_name: String,
    pub status: FlowStatus,
    pub plan: serde_json::Value,
    pub current_stage: i32,
    pub running_tasks: Vec<i32>,
    pub finished_tasks: Vec<i32>,
    pub failed_tasks: Vec<i32>,
    pub task_definitions: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, sqlx::FromRow)]
pub struct FlowListRecord {
    pub id: i32,
    pub flow_name: String,
    pub status: FlowStatus,
    pub num_running: Option<i32>,
    pub num_finished: Option<i32>,
    pub num_failed: Option<i32>,
    pub num_total: Option<i32>,
}

use core::fmt;

use serde::{Deserialize, Serialize};

/// Status of a flow.
#[derive(sqlx::Type, Debug, PartialEq, Serialize, Deserialize, Clone)]
#[sqlx(rename_all = "snake_case", type_name = "flow_status")]
pub enum FlowStatus {
    /// Flow is yet to run. None of the task has been spawned yet.
    Pending,
    /// Flow is running and at least one of the task has been spawned.
    Running,
    /// Flow has finished successfully and all tasks in the flow has also been completed successfully.
    Success,
    /// Flow has been aborted with a failure because one of the tasks terminated with a failure.
    Failed,
}

/// Status of a task belonging to a flow.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is running.
    Running,
    /// Task has terminated with a failure.
    Failed,
    /// Task has finished successfully.
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

/// Detailed status of a flow.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, sqlx::FromRow)]
pub struct FlowRecord {
    /// Unique identifier for the flow.
    pub id: i32,
    /// Name of the flow as specified in [`crate::server::model::Flow`].
    pub flow_name: String,
    /// Status of the flow.
    pub status: FlowStatus,
    /// Execution plan of the flow. This is a nested 2D JSON array container integer elements.
    /// The integer elements refer to index of a task defined in [`crate::server::model::Flow`].
    /// A task is executed in multiple stages, where each stage is a set of tasks.
    /// The set of tasks in the last stage are dependent set of tasks in the last but second stage and so on,
    /// with the first stage having tasks that are independent having no dependencies (leaf tasks).
    /// Tasks belonging to the same stage are not dependent on each other, if a task is dependent on another task,
    /// they will belong to different stages.
    pub plan: serde_json::Value,
    /// Current stage that is running.
    /// Set of tasks in the same stage are spawned at the same time because they are not dependent on each other.
    pub current_stage: i32,
    /// Indices of tasks that are currently running.
    pub running_tasks: Vec<i32>,
    /// Indices of tasks that have finished.
    pub finished_tasks: Vec<i32>,
    /// Indices of tasks that have failed.
    pub failed_tasks: Vec<i32>,
    /// List of tasks that belong to this flow, as define in [`crate::server::model::Flow`].
    pub task_definitions: serde_json::Value,
}

/// Brief status summary of a flow.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, sqlx::FromRow)]
pub struct FlowListRecord {
    /// Unique identifier for the flow.
    pub id: i32,
    /// Name of the flow as specified in [`crate::server::model::Flow`].
    pub flow_name: String,
    /// Status of the flow.
    pub status: FlowStatus,
    /// Number of tasks belonging to this flow that are currently running.
    pub num_running: Option<i32>,
    /// Number of tasks belonging to this flow that have finished.
    pub num_finished: Option<i32>,
    /// Number of tasks belonging to this flow that have failed.
    pub num_failed: Option<i32>,
    /// Total number of tasks defined in the flow.
    pub num_total: Option<i32>,
}

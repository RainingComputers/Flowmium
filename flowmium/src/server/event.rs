use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use super::record::TaskStatus;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SchedulerEventResult {
    Event(SchedulerEvent),
    Lag(u64),
}

pub(crate) fn to_event_result(
    event: Result<SchedulerEvent, BroadcastStreamRecvError>,
) -> SchedulerEventResult {
    match event {
        Ok(event) => SchedulerEventResult::Event(event),
        Err(error) => match error {
            BroadcastStreamRecvError::Lagged(count) => SchedulerEventResult::Lag(count),
        },
    }
}

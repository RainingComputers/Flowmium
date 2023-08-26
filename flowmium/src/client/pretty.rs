use core::fmt;

use crate::flow::{
    record::FlowRecord,
    record::{FlowListRecord, FlowStatus},
};

use super::requests::{BytesDownloaded, FlowList, Okay};

impl fmt::Display for Okay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl fmt::Display for BytesDownloaded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Downloaded {} bytes", self.bytes())
    }
}

impl fmt::Display for FlowStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FlowStatus::Pending => write!(f, "PENDING"),
            FlowStatus::Running => write!(f, "RUNNING"),
            FlowStatus::Success => write!(f, "SUCCESS"),
            FlowStatus::Failed => write!(f, "FAILED"),
        }
    }
}

fn get_progress_string_from_rec(rec: &FlowListRecord) -> String {
    fn optional_to_str(opt: Option<i32>) -> String {
        match opt {
            None => "0".to_owned(),
            Some(val) => val.to_string(),
        }
    }

    format!(
        "{}/{}",
        optional_to_str(rec.num_finished),
        optional_to_str(rec.num_total)
    )
}

impl fmt::Display for FlowList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{: <8} {: <40} {: <12} {: <8}",
            "ID", "NAME", "PROGRESS", "STATUS"
        )?;

        for rec in self {
            writeln!(
                f,
                "{: <8} {: <40} {: <12} {: <8}",
                rec.id,
                rec.flow_name,
                get_progress_string_from_rec(rec),
                rec.status
            )?
        }

        Ok(())
    }
}

impl fmt::Display for FlowRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string_pretty(self).expect("Cannot serialize response to JSON")
        )
    }
}

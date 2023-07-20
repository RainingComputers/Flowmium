use core::fmt;

use crate::flow::scheduler::{FlowRecord, FlowStatus};

use super::requests::{BytesDownloaded, FlowList, Okay};

impl fmt::Display for Okay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl fmt::Display for BytesDownloaded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "Downloaded {} bytes", self.bytes());
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

impl fmt::Display for FlowList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: <6} {: <40} {: <8}", "ID", "NAME", "STATUS")?;

        for rec in self {
            write!(
                f,
                "{: <6} {: <40} {: <8}",
                rec.id(),
                rec.flow_name(),
                rec.status()
            )?
        }

        Ok(())
    }
}

impl fmt::Display for FlowRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

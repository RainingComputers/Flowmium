use crate::flow::scheduler::{FlowListRecord, FlowRecord};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("invalid url error: {0}")]
    InvalidUrlErr(
        #[source]
        #[from]
        url::ParseError,
    ),
    #[error("request error: {0}")]
    RequestErr(
        #[source]
        #[from]
        reqwest::Error,
    ),
}

fn get_abs_url(url: &str, path: &str) -> Result<Url, ClientError> {
    let base = Url::parse(url)?;
    let joined = base.join(path)?;

    Ok(joined)
}

pub async fn list_workflows(url: &str) -> Result<Vec<FlowListRecord>, ClientError> {
    let abs_url = get_abs_url(url, "/api/v1/job")?;

    Ok(reqwest::get(abs_url)
        .await?
        .json::<Vec<FlowListRecord>>()
        .await?)
}

pub async fn get_status(url: &str, id: &str) -> Result<FlowRecord, ClientError> {
    let abs_url = get_abs_url(url, &format!("/api/v1/job/{}", id))?;

    Ok(reqwest::get(abs_url).await?.json::<FlowRecord>().await?)
}

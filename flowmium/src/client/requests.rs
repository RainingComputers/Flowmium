use crate::flow::scheduler::{FlowListRecord, FlowRecord};
use getset::Getters;
use thiserror::Error;
use tokio_stream::StreamExt;
use url::Url;

use std::fs::File;
use std::io::copy;
use std::path::{Path, PathBuf};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("invalid url error: {0}")]
    InvalidUrl(
        #[source]
        #[from]
        url::ParseError,
    ),
    #[error("request error: {0}")]
    RequestError(
        #[source]
        #[from]
        reqwest::Error,
    ),
    #[error("response not ok error: {0}")]
    ResponseNotOk(String),
    #[error("io error: {0}")]
    IoError(
        #[source]
        #[from]
        std::io::Error,
    ),
    #[error("websocket error: {0}")]
    WebsocketError(
        #[source]
        #[from]
        tokio_tungstenite::tungstenite::Error,
    ),
    #[error("url scheme conversion error")]
    UrlSchemeConversionErr,
}

#[derive(Getters, Debug)]
pub struct FlowList {
    #[getset(get = "pub")]
    list: Vec<FlowListRecord>,
}

impl IntoIterator for FlowList {
    type Item = FlowListRecord;
    type IntoIter = std::vec::IntoIter<FlowListRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}

impl<'a> IntoIterator for &'a FlowList {
    type Item = &'a FlowListRecord;
    type IntoIter = std::slice::Iter<'a, FlowListRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

#[derive(Getters, Debug)]
pub struct BytesDownloaded {
    #[getset(get = "pub")]
    bytes: u64,
}

pub struct Okay();

fn get_abs_url(url: &str, path: &str) -> Result<Url, ClientError> {
    let base = Url::parse(url)?;
    let joined = base.join(path)?;

    Ok(joined)
}

pub async fn list_workflows(url: &str) -> Result<FlowList, ClientError> {
    let abs_url = get_abs_url(url, "/api/v1/job")?;

    Ok(FlowList {
        list: reqwest::get(abs_url)
            .await?
            .json::<Vec<FlowListRecord>>()
            .await?,
    })
}

pub async fn get_status(url: &str, id: &str) -> Result<FlowRecord, ClientError> {
    let abs_url = get_abs_url(url, &format!("/api/v1/job/{}", id))?;

    Ok(reqwest::get(abs_url).await?.json::<FlowRecord>().await?)
}

pub async fn response_to_result(response: reqwest::Response) -> Result<Okay, ClientError> {
    if response.status() != 200 {
        return Err(ClientError::ResponseNotOk(response.text().await?));
    }

    Ok(Okay())
}

pub async fn create_secret(url: &str, key: &str, value: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    response_to_result(client.post(abs_url).json::<str>(value).send().await?).await
}

pub async fn update_secret(url: &str, key: &str, value: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    response_to_result(client.put(abs_url).json::<str>(value).send().await?).await
}

pub async fn delete_secret(url: &str, key: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    response_to_result(client.delete(abs_url).send().await?).await
}

fn get_path_from_response_url(
    response: &reqwest::Response,
    dir_path: &str,
    default_name: &str,
) -> PathBuf {
    let file_name = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
        .unwrap_or(&default_name);

    Path::new(dir_path).join(file_name)
}

pub async fn download_artefact(
    url: &str,
    id: &str,
    name: &str,
    dest: &str,
) -> Result<BytesDownloaded, ClientError> {
    let abs_url = get_abs_url(url, &format!("/api/v1/artefact/{}/{}", id, name))?;

    let response = reqwest::get(abs_url).await?;

    let file_path = get_path_from_response_url(&response, dest, &format!("flow-{}-output", id));

    let content = response.text().await?;

    let mut file = File::create(file_path)?;

    let num_bytes = copy(&mut content.as_bytes(), &mut file)?;

    Ok(BytesDownloaded { bytes: num_bytes })
}

fn get_ws_scheme(secure: bool) -> &'static str {
    if secure {
        return "wss";
    }

    return "ws";
}

pub async fn subscribe<F>(url: &str, secure: bool, on_message: F) -> Result<Okay, ClientError>
where
    F: Fn(String),
{
    let mut abs_url = get_abs_url(url, "/api/v1/scheduler/ws")?;

    if let Err(_) = abs_url.set_scheme(get_ws_scheme(secure)) {
        return Err(ClientError::UrlSchemeConversionErr);
    };

    let (mut ws_stream, _) = tokio_tungstenite::connect_async(abs_url).await?;

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;

        if msg.is_text() {
            on_message(msg.to_string());
        }
    }

    Ok(Okay())
}

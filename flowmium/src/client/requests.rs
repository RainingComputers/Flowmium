use getset::Getters;
use reqwest::Response;
use thiserror::Error;
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::server::event::{SchedulerEvent, SchedulerEventResult};
use crate::server::model::Flow;
use crate::server::record::{FlowListRecord, FlowRecord};

/// An error while making a request to the server.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Unable to parse an invalid or malformed URL.
    #[error("invalid url error: {0}")]
    InvalidUrl(
        #[source]
        #[from]
        url::ParseError,
    ),
    /// Unable to make a request or connection or malformed response in body.
    #[error("request error: {0}")]
    Request(
        #[source]
        #[from]
        reqwest::Error,
    ),
    /// Request was sent but the server responded with non 200 HTTP status code.
    #[error("response {0} error: {1}")]
    ResponseNotOk(u16, String),
    /// Error performing file operations.
    #[error("io error: {0}")]
    Io(
        #[source]
        #[from]
        std::io::Error,
    ),
    /// Unable to edit the URL scheme into a `ws://` or `wss://`.
    #[error("url scheme conversion error")]
    UrlSchemeConversion,
    /// Unable to make a websocket connection.
    #[error("websocket error: {0}")]
    Websocket(
        #[source]
        #[from]
        tokio_tungstenite::tungstenite::Error,
    ),
}

/// An error while receiving events from websocket.
#[derive(Error, Debug)]
pub enum ClientWebsocketError {
    /// Unable to make a websocket connection or websocket connection closed unexpectedly.
    #[error("websocket error: {0}")]
    Websocket(
        #[source]
        #[from]
        tokio_tungstenite::tungstenite::Error,
    ),
    /// Invalid or malformed event from server.
    #[error("malformed event error: {0}")]
    MalformedEvent(serde_json::Error),
    /// Client or server is unable to keep up and some events might have been missed.
    #[error("lag error: {0}")]
    Lag(u64),
}

/// Wrapper type for [`Vec<FlowListRecord>`](FlowListRecord) with a pretty implementation for [`std::fmt::Display`].
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

/// New type for number of bytes downloaded with a pretty implementation for [`std::fmt::Display`].
#[derive(Getters, Debug)]
pub struct BytesDownloaded {
    #[getset(get = "pub")]
    num_bytes: u64,
}

/// Indicates the request was successful and the server responded with a 200 HTTP status code.
pub struct Okay();

fn get_abs_url(url: &str, path: &str) -> Result<Url, ClientError> {
    let base = Url::parse(url)?;
    let joined = base.join(path)?;

    Ok(joined)
}

/// List workflows and their status in the server.
pub async fn list_workflows(url: &str) -> Result<FlowList, ClientError> {
    let abs_url = get_abs_url(url, "/api/v1/job")?;

    Ok(FlowList {
        list: reqwest::get(abs_url)
            .await?
            .json::<Vec<FlowListRecord>>()
            .await?,
    })
}

/// Get more details status of a workflow, like the plan, number of running tasks etc.
pub async fn get_status(url: &str, id: &str) -> Result<FlowRecord, ClientError> {
    let abs_url = get_abs_url(url, &format!("/api/v1/job/{}", id))?;

    Ok(reqwest::get(abs_url).await?.json::<FlowRecord>().await?)
}

async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, ClientError> {
    let response_status = response.status();

    if response_status != 200 {
        return Err(ClientError::ResponseNotOk(
            response_status.as_u16(),
            response.text().await?,
        ));
    }

    Ok(response)
}

async fn check_status_take(response: reqwest::Response) -> Result<Okay, ClientError> {
    check_status(response).await?;
    Ok(Okay())
}

/// Create a secret in the server.
pub async fn create_secret(url: &str, key: &str, value: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    check_status_take(client.post(abs_url).json::<str>(value).send().await?).await
}

/// Update a secret in the server.
pub async fn update_secret(url: &str, key: &str, value: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    check_status_take(client.put(abs_url).json::<str>(value).send().await?).await
}

/// Delete a secret in the server.
pub async fn delete_secret(url: &str, key: &str) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, &format!("api/v1/secret/{}", key))?;

    let client = reqwest::Client::new();

    check_status_take(client.delete(abs_url).send().await?).await
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
        .unwrap_or(default_name);

    Path::new(dir_path).join(file_name)
}

/// Download artefact output of a task in a workflow.
pub async fn download_artefact(url: &str, id: &str, name: &str) -> Result<Response, ClientError> {
    let abs_url = get_abs_url(url, &format!("/api/v1/artefact/{}/{}", id, name))?;

    let response = reqwest::get(abs_url).await?;

    check_status(response).await
}

/// Download artefact output of a task in a workflow and save it to a directory path.
/// Here `name` is the name of the output as defined in the flow definition and `dest` is path to a directory.
pub async fn download_artefact_to_path(
    url: &str,
    id: &str,
    name: &str,
    dest: &str,
) -> Result<BytesDownloaded, ClientError> {
    let response = download_artefact(url, id, name).await?;

    let file_path = get_path_from_response_url(&response, dest, &format!("flow-{}-output", id));

    let content = response.text().await?;

    let mut file = File::create(file_path)?;

    let num_bytes = std::io::copy(&mut content.as_bytes(), &mut file)?;

    Ok(BytesDownloaded { num_bytes })
}

fn get_ws_scheme(secure: bool) -> &'static str {
    if secure {
        return "wss";
    }

    "ws"
}

/// Subscribe to scheduler events on the server.
pub async fn subscribe(
    url: &str,
    secure: bool,
) -> Result<impl StreamExt<Item = Result<SchedulerEvent, ClientWebsocketError>>, ClientError> {
    let mut abs_url = get_abs_url(url, "/api/v1/scheduler/ws")?;

    if abs_url.set_scheme(get_ws_scheme(secure)).is_err() {
        return Err(ClientError::UrlSchemeConversion);
    };

    let (ws_stream, _) = tokio_tungstenite::connect_async(abs_url).await?;

    fn text_only(msg: &Result<Message, tungstenite::Error>) -> bool {
        match msg {
            Err(_) => true,
            Ok(msg) => msg.is_text(),
        }
    }

    fn deserialize_msg(
        msg: Result<Message, tungstenite::Error>,
    ) -> Result<SchedulerEvent, ClientWebsocketError> {
        match msg {
            Err(error) => Err(ClientWebsocketError::Websocket(error)),
            Ok(msg) => match serde_json::from_str::<SchedulerEventResult>(&msg.to_string()) {
                Err(error) => Err(ClientWebsocketError::MalformedEvent(error)),
                Ok(event_result) => match event_result {
                    SchedulerEventResult::Lag(lag) => Err(ClientWebsocketError::Lag(lag)),
                    SchedulerEventResult::Event(event) => Ok(event),
                },
            },
        }
    }

    let output_stream = ws_stream.filter(text_only).map(deserialize_msg);

    Ok(output_stream)
}

/// Submit a workflow to the server.
pub async fn submit(url: &str, flow: &Flow) -> Result<Okay, ClientError> {
    let abs_url = get_abs_url(url, "/api/v1/job")?;

    let client = reqwest::Client::new();

    check_status_take(client.post(abs_url).json(flow).send().await?).await
}

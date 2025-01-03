use actix_web::{
    delete, get,
    http::StatusCode,
    post, put,
    web::{self},
    App, HttpRequest, HttpResponse, HttpServer, ResponseError,
};
use s3::Bucket;
use sqlx::{Pool, Postgres};
use tokio::sync::broadcast;

use actix::{Actor, AsyncContext, SpawnHandle, StreamHandler};
use actix_web_actors::ws;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::{
    server::{
        executor::{instantiate_flow, ExecutorError},
        model::Flow,
        record::{FlowListRecord, FlowRecord},
        scheduler::Scheduler,
        secrets::SecretsCrud,
    },
    task::{bucket::get_artefact, driver::get_store_path, errors::ArtefactError},
};

use super::{
    event::{to_event_result, SchedulerEvent},
    scheduler::SchedulerError,
    secrets::SecretsCrudError,
};

impl ResponseError for ExecutorError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ExecutorError::UnableToConstructPlan(_) | ExecutorError::FlowNameTooLong(_) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[post("/job")]
async fn create_job(
    flow: web::Json<Flow>,
    sched: web::Data<Scheduler>,
) -> Result<String, ExecutorError> {
    instantiate_flow(flow.into_inner(), &sched)
        .await
        .map(|id| id.to_string())
}

impl ResponseError for SchedulerError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SchedulerError::FlowDoesNotExist(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[get("/job")]
async fn list_jobs(
    sched: web::Data<Scheduler>,
) -> Result<web::Json<Vec<FlowListRecord>>, SchedulerError> {
    sched.list_flows().await.map(web::Json)
}

#[get("/job/{id}")]
async fn get_single_job(
    path: web::Path<i32>,
    sched: web::Data<Scheduler>,
) -> Result<web::Json<FlowRecord>, SchedulerError> {
    let id = path.into_inner();
    sched.get_flow(id).await.map(web::Json)
}

impl ResponseError for ArtefactError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ArtefactError::ArtefactDoesNotExist(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[get("/artefact/{flow_id}/{output_name}")]
async fn download_artefact(
    path: web::Path<(usize, String)>,
    bucket: web::Data<Box<Bucket>>,
) -> Result<HttpResponse, ArtefactError> {
    let (flow_id, output_name) = path.into_inner();
    let store_path = get_store_path(flow_id, &output_name);

    let bytes: Vec<u8> = get_artefact(&bucket, store_path).await?.into();

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/octet-stream")
        .body(bytes))
}

impl ResponseError for SecretsCrudError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SecretsCrudError::DatabaseQuery(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[post("/secret/{key}")]
async fn create_secret(
    key: web::Path<String>,
    value: web::Json<String>,
    secrets: web::Data<SecretsCrud>,
) -> Result<&'static str, SecretsCrudError> {
    secrets.create_secret(&key, &value).await?;

    Ok("")
}

#[delete("/secret/{key}")]
async fn delete_secret(
    key: web::Path<String>,
    secrets: web::Data<SecretsCrud>,
) -> Result<&'static str, SecretsCrudError> {
    secrets.delete_secret(&key).await?;

    Ok("")
}

#[put("/secret/{key}")]
async fn update_secret(
    key: web::Path<String>,
    value: web::Json<String>,
    secrets: web::Data<SecretsCrud>,
) -> Result<&'static str, SecretsCrudError> {
    secrets.update_secret(&key, &value).await?;

    Ok("")
}

struct SchedulerWebsocket {
    rx: Option<broadcast::Receiver<SchedulerEvent>>,
    spawn_handle: Option<SpawnHandle>,
}

impl Actor for SchedulerWebsocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // SAFETY: This function will only be called once, so unwrap() is okay
        let rx = self.rx.take().unwrap();

        let to_json_string = |event| serde_json::to_string(&event).unwrap();

        let stream = BroadcastStream::new(rx)
            .map(to_event_result)
            .map(to_json_string)
            .map(bytestring::ByteString::from)
            .map(ws::Message::Text)
            .map(Ok);

        self.spawn_handle = Some(ctx.add_stream(stream));
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SchedulerWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            _ => (),
        }
    }
}

#[get("/scheduler/ws")]
async fn listen_to_scheduler(
    req: HttpRequest,
    stream: web::Payload,
    sched: web::Data<Scheduler>,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(
        SchedulerWebsocket {
            rx: Some(sched.subscribe()),
            spawn_handle: None,
        },
        &req,
        stream,
    )
}

pub async fn start_server(
    port: u16,
    pool: Pool<Postgres>,
    sched: &Scheduler,
    bucket: Box<Bucket>,
) -> std::io::Result<()> {
    let sched = sched.clone();
    let secrets = SecretsCrud::new(pool.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(sched.clone()))
            .app_data(web::Data::new(bucket.clone()))
            .app_data(web::Data::new(secrets.clone()))
            .service(
                web::scope("/api/v1")
                    .service(create_job)
                    .service(list_jobs)
                    .service(get_single_job)
                    .service(download_artefact)
                    .service(create_secret)
                    .service(update_secret)
                    .service(delete_secret)
                    .service(listen_to_scheduler),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

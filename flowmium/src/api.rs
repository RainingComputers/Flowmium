use actix_web::{get, http::StatusCode, post, web, App, HttpResponse, HttpServer, ResponseError};
use s3::Bucket;
use serde::{Deserialize, Serialize};

use crate::{
    args::ServerOpts,
    artefacts::{bucket::get_artefact, errors::ArtefactError, task::get_store_path},
    flow::{
        executor::{instantiate_flow, ExecutorConfig, ExecutorError},
        model::Flow,
        scheduler::{FlowRecord, Scheduler, SchedulerError},
    },
};

impl ResponseError for ExecutorError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ExecutorError::UnableToConstructPlanError(_) => StatusCode::BAD_REQUEST,
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
            SchedulerError::FlowDoesNotExistError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[get("/job/running")]
async fn get_running_jobs(
    sched: web::Data<Scheduler>,
) -> Result<web::Json<Vec<FlowRecord>>, SchedulerError> {
    return sched.get_running_or_pending_flows().await.map(web::Json);
}

#[derive(Serialize, Deserialize)]
struct OffsetLimitParams {
    offset: i64,
    limit: i64,
}

#[get("/job/terminated")]
async fn get_terminated_jobs(
    offset_limit: web::Query<OffsetLimitParams>,
    sched: web::Data<Scheduler>,
) -> Result<web::Json<Vec<FlowRecord>>, SchedulerError> {
    return sched
        .get_terminated_flows(offset_limit.offset, offset_limit.limit)
        .await
        .map(web::Json);
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
            ArtefactError::ArtefactDoesNotExistError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[get("/artefact/{flow_id}/{output_name}")]
async fn download_artefact(
    path: web::Path<(usize, String)>,
    bucket: web::Data<Bucket>,
) -> Result<HttpResponse, ArtefactError> {
    let (flow_id, output_name) = path.into_inner();
    let store_path = get_store_path(flow_id, &output_name);

    let bytes: Vec<u8> = get_artefact(&bucket, store_path).await?.into();

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/octet-stream")
        .body(bytes))
}

pub async fn start_server(
    server_opts: ServerOpts,
    sched: Scheduler,
    executor_config: ExecutorConfig,
    bucket: Bucket,
) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(sched.clone()))
            .app_data(web::Data::new(executor_config.clone()))
            .app_data(web::Data::new(bucket.clone()))
            .service(
                web::scope("/api/v1")
                    .service(create_job)
                    .service(get_running_jobs)
                    .service(get_terminated_jobs)
                    .service(get_single_job)
                    .service(download_artefact),
            )
    })
    .bind(("0.0.0.0", server_opts.port))?
    .run()
    .await
}

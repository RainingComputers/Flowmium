use actix_web::{get, http::StatusCode, post, web, App, HttpServer};
use serde::{Deserialize, Serialize};

use crate::{
    args::ServerOpts,
    flow::{
        executor::{instantiate_flow, ExecutorConfig, ExecutorError},
        model::Flow,
        scheduler::{FlowRecord, Scheduler, SchedulerError},
    },
};

impl actix_web::error::ResponseError for ExecutorError {
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

impl actix_web::error::ResponseError for SchedulerError {
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
async fn get_job_single(
    path: web::Path<i32>,
    sched: web::Data<Scheduler>,
) -> Result<web::Json<FlowRecord>, SchedulerError> {
    let id = path.into_inner();
    sched.get_flow(id).await.map(web::Json)
}

pub async fn start_server(
    server_opts: ServerOpts,
    sched: Scheduler,
    executor_config: ExecutorConfig,
) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(sched.clone()))
            .app_data(web::Data::new(executor_config.clone()))
            .service(web::scope("/api/v1").service(create_job))
    })
    .bind(("0.0.0.0", server_opts.port))?
    .run()
    .await
}

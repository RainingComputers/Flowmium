use actix_web::{
    http::{header::ContentType, StatusCode},
    post, web, App, HttpResponse, HttpServer,
};

use crate::{
    args::ServerOpts,
    flow::{
        executor::{instantiate_flow, ExecutorConfig, ExecutorError},
        model::Flow,
        scheduler::Scheduler,
    },
};

impl actix_web::error::ResponseError for ExecutorError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

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

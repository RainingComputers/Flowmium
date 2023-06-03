use actix_web::{post, web, App, HttpServer, Responder};

use crate::{
    args::ServerOpts,
    flow::{executor::ExecutorConfig, scheduler::Scheduler},
};

#[post("/job")]
async fn create_job() -> impl Responder {
    "Hello world!"
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

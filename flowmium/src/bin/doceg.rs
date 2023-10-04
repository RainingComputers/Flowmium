use flowmium::driver;
use flowmium::executor;
use flowmium::scheduler;
use flowmium::secrets;

use flowmium::model::*;

#[tokio::main]
async fn main() {
    let pool = driver::get_default_postgres_pool().await.unwrap();

    let executor_config = driver::get_default_executor_config().await.unwrap();

    let scheduler = scheduler::Scheduler::new(pool.clone());

    let secrets = secrets::SecretsCrud::new(pool.clone());
    secrets
        .create_secret("super-secret-message", "hello world")
        .await
        .unwrap();

    let handle = driver::spawn_executor(&pool, &scheduler, &executor_config);

    let flow = create_example_flow();
    executor::instantiate_flow(flow, &scheduler).await.unwrap();

    handle.await.unwrap();
}

fn create_example_flow() -> Flow {
    Flow {
        name: "hello-world".to_string(),
        tasks: vec![Task {
            name: "hello-world".to_string(),
            image: "debian:latest".to_string(),
            depends: vec![],
            cmd: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo $MESSAGE".to_string(),
            ],
            env: vec![EnvVar::SecretRef(SecretRef {
                name: "MESSAGE".to_string(),
                from_secret: "super-secret-message".to_string(),
            })],
            inputs: None,
            outputs: None,
        }],
    }
}

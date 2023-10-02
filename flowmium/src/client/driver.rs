use std::future::Future;
use std::process::ExitCode;

use tokio_stream::StreamExt;

use crate::client::args;
use crate::client::requests;

use crate::client::requests::ClientError;
use crate::server::model::Flow;

async fn make_request<T, F>(req_func: impl Fn() -> F) -> ExitCode
where
    F: Future<Output = Result<T, ClientError>>,
    T: std::fmt::Display,
{
    match req_func().await {
        Ok(resp) => {
            println!("{}", resp);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprint!("{}", error);
            ExitCode::FAILURE
        }
    }
}

async fn get_flow_from_file(file_path: String) -> Result<Flow, ExitCode> {
    let contents = tokio::fs::read_to_string(file_path).await;

    let contents = match contents {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("unable to open file: {}", err);
            return Err(ExitCode::FAILURE);
        }
    };

    let flow = serde_yaml::from_str(&contents);

    let flow = match flow {
        Ok(flow) => flow,
        Err(err) => {
            eprint!("invalid definition: {}", err);
            return Err(ExitCode::FAILURE);
        }
    };

    Ok(flow)
}

/// Parse CLI arguments and run `flowctl`.
pub async fn run() -> ExitCode {
    let args: args::FlowCtlOptions = argh::from_env();

    match args.command {
        args::Command::List(_) => make_request(|| requests::list_workflows(&args.url)).await,
        args::Command::Describe(describe_opts) => {
            make_request(|| requests::get_status(&args.url, &describe_opts.id)).await
        }
        args::Command::Secret(secret_opts) => match secret_opts.command {
            args::SecretCommand::Create(create_opts) => {
                make_request(|| {
                    requests::create_secret(&args.url, &create_opts.key, &create_opts.value)
                })
                .await
            }
            args::SecretCommand::Update(update_opts) => {
                make_request(|| {
                    requests::update_secret(&args.url, &update_opts.key, &update_opts.value)
                })
                .await
            }
            args::SecretCommand::Delete(delete_opts) => {
                make_request(|| requests::delete_secret(&args.url, &delete_opts.key)).await
            }
        },
        args::Command::Download(download_opts) => {
            make_request(|| {
                requests::download_artefact_to_path(
                    &args.url,
                    &download_opts.id,
                    &download_opts.name,
                    &download_opts.local_dir_path,
                )
            })
            .await
        }
        args::Command::Submit(submit_opts) => {
            let flow = match get_flow_from_file(submit_opts.file_path).await {
                Err(exit_code) => return exit_code,
                Ok(flow) => flow,
            };

            make_request(|| requests::submit(&args.url, &flow)).await
        }
        args::Command::Subscribe(subscribe_opts) => {
            let stream = requests::subscribe(&args.url, subscribe_opts.secure).await;

            match stream {
                Err(error) => {
                    eprintln!("{}", error);
                    ExitCode::FAILURE
                }
                Ok(mut stream) => loop {
                    match stream.next().await {
                        Some(Err(error)) => {
                            eprintln!("{}", error);
                            break ExitCode::FAILURE;
                        }
                        Some(Ok(event)) => {
                            println!("{}", serde_json::to_string(&event).unwrap())
                        }
                        None => break ExitCode::SUCCESS,
                    }
                },
            }
        }
    }
}

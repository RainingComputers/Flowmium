use std::future::Future;
use std::process::ExitCode;

use crate::client::args;
use crate::client::requests;

use crate::client::requests::ClientError;

pub async fn make_request<T, F>(req_func: impl Fn() -> F) -> Result<String, ClientError>
where
    F: Future<Output = Result<T, ClientError>>,
    T: std::fmt::Debug,
{
    match req_func().await {
        Ok(resp) => Ok(format!("{:?}", resp)),
        Err(error) => Err(error),
    }
}

pub async fn run() -> ExitCode {
    let args: args::FlowCtlOptions = argh::from_env();

    let request_resp = match args.command {
        args::Command::Ls(_) => make_request(|| requests::list_workflows(&args.url)).await,
        args::Command::Status(status_opts) => {
            make_request(|| requests::get_status(&args.url, &status_opts.id)).await
        }
    };

    match request_resp {
        Ok(string) => {
            println!("{}", string);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprint!("{}", error);
            ExitCode::FAILURE
        }
    }
}

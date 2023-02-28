use kube::config;
use kube::runtime::wait;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{bucket, Bucket};
use serde::Deserialize;
use serde_json;

use std::env::{self, VarError};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tokio::fs;

use crate::flow::model::{Input, Output};

use super::bucket::{download_input, get_bucket, upload_output};
use super::errors::ArtefactError;

async fn wait_for_file(path: &PathBuf, timeout: Duration) -> bool {
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout {
        if fs::metadata(path).await.is_ok() {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    false
}

fn get_store_path(flow_id: usize, output_name: &str) -> String {
    return flow_id.to_string() + "/" + output_name;
}

async fn download_all_inputs(
    bucket: Bucket,
    flow_id: usize,
    inputs: Vec<Input>,
) -> Result<(), ArtefactError> {
    for input in inputs {
        let store_path = get_store_path(flow_id, &input.from);
        download_input(&bucket, input.path, store_path).await?;
    }

    return Ok(());
}

async fn upload_all_outputs(
    bucket: Bucket,
    flow_id: usize,
    outputs: Vec<Output>,
) -> Result<(), ArtefactError> {
    for output in outputs {
        let store_path = get_store_path(flow_id, &output.name);
        upload_output(&bucket, output.path, store_path).await?;
    }

    return Ok(());
}

#[derive(Deserialize, Debug)]
struct InitSidecarConfig {
    input_json: String,
    flow_id: usize,
    access_key: String,
    secret_key: String,
    bucket_name: String,
    store_url: String,
}

#[tracing::instrument]
async fn sidecar_init_main(config: InitSidecarConfig) -> ExitCode {
    let inputs: Vec<Input> = match serde_json::from_str(&config.input_json) {
        Ok(inputs) => inputs,
        Err(error) => {
            tracing::error!(%error, "Unable to parse inputs json in env variable");
            return ExitCode::FAILURE;
        }
    };

    let Ok(bucket) = get_bucket(&config.access_key, &config.secret_key, &config.bucket_name, config.store_url) else {
        return  ExitCode::FAILURE;
    };

    if let Err(_) = download_all_inputs(bucket, config.flow_id, inputs).await {
        return ExitCode::FAILURE;
    }

    return ExitCode::SUCCESS;
}

#[derive(Deserialize, Debug)]
struct WaitSidecarConfig {
    output_json: String,
    finish_file: String,
    finish_wait_timeout: u64,
    flow_id: usize,
    access_key: String,
    secret_key: String,
    bucket_name: String,
    store_url: String,
}

#[tracing::instrument]
async fn sidecar_wait_main(config: WaitSidecarConfig) -> ExitCode {
    let outputs: Vec<Output> = match serde_json::from_str(&config.output_json) {
        Ok(inputs) => inputs,
        Err(error) => {
            tracing::error!(%error, "Unable to parse outputs json in env variable");
            return ExitCode::FAILURE;
        }
    };

    if !wait_for_file(
        &PathBuf::from(config.finish_file),
        Duration::from_secs(config.finish_wait_timeout),
    )
    .await
    {
        tracing::error!("Waiting for output timedout");
        return ExitCode::FAILURE;
    };

    let Ok(bucket) = get_bucket(&config.access_key, &config.secret_key, &config.bucket_name, config.store_url) else {
        return  ExitCode::FAILURE;
    };

    if let Err(_) = upload_all_outputs(bucket, config.flow_id, outputs).await {
        return ExitCode::FAILURE;
    }

    return ExitCode::SUCCESS;
}

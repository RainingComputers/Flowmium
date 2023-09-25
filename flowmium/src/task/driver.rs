use s3::Bucket;
use serde::Deserialize;
use serde_json;

use std::process::{Command, ExitCode, Stdio};

use crate::server::model::{Input, Output};

use super::bucket::{download_input, get_bucket, upload_output};
use super::errors::ArtefactError;

pub fn get_store_path(flow_id: usize, output_name: &str) -> String {
    flow_id.to_string() + "/" + output_name
}

async fn download_all_inputs(
    bucket: &Bucket,
    flow_id: usize,
    inputs: Vec<Input>,
) -> Result<(), ArtefactError> {
    for input in inputs {
        let store_path = get_store_path(flow_id, &input.from);
        download_input(bucket, input.path, store_path).await?;
    }

    Ok(())
}

async fn upload_all_outputs(
    bucket: &Bucket,
    flow_id: usize,
    outputs: Vec<Output>,
) -> Result<(), ArtefactError> {
    for output in outputs {
        let store_path = get_store_path(flow_id, &output.name);
        upload_output(bucket, output.path, store_path).await?;
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct SidecarConfig {
    input_json: String,
    output_json: String,
    flow_id: usize,
    access_key: String,
    secret_key: String,
    bucket_name: String,
    task_store_url: String,
}

fn get_command(cmd: Vec<String>) -> Option<Command> {
    if cmd.is_empty() {
        tracing::error!("Invalid command");
        return None;
    }

    let mut command = Command::new(&cmd[0]);

    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }

    command.stdout(Stdio::inherit());

    Some(command)
}

#[tracing::instrument(skip(config, cmd))]
pub async fn run_task(config: SidecarConfig, cmd: Vec<String>) -> ExitCode {
    let option_inputs: Option<Vec<Input>> = match serde_json::from_str(&config.input_json) {
        Ok(inputs) => inputs,
        Err(error) => {
            tracing::error!(%error, "Unable to parse inputs json in env variable");
            return ExitCode::FAILURE;
        }
    };

    let option_outputs: Option<Vec<Output>> = match serde_json::from_str(&config.output_json) {
        Ok(inputs) => inputs,
        Err(error) => {
            tracing::error!(%error, "Unable to parse output json in env variable");
            return ExitCode::FAILURE;
        }
    };

    let Ok(bucket) = get_bucket(
        &config.access_key,
        &config.secret_key,
        &config.bucket_name,
        config.task_store_url,
    )
    .await
    else {
        return ExitCode::FAILURE;
    };

    if let Some(inputs) = option_inputs {
        if (download_all_inputs(&bucket, config.flow_id, inputs).await).is_err() {
            return ExitCode::FAILURE;
        }
    }

    let Some(mut command) = get_command(cmd) else {
        tracing::error!("Invalid command");
        return ExitCode::FAILURE;
    };

    // TODO: Add timeout
    let task_output = match command.output() {
        Ok(task_output) => task_output,
        Err(error) => {
            tracing::error!(%error, "Failed to run task");
            return ExitCode::FAILURE;
        }
    };

    if !task_output.status.success() {
        tracing::error!("Task existed with status {}", task_output.status);

        if let Ok(stdout_utf8) = String::from_utf8(task_output.stdout) {
            if !stdout_utf8.is_empty() {
                tracing::error!("Task exited with stdout {}", stdout_utf8);
            }
        }

        if let Ok(stderr_utf8) = String::from_utf8(task_output.stderr) {
            if !stderr_utf8.is_empty() {
                tracing::error!("Task exited with stderr {}", stderr_utf8);
            }
        }

        return ExitCode::FAILURE;
    }

    if let Some(outputs) = option_outputs {
        if (upload_all_outputs(&bucket, config.flow_id, outputs).await).is_err() {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

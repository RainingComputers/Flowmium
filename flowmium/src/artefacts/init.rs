use std::process::ExitCode;

use tokio::fs;

#[tracing::instrument]
pub async fn do_init(src: String, dest: String) -> ExitCode {
    if let Err(error) = fs::copy(src, dest).await {
        tracing::error!(%error, "Unable to copy flowmium executable");
        return ExitCode::FAILURE;
    }

    return ExitCode::SUCCESS;
}

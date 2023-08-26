use std::process::ExitCode;

#[tracing::instrument]
pub async fn do_init(src: String, dest: String) -> ExitCode {
    if let Err(error) = tokio::fs::copy(src, dest).await {
        tracing::error!(%error, "Unable to copy flowmium executable");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    flowmium::driver_client::run().await
}

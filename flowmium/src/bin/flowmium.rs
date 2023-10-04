use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    flowmium::driver::run().await
}

use std::process::ExitCode;

use flowmium::server::driver;

#[tokio::main]
async fn main() -> ExitCode {
    driver::run().await
}

use std::process::ExitCode;

use flowmium::client::driver;

#[tokio::main]
async fn main() -> ExitCode {
    driver::run().await
}

use std::process::ExitCode;

use flowmium::driver;

#[tokio::main]
async fn main() -> ExitCode {
    driver::run().await
}

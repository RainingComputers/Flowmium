use std::process::ExitCode;

use flowmium::flow::driver;

#[tokio::main]
async fn main() -> ExitCode {
    driver::run().await
}

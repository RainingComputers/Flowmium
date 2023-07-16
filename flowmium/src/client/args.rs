use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// flowctl, CLI tool for interacting with the Flowmium server
pub struct FlowCtlOptions {
    #[argh(option, default = "String::from(\"http://localhost:8080\")")]
    /// flowmium server url
    pub url: String,

    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    Ls(LsOpts),
    Status(StatusOpts),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "ls")]
/// List all workflows
pub struct LsOpts {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "status")]
/// Get status of a workflow
pub struct StatusOpts {
    #[argh(positional)]
    /// id of the workflow
    pub id: String,
}

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// flowmium, workflow orchestrator written in rust
pub struct FlowmiumOptions {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    Init(InitOpts),
    Task(TaskOpts),
    Server(ServerOpts),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "task")]
/// run flowmium task pod
pub struct TaskOpts {
    #[argh(greedy, positional)]
    /// command for the task to run
    pub cmd: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "init")]
/// run init container to copy the flowmium executable into the task container
pub struct InitOpts {
    #[argh(positional)]
    /// source of the flowmium executable in the init container
    pub src: String,
    #[argh(positional)]
    /// shared volume destination to where the flowmium executable should be copied
    pub dest: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "server")]
/// start executor server
pub struct ServerOpts {
    #[argh(option)]
    /// port for the API server
    pub port: u16,
}

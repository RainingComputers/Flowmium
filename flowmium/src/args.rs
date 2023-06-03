use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Flowmium, workflow orchestrator written in rust
pub struct FlowmiumOptions {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    Init(InitOpts),
    Task(TaskOpts),
    Execute(ExecuteOpts),
    Server(ServerOpts),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "task")]
/// Run flowmium task pod
pub struct TaskOpts {
    #[argh(positional)]
    /// command for the task to run
    pub cmd: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "init")]
/// Run init container to copy the flowmium executable into the task container
pub struct InitOpts {
    #[argh(positional)]
    /// source of the flowmium executable in the init container
    pub src: String,
    #[argh(positional)]
    /// shared volume destination to where the flowmium executable should be copied
    pub dest: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "execute")]
/// Run YAML dag job with a temporary local executor without the API server
pub struct ExecuteOpts {
    #[argh(positional)]
    /// list of paths to YAML files containing flow definitions
    pub files: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "server")]
/// Start executor server
pub struct ServerOpts {
    #[argh(option)]
    /// port for the API server
    pub port: u16,
}

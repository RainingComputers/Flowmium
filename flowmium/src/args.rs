use gumdrop::Options;

#[derive(Debug, Options)]
pub struct FlowmiumOptions {
    #[options(help = "Print help message")]
    pub help: bool,

    #[options(command, required)]
    pub command: Option<Command>,
}

#[derive(Debug, Options)]
pub enum Command {
    #[options(help = "Run flowmium main")]
    Init(InitOpts),
    #[options(help = "Run flowmium task")]
    Task(TaskOpts),
    #[options(help = "Execute DAG flows")]
    Execute(ExecuteOpts),
}

#[derive(Debug, Options)]
pub struct TaskOpts {
    #[options(free, help = "Command for task")]
    pub cmd: Vec<String>,
}

#[derive(Debug, Options)]
pub struct InitOpts {
    #[options(free, help = "Path to flowmium executable in the container")]
    pub src: String,
    #[options(
        free,
        help = "Destination path of the volume to copy the executable to"
    )]
    pub dest: String,
}

#[derive(Debug, Options)]
pub struct ExecuteOpts {
    #[options(free, help = "List of DAG flow definition")]
    pub files: Vec<String>,
}

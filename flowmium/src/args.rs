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
    #[options(help = "Run flowmium sidecar container")]
    Sidecar(SidecarOpts),
    #[options(help = "Run flowmium main container")]
    Main(MainOpts),
    #[options(help = "Execute DAG flows")]
    Execute(ExecuteOpts),
}

#[derive(Debug, Options)]
pub struct SidecarOpts {
    #[options(help = "Launch init sidecar which will download all required inputs")]
    pub init: bool,
    #[options(
        help = "Launch main sidecar which will wait for main container and upload all outputs"
    )]
    pub wait: bool,
}

#[derive(Debug, Options)]
pub struct MainOpts {
    #[options(help = "Command for main container")]
    pub cmd: Vec<String>,
}

#[derive(Debug, Options)]
pub struct ExecuteOpts {
    #[options(help = "List of DAG flow definition")]
    pub files: Vec<String>,
}

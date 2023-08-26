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
    List(LsOpts),
    Describe(DescribeOpts),
    Download(DownloadOpts),
    Secret(SecretOpts),
    Subscribe(SubscribeOpts),
    Submit(SubmitOpts),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// list all workflows
pub struct LsOpts {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "describe")]
/// describe workflow properties and status in json
pub struct DescribeOpts {
    #[argh(positional)]
    /// id of the workflow
    pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "download")]
/// download output from a workflow
pub struct DownloadOpts {
    #[argh(positional)]
    /// if of the workflow
    pub id: String,

    #[argh(positional)]
    /// name of the output from the workflow
    pub name: String,

    #[argh(positional)]
    /// local directory path to download the output to
    pub local_dir_path: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "secret")]
/// manage secrets stored in the server
pub struct SecretOpts {
    #[argh(subcommand)]
    pub command: SecretCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum SecretCommand {
    Create(SecretCreateOpts),
    Delete(SecretDeleteOpts),
    Update(SecretUpdateOpts),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "create")]
/// create a secret
pub struct SecretCreateOpts {
    #[argh(positional)]
    /// key for the secret
    pub key: String,
    #[argh(positional)]
    /// value for the secret
    pub value: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "delete")]
/// delete a secret
pub struct SecretDeleteOpts {
    #[argh(positional)]
    /// key for the secret
    pub key: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "update")]
/// update a secret
pub struct SecretUpdateOpts {
    #[argh(positional)]
    /// key for the secret
    pub key: String,
    #[argh(positional)]
    /// value for the secret
    pub value: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "subscribe")]
/// subscribe to server's scheduler events
pub struct SubscribeOpts {
    #[argh(switch)]
    /// use wss:// scheme instead of ws:// scheme
    pub secure: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "submit")]
/// submit workflow yaml definition file
pub struct SubmitOpts {
    #[argh(positional)]
    /// path to the yaml definition file
    pub file_path: String,
}

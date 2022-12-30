use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct KeyValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputRef {
    pub name: String,
    pub from_input: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    pub name: String,
    pub from_secret: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum EnvVar {
    KeyValuePair(KeyValuePair),
    InputRef(InputRef),
    SecretRef(SecretRef),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Input {
    pub from: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Output {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Step {
    pub name: String,
    pub image: String,
    pub depends: Vec<String>,
    pub cmd: Vec<String>,
    pub env: Vec<EnvVar>,
    pub inputs: Option<Vec<Input>>,
    pub outputs: Option<Vec<Output>>,
}

// TODO: Add kubernetes config
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ContainerDAGJob {
    pub name: String,
    pub schedule: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PythonJob {
    pub image: String,
    pub registry: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Job {
    ContainerDAGJob(ContainerDAGJob),
    PythonJob(PythonJob),
}

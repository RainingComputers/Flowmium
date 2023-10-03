use serde::{Deserialize, Serialize};

/// String literal environment variable.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct KeyValuePair {
    /// Name for the environment variable.
    pub name: String,
    /// String value for the environment variable.
    pub value: String,
}

/// Environment variable whose value comes from a secret stored in the server.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    /// Name for the environment variable.
    pub name: String,
    /// Name of the secret key to extract the value from. The secret can be create via
    /// `flowctl secret create <key> <value>` or [`crate::client::requests::create_secret`] or [`crate::server::secrets::SecretsCrud`].
    pub from_secret: String,
}

/// Define an environment variable for the task.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum EnvVar {
    /// Create an environment variable with a string literal value.
    KeyValuePair(KeyValuePair),
    /// Create an environment variable with a value from a secret stored in the server.
    SecretRef(SecretRef),
}

/// An input file consumed by the task.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Input {
    /// Name of the output from a dependent task.
    pub from: String,
    /// Path to which the output should be copied to within the task container.
    pub path: String,
}

/// An output file emitted by this task.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Output {
    /// Name for the output.
    pub name: String,
    /// Path to the output file inside the task container.
    pub path: String,
}

// TODO: Add kubernetes config
// active_deadline_seconds: 34
// affinity: 34
// tolerations: 34
// image_pull_secrets: 34
// priority: 3
// limits: 23
// requests: 23

/// Defines a single task belonging to a flow.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Task {
    /// Name for the task.
    pub name: String,
    /// Container image for the task.
    pub image: String,
    /// List of names of the task that this task depends on.
    pub depends: Vec<String>,
    /// Command to be executed inside the container image to run that task.
    pub cmd: Vec<String>,
    /// List of environment variable for the task.
    pub env: Vec<EnvVar>,
    /// List of input files that this task will consume. Each input will refer to
    /// an output file from a dependent task.
    pub inputs: Option<Vec<Input>>,
    /// List of output files from this task.
    pub outputs: Option<Vec<Output>>,
}

/// Defines a workflow composed of multiple tasks that depend on each other in a DAG.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Flow {
    /// Name for the flow.
    pub name: String,
    /// Set of tasks in a DAG.
    pub tasks: Vec<Task>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model() {
        let serialized = r#"
        name: "hello-world"
        tasks:
          - name: "hello-world-zero"
            image: "foo/bar"
            depends: ["foo", "bar"]
            cmd: ["echo", "hello world"]
            env:
              - name: "ENV_VAR_ONE"
                value: "foobar"
              - name: "ENV_VAR_TWO"
                fromSecret: "some-secret"
              - name: "ENV_VAR_THREE"
                fromSecret: "this-is-some-secret"
            inputs:
              - from: "output-from-previous-step"
                path: "/some/random/path"
            outputs:
              - name: "some-random-output"
                path: "/some/random/output/path"
        "#;

        let job: Flow = serde_yaml::from_str(serialized).unwrap();

        let job_expected = Flow {
            name: "hello-world".to_owned(),
            tasks: vec![Task {
                name: "hello-world-zero".to_owned(),
                image: "foo/bar".to_owned(),
                depends: vec!["foo".to_owned(), "bar".to_owned()],
                cmd: vec!["echo".to_owned(), "hello world".to_owned()],
                env: vec![
                    EnvVar::KeyValuePair(KeyValuePair {
                        name: "ENV_VAR_ONE".to_owned(),
                        value: "foobar".to_owned(),
                    }),
                    EnvVar::SecretRef(SecretRef {
                        name: "ENV_VAR_TWO".to_owned(),
                        from_secret: "some-secret".to_owned(),
                    }),
                    EnvVar::SecretRef(SecretRef {
                        name: "ENV_VAR_THREE".to_owned(),
                        from_secret: "this-is-some-secret".to_owned(),
                    }),
                ],
                inputs: Some(vec![Input {
                    from: "output-from-previous-step".to_owned(),
                    path: "/some/random/path".to_owned(),
                }]),
                outputs: Some(vec![Output {
                    name: "some-random-output".to_owned(),
                    path: "/some/random/output/path".to_owned(),
                }]),
            }],
        };

        assert_eq!(job, job_expected);
    }
}

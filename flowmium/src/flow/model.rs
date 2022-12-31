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
pub struct Task {
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
pub struct ContainerDAGFlow {
    pub name: String,
    pub schedule: Option<String>,
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PythonFlow {
    pub image: String,
    pub registry: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Flow {
    ContainerDAGJob(ContainerDAGFlow),
    PythonJob(PythonFlow),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model() {
        let serialized = r#"
        name: "hello-world"
        schedule: ""
        tasks:
          - name: "hello-world-zero"
            image: "foo/bar"
            depends: ["foo", "bar"]
            cmd: ["echo", "hello world"]
            env:
              - name: "ENV_VAR_ONE"
                value: "foobar"
              - name: "ENV_VAR_TWO"
                fromInput: "some-other-input"
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

        let job_expected = Flow::ContainerDAGJob(ContainerDAGFlow {
            name: "hello-world".to_owned(),
            schedule: Some("".to_owned()),
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
                    EnvVar::InputRef(InputRef {
                        name: "ENV_VAR_TWO".to_owned(),
                        from_input: "some-other-input".to_owned(),
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
        });

        assert_eq!(job, job_expected);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct KeyValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    pub name: String,
    pub from_secret: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum EnvVar {
    KeyValuePair(KeyValuePair),
    SecretRef(SecretRef),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Input {
    pub from: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Output {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
// active_deadline_seconds: 34
// affinity: 34
// tolerations: 34
// image_pull_secrets: 34
// priority: 3
// limits: 23
// requests: 23
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Flow {
    pub name: String,
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

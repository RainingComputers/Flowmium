mod job;

use job::{EnvVar, Input, InputRef, Job, KeyValuePair, Output, SecretRef};
use std::{process::ExitCode, vec};

use crate::job::{ContainerDAGJob, Step};

fn main() -> ExitCode {
    let deserialized = r#"
    name: "hello-world"
    schedule: ""
    steps:
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

    let job: Job = match serde_yaml::from_str(deserialized) {
        Ok(value) => value,
        Err(err) => {
            print!("ERROR: {:?}", err);
            return ExitCode::FAILURE;
        }
    };

    let job_expected = Job::ContainerDAGJob(ContainerDAGJob {
        name: "hello-world".to_owned(),
        schedule: Some("".to_owned()),
        steps: vec![Step {
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

    println!("{:?}", job);
    assert_eq!(job, job_expected);

    return ExitCode::SUCCESS;
}

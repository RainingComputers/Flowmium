<img src="./logo.svg" width="128px"><br>

# Flowmium

Flowmium is a workflow orchestrator that uses Kubernetes. You can define and run a [YAML workflow](/examples/yaml_flow_definition/my_flow.yaml) of containers or you can run a python workflow where each function runs as a kubernetes pod.

A python workflow would look like this

```python
from flowmium import Flow, FlowContext
from flowmium.serializers import plain_text, json_text, pkl


flow = Flow("testing")


@flow.task(serializer=json_text)
def foo() -> str:
    return "Hallo world"


@flow.task({"input_str": foo}, serializer=plain_text)
def replace_letter_a(input_str: str, flowctx: FlowContext) -> str:
    return input_str.replace("a", "e") + str(flowctx.task_id)


@flow.task({"input_str": foo}, serializer=pkl)
def replace_letter_t(input_str: str) -> str:
    return input_str.replace("t", "d")


@flow.task(
    {"first": replace_letter_t, "second": replace_letter_a}, serializer=plain_text
)
def concat(first: str, second: str) -> str:
    return f"{first} {second}"


if __name__ == "__main__":
    flow.run()

```

## Getting started

-   [Setting up on local for testing](examples/deployment/)
-   [Deploying on production](examples/deployment/README.md#for-production)
-   [Example python package workflow](examples/python_package_workflow/)
-   [Example python script workflow](examples/python_script_workflow/)
-   [Example YAML definition workflow](examples/yaml_flow_definition/)
-   [Python framework documentation](http://flowmium.readthedocs.io/)
-   [API documentation](flowmium/apidoc.http)

## `flowctl` CLI

The `flowctl` CLI is used to monitor current status of workflows, submit new workflows and download artifacts.

### Install

```
cargo install flowmium
```

### Usage

| Action              | Command                                                     |
| ------------------- | ----------------------------------------------------------- |
| List workflows      | `flowctl list`                                              |
| Use explicit URL    | `flowctl --url http://localhost:8080 list`                  |
| Submit a YAML flow  | `flowctl submit flow.yaml`                                  |
| Download artefact   | `flowctl download <flow-id> <output-name> <local-dir-path>` |
| Subscribe to events | `flowctl subscribe`                                         |
| Describe a flow     | `flowctl describe <id>`                                     |
| Create secrets      | `flowctl secret create <key> <value>`                       |
| Update secret       | `flowctl secret update <key> <value>`                       |
| Delete secret       | `flowctl secret delete <key>`                               |

### Notes

Secrets are stored in the server and can be referred to set environment variable values in YAML definition or the Python workflows. This is so you don't have to commit secrets to your repository. They don't however use Kubernetes secrets, they are set as normal environment variables when workflow tasks are deployed as a Job.

## YAML flow definition schema

Reference for YAML flow definition. See [example](examples/yaml_flow_definition/my_flow.yaml).

### Root

| Key     | Type                  | Description                                                   |
| ------- | --------------------- | ------------------------------------------------------------- |
| `name`  | string                | Name of the flow                                              |
| `tasks` | list of [Task](#task) | List of tasks, each task will be deployed as a kubernetes job |

### Task

| Key       | Type                      | Description                                                                                 |
| --------- | ------------------------- | ------------------------------------------------------------------------------------------- |
| `name`    | string                    | Name of the task                                                                            |
| `image`   | string                    | Docker image for the task                                                                   |
| `depends` | list of string            | List of names of other tasks this task depends on, these tasks will be run before this task |
| `cmd`     | list of string            | Entry point command the task                                                                |
| `env`     | list of [Env](#env)       | List of environment variables for the task                                                  |
| `inputs`  | list of [Input](#input)   | List of inputs to download from dependency tasks                                            |
| `outputs` | list of [Output](#output) | List of outputs to upload from the task so it can be used by other tasks                    |

### Env

| Key                     | Type   | Description                                                           |
| ----------------------- | ------ | --------------------------------------------------------------------- |
| `name`                  | string | Name of the environment variable                                      |
| `value` or `fromSecret` | string | Literal string value if `value` or name of the secret if `fromSecret` |

### Input

| Key    | Type   | Description                                            |
| ------ | ------ | ------------------------------------------------------ |
| `from` | string | Name of output from a dependency task to be downloaded |
| `path` | string | The path to which to the input should be downloaded to |

### Output

| Key    | Type   | Description                                                         |
| ------ | ------ | ------------------------------------------------------------------- |
| `name` | string | Name of the output                                                  |
| `path` | string | The path to which to the output will be written to by running `cmd` |

## Running from source

### Running python flow example from source

These instructions will allow you to run an example python flow (`framework/tests/example_flow.py`) all from local source without pulling from upstream (including the executor).
Use this to validate your changes.
Instructions assume you are at the root of the repo.

-   Install sqlx CLI

    ```
    cargo install sqlx-cli
    ```

-   Run a test kubernetes cluster, minio and container registry in local

    ```
    cd flowmium/
    make up
    ```

-   Watch for pods running in the local cluster

    ```
    cd flowmium/
    make watch
    ```

-   Run migrations

    ```
    cd flowmium/
    sqlx migrate run
    ```

-   Run the flowmium server from root of this repo

    ```
    cd flowmium/
    export FLOWMIUM_POSTGRES_URL='postgres://flowmium:flowmium@localhost/flowmium'
    export FLOWMIUM_STORE_URL='http://localhost:9000'
    export FLOWMIUM_TASK_STORE_URL='http://172.16.238.4:9000'
    export FLOWMIUM_BUCKET_NAME='flowmium-test'
    export FLOWMIUM_ACCESS_KEY='minio'
    export FLOWMIUM_SECRET_KEY='password'
    export FLOWMIUM_INIT_CONTAINER_IMAGE='docker.io/shnoo28/flowmium:latest'
    export FLOWMIUM_NAMESPACE=default
    export KUBECONFIG=./kubeconfig.yaml
    cargo run --bin flowmium -- server --port 8080
    ```

-   Watch flow status using `flowctl`

    ```
    cd flowmium/
    cargo build
    watch ./target/debug/flowctl list
    ```

-   Build and push the example python flow (NOTE: You might want to use a different image name if you running the test for the second time or prune docker images on your machine)

    ```
    cd framework/
    docker build . -t py-flow-test
    docker tag py-flow-test localhost:5180/py-flow-test:latest
    docker push localhost:5180/py-flow-test:latest
    ```

-   Submit the flow to the executor server

    ```
    python3 -m tests --image registry:5000/py-flow-test:latest --cmd 'python3 -m tests' --flowmium-server http://localhost:8080
    ```

### Running e2e tests

-   For running e2e tests with init container from upstream

    ```
    make test
    ```

-   For running e2e tests with init container from source

    ```
    FLOWMIUM_INIT_CONTAINER_IMAGE_FROM_SOURCE=true make test
    ```

### Running unit tests for python framework

Run `make test` from `framework/` path.

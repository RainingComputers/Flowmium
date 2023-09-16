# Running tests

## Running integration test for python framework and executor from source (Linux)

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

-   Run tests (this is required)

    ```
    cd flowmium/
    make test
    ```

-   Run the flowmium server from root of this repo

    ```
    cd flowmium/
    export FLOWMIUM_POSTGRES_URL='postgres://flowmium:flowmium@localhost/flowmium'
    export FLOWMIUM_STORE_URL='http://172.16.238.4:9000'
    export FLOWMIUM_BUCKET_NAME='flowmium-test'
    export FLOWMIUM_ACCESS_KEY='minio'
    export FLOWMIUM_SECRET_KEY='password'
    export FLOWMIUM_EXECUTOR_IMAGE='registry:5000/flowmium-debug'
    export KUBECONFIG=./kubeconfig.yaml
    cargo run --bin flowmium -- server --port 8080
    ```

-   Watch flow status using `flowctl`

    ```
    cd flowmium/
    cargo build
    watch ./target/debug/flowctl list
    ```

-   Build and push the example flow (NOTE: You might want to use a different image name if you running the test for the second time or prune docker images on your machine)

    ```
    cd framework/
    docker build . -t py-flow-test
    docker tag py-flow-test localhost:5000/py-flow-test:latest
    docker push localhost:5000/py-flow-test:latest
    ```

-   Submit the flow to the executor server

    ```
    python3 -m tests --image registry:5000/py-flow-test:latest --cmd 'python3 -m tests' --flowmium-server http://localhost:8080
    ```

## Running unit tests for python framework

Run `make test` from `framework/` path.

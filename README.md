<img src="./logo.svg" width="128px"><br>

# Flowmium

Flowmium is a workflow orchestrator that uses kubernetes written in rust.

## Running the python flow example

Run the below commands to run an [example python flow](framework/test.py)

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

-   Build and push the example flow

    ```
    cd framework/
    docker build . -t py-flow-test
    docker tag py-flow-test localhost:5000/py-flow-test:latest
    docker push localhost:5000/py-flow-test:latest
    ```

-   Submit the flow to the executor server

    ```
    python3 test.py --image registry:5000/py-flow-test:latest --cmd python3 test.py --flowmium-server http://localhost:8080
    ```

## TODO

-   [ ] Workers
-   [ ] Handle unknown pod statuses
-   [ ] Publish to crates.io and PyPI

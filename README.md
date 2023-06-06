<img src="./logo.svg" width="128px"><br>

# Flowmium

Flowmium is a workflow orchestrator that uses kubernetes written in rust.

## Running the python flow example

Run the below commands to run an [example python flow](framework/test.py)

- Run a test kubernetes cluster, minio and container registry in local

  ```
  cd flowmium/
  make up
  ```

- Watch for pods running in the local cluster

  ```
  cd flowmium/
  make watch
  ```

- Run tests (this is required)

  ```
  cd flowmium/
  make test
  ```

- Run the flowmium server from root of this repo

  ```
  cd flowmium/
  export FLOWMIUM_POSTGRES_URL='postgres://flowmium:flowmium@localhost/flowmium'
  export FLOWMIUM_STORE_URL='http://172.16.238.4:9000'
  export FLOWMIUM_BUCKET_NAME='flowmium-test'
  export FLOWMIUM_ACCESS_KEY='minio'
  export FLOWMIUM_SECRET_KEY='password'
  export FLOWMIUM_EXECUTOR_IMAGE='registry:5000/flowmium-debug'
  export KUBECONFIG=./kubeconfig.yaml
  cargo run -- server --port 8080
  ```

- Build and push the example flow

  ```
  cd framework/
  docker build . -t py-flow-test
  docker tag py-flow-test localhost:5000/py-flow-test:latest
  docker push localhost:5000/py-flow-test:latest
  python3 test.py --image registry:5000/py-flow-test:latest --cmd python3 test.py --flowmium-server http://localhost:8080
  ```

- Submit the flow to the executor server

  ```
  python3 test.py --image registry:5000/py-flow-test:latest --cmd python3 test.py --flowmium-server http://localhost:8080
  ```

## TODO

### Features

- [ ] Workers in executor
- [ ] Workers in the framework
- [ ] Secrets in executor
- [ ] Secrets in the framework
- [ ] Remove credentials from logs
- [ ] Environment variables in framework

### Bugs

- [ ] Handle unknown pod statuses

### API

- [x] `POST /api/v1/job`
- [x] `GET /api/v1/job/` and `GET /api/v1/job/{name}`
- [ ] `DELETE /api/v1/job/{name}`
- [ ] `POST /api/v1/secret` and `PUT /api/v1/secret`
- [ ] `GET /api/v1/secret/`
- [ ] `DELETE /api/v1/secret`
- [ ] Artifacts API

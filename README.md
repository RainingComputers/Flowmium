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

- Run the flowmium server from root of this repo

  ```
  cd flowmium/
  export FLOWMIUM_POSTGRES_URL='postgres://flowmium:flowmium@localhost/flowmium'
  export FLOWMIUM_STORE_URL='http://172.16.238.4:9000'
  export FLOWMIUM_BUCKET_NAME='flowmium-test'
  export FLOWMIUM_ACCESS_KEY='minio'
  export FLOWMIUM_SECRET_KEY='password'
  export FLOWMIUM_EXECUTOR_IMAGE='registry:5000/flowmium-debug'
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

## Draft design notes

These designs are still in early stages, detailed documentation is yet to be done.

### Endpoints and job models

- `POST /api/v1/job`
- `GET /api/v1/job/` and `GET /api/v1/job/{name}`
- `DELETE /api/v1/job/{name}`
- `POST /api/v1/secret` and `PUT /api/v1/secret`
- `GET /api/v1/secret/`
- `DELETE /api/v1/secret`
- Artifacts

### Python framework design and usage example

```python
from flowmium import flow

@flow.task()
def foo1(context):
    return 1

@flow.task({'arg_1': foo1})
def foo2(context, arg_1)
    print(context.secrets)
    return arg_1 + 1

@flow.task({'arg_1': foo1}, workers=8)
def foo3(context, arg_1)
    print(context.worker_id)
    return arg_1 + 2

@flow.task(depends={'arg_1': foo2, 'arg_2': foo3}, secrets=[""], config={})
def foo3(context, arg_1, arg_2)
    return arg_1 * arg_2

flow.run(name="", schedule="", secrets=[""], config={})
```

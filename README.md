![](logo.svg)

# Flowmium

Flowmium is a workflow orchestrator that uses kubernetes written in rust.

## Draft design notes

These designs are still in early stages, detailed documentation is yet to be done.

### Endpoints and job models

-   `POST /api/v1/job` and `PUT /api/v1/job`

_DAG container job definition_

```yaml
name: ""
schedule: ""
tasks:
  - name: ""
    image: ""
    depends: ["", ""]
    cmd: [],
    env:
      - name: ""
        value: ""
      - name: ""
        fromInput: ""
      - name: ""
        fromSecret: ""
    inputs:
      - from: ""
        path: ""
    outputs:
      - name: ""
        path: ""
config:
  active_deadline_seconds: 34
  affinity: 34
  tolerations: 34
  image_pull_secrets: 34
  priority: 3
  limits: 23
  requests: 23
```

_Python framework job definition_

```yaml
python:
    image: ""
    registry: ""
```

-   `GET /api/v1/job/` and `GET /api/v1/job/{name}`
-   `DELETE /api/v1/job/{name}`
-   `POST /api/v1/registry` and `PUT /api/v1/registry`
-   `GET /api/v1/registry/`
-   `DELETE /api/v1/registry`
-   `POST /api/v1/secret` and `PUT /api/v1/secret`
-   `GET /api/v1/secret/`
-   `DELETE /api/v1/secret`
-   Artifacts

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
    print(context.worker_id)    # used for manual sharding
    return arg_1 + 2

@flow.task(depends={'arg_1': foo2, 'arg_2': foo3}, secrets=[""], config={})
def foo3(context, arg_1, arg_2)
    return arg_1 * arg_2

flow.run(name="", schedule="", secrets=[""], config={})
# prints yaml DAG container job definition to run each function (task) as a pod
# which function to run in will be specified by CLI arguments
# the flow object will take care of parsing the arguments
```

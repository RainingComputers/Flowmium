from flowmium import Flow, FlowContext


flow = Flow("testing")


@flow.task()
def foo() -> str:
    return "Hallo worlt"


@flow.task({"input_str": foo})
def replace_letter_a(input_str: str, flowctx: FlowContext) -> str:
    return input_str.replace("a", "e") + str(flowctx.task_id)


@flow.task({"input_str": foo})
def replace_letter_t(input_str: str) -> str:
    return input_str.replace("t", "d")


@flow.task({"first": replace_letter_t, "second": replace_letter_a})
def concat(first: str, second: str) -> str:
    return f"{first} {second}"


def test_dag_yaml() -> None:
    expected_dag = {
        "name": "testing",
        "tasks": [
            {
                "name": "foo",
                "image": "registry:5000/localhost",
                "depends": [],
                "cmd": ["python3", "test.py"],
                "env": [{"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "0"}],
                "inputs": [],
                "outputs": [{"name": "foo-output", "path": "task-output-foo.pkl"}],
            },
            {
                "name": "replace-letter-a",
                "image": "registry:5000/localhost",
                "depends": ["foo"],
                "cmd": ["python3", "test.py"],
                "env": [{"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "1"}],
                "inputs": [{"from": "foo-output", "path": "task-inputs-input_str.pkl"}],
                "outputs": [
                    {
                        "name": "replace-letter-a-output",
                        "path": "task-output-replace-letter-a.pkl",
                    }
                ],
            },
            {
                "name": "replace-letter-t",
                "image": "registry:5000/localhost",
                "depends": ["foo"],
                "cmd": ["python3", "test.py"],
                "env": [{"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "2"}],
                "inputs": [{"from": "foo-output", "path": "task-inputs-input_str.pkl"}],
                "outputs": [
                    {
                        "name": "replace-letter-t-output",
                        "path": "task-output-replace-letter-t.pkl",
                    }
                ],
            },
            {
                "name": "concat",
                "image": "registry:5000/localhost",
                "depends": ["replace-letter-t", "replace-letter-a"],
                "cmd": ["python3", "test.py"],
                "env": [{"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "3"}],
                "inputs": [
                    {
                        "from": "replace-letter-t-output",
                        "path": "task-inputs-first.pkl",
                    },
                    {
                        "from": "replace-letter-a-output",
                        "path": "task-inputs-second.pkl",
                    },
                ],
                "outputs": [
                    {"name": "concat-output", "path": "task-output-concat.pkl"}
                ],
            },
        ],
    }

    assert (
        flow.get_dag_dict("registry:5000/localhost", ["python3", "test.py"])
        == expected_dag
    )

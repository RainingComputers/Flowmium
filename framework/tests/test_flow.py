from flowmium import Flow, FlowContext
from flowmium.serializers import json_text, plain_text
import os


flow = Flow("testing")


@flow.task()
def foo() -> str:
    return os.environ["GREETINGS"]


@flow.task({"input_str": foo}, serializer=plain_text)
def replace_letter_a(input_str: str, flowctx: FlowContext) -> str:
    return input_str.replace("a", "e") + str(flowctx.task_id)


@flow.task({"input_str": foo}, serializer=json_text)
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
                "env": [
                    {"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "0"},
                    {"name": "GREETINGS", "fromSecret": "test-greetings-secret"},
                ],
                "inputs": [],
                "outputs": [{"name": "foo-output.pkl", "path": "task-output-foo.pkl"}],
            },
            {
                "name": "replace-letter-a",
                "image": "registry:5000/localhost",
                "depends": ["foo"],
                "cmd": ["python3", "test.py"],
                "env": [
                    {"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "1"},
                    {"name": "GREETINGS", "fromSecret": "test-greetings-secret"},
                ],
                "inputs": [
                    {"from": "foo-output.pkl", "path": "task-inputs-input_str.pkl"}
                ],
                "outputs": [
                    {
                        "name": "replace-letter-a-output.txt",
                        "path": "task-output-replace-letter-a.txt",
                    }
                ],
            },
            {
                "name": "replace-letter-t",
                "image": "registry:5000/localhost",
                "depends": ["foo"],
                "cmd": ["python3", "test.py"],
                "env": [
                    {"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "2"},
                    {"name": "GREETINGS", "fromSecret": "test-greetings-secret"},
                ],
                "inputs": [
                    {"from": "foo-output.pkl", "path": "task-inputs-input_str.pkl"}
                ],
                "outputs": [
                    {
                        "name": "replace-letter-t-output.json",
                        "path": "task-output-replace-letter-t.json",
                    }
                ],
            },
            {
                "name": "concat",
                "image": "registry:5000/localhost",
                "depends": ["replace-letter-t", "replace-letter-a"],
                "cmd": ["python3", "test.py"],
                "env": [
                    {"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": "3"},
                    {"name": "GREETINGS", "fromSecret": "test-greetings-secret"},
                ],
                "inputs": [
                    {
                        "from": "replace-letter-t-output.json",
                        "path": "task-inputs-first.json",
                    },
                    {
                        "from": "replace-letter-a-output.txt",
                        "path": "task-inputs-second.txt",
                    },
                ],
                "outputs": [
                    {"name": "concat-output.pkl", "path": "task-output-concat.pkl"}
                ],
            },
        ],
    }

    assert (
        flow.get_dag_dict(
            "registry:5000/localhost",
            ["python3", "test.py"],
            {"GREETINGS": "test-greetings-secret"},
        )
        == expected_dag
    )

from tests.example_flow import flow


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
                "outputs": [
                    {"name": "foo-output.json", "path": "task-output-foo.json"}
                ],
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
                    {"from": "foo-output.json", "path": "task-inputs-input_str.json"}
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
                    {"from": "foo-output.json", "path": "task-inputs-input_str.json"}
                ],
                "outputs": [
                    {
                        "name": "replace-letter-t-output.pkl",
                        "path": "task-output-replace-letter-t.pkl",
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
                        "from": "replace-letter-t-output.pkl",
                        "path": "task-inputs-first.pkl",
                    },
                    {
                        "from": "replace-letter-a-output.txt",
                        "path": "task-inputs-second.txt",
                    },
                ],
                "outputs": [
                    {"name": "concat-output.txt", "path": "task-output-concat.txt"}
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

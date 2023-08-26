import os
import inspect
from urllib.parse import urljoin
import argparse
from typing import Callable, Any
from dataclasses import dataclass
from flowmium import serializers
from flowmium.serializers import Serializer


@dataclass
class Input:
    arg_name: str
    depends: str
    frm: str
    path: str
    load: Callable[[str], Any]


@dataclass
class Output:
    name: str
    path: str
    dump: Callable[[Any, str], None]


@dataclass
class Task:
    name: str
    func: Callable
    inputs: list[Input]
    output: Output
    requires_flowctx: bool


@dataclass
class FlowContext:
    task_id: int


class ArgDoesNotExist(Exception):
    def __init__(self, arg_name: str) -> None:
        super().__init__(f"Arg {arg_name} does not exist")
        self.arg_name = arg_name


class Flow:
    INPUT_PATH_TEMPLATE = "task-inputs-{}.pkl"
    OUTPUT_PATH_TEMPLATE = "task-output-{}.pkl"
    OUTPUT_NAME_TEMPLATE = "{}-output"

    def __init__(self, name: str) -> None:
        self.tasks: list[Task] = []
        self.name = name
        self.serializers: dict[str, Serializer] = {}

    @staticmethod
    def _get_task_name(task_func: Callable) -> str:
        return task_func.__name__.replace("_", "-")

    def _parse_inputs_dict_tuple(
        self,
        arg_names_list: list[str],
        input_dict_tuple: tuple[str, Callable],
    ) -> Input:
        arg_name, inp_task_func = input_dict_tuple

        if arg_name not in arg_names_list:
            raise ArgDoesNotExist(arg_name=arg_name)

        input_task_name = Flow._get_task_name(inp_task_func)

        return Input(
            depends=input_task_name,
            arg_name=arg_name,
            frm=Flow.OUTPUT_NAME_TEMPLATE.format(input_task_name),
            path=Flow.INPUT_PATH_TEMPLATE.format(arg_name),
            load=self.serializers[input_task_name].load,
        )

    @staticmethod
    def _get_arg_names_list(task_func: Callable) -> tuple[list[str], bool]:
        arg_names_list = inspect.getfullargspec(task_func).args
        arg_names_list_no_flowctx = list(
            filter(lambda item: item != "flowctx", arg_names_list)
        )

        return arg_names_list_no_flowctx, "flowctx" in arg_names_list

    def task(
        self,
        inputs: dict[str, Callable] = {},
        serializer: Serializer = serializers.pkl,
    ) -> Callable:
        def task_decorator(task_func: Callable) -> Callable:
            task_name = Flow._get_task_name(task_func)
            self.serializers[task_name] = serializer

            task_output = Output(
                name=Flow.OUTPUT_NAME_TEMPLATE.format(task_name),
                path=Flow.OUTPUT_PATH_TEMPLATE.format(task_name),
                dump=serializer.dump,
            )

            arg_names_list, requires_flowctx = Flow._get_arg_names_list(
                task_func=task_func
            )

            task_inputs = [
                self._parse_inputs_dict_tuple(arg_names_list, item)
                for item in inputs.items()
            ]

            task_def = Task(
                name=task_name,
                func=task_func,
                inputs=task_inputs,
                output=task_output,
                requires_flowctx=requires_flowctx,
            )

            self.tasks.append(task_def)

            return task_func

        return task_decorator

    def run_task(self, flowctx: FlowContext) -> None:
        task_def = self.tasks[flowctx.task_id]

        args_dict = dict(
            [(inp.arg_name, inp.load(inp.path)) for inp in task_def.inputs]
        )

        if task_def.requires_flowctx:
            args_dict["flowctx"] = flowctx

        task_output = task_def.func(**args_dict)

        if task_output is not None:
            task_def.output.dump(task_output, task_def.output.path)

    def get_dag_dict(
        self, image: str, cmd: list[str], secrets_refs: dict[str, str]
    ) -> dict[str, Any]:
        tasks: list[dict[str, Any]] = []

        for task_id, task in enumerate(self.tasks):
            tasks.append(
                {
                    "name": task.name,
                    "image": image,
                    "depends": [inp.depends for inp in task.inputs],
                    "cmd": cmd,
                    "env": [
                        {
                            "name": "FLOWMIUM_FRAMEWORK_TASK_ID",
                            "value": f"{task_id}",
                        },
                    ]
                    + [
                        {"name": env_name, "fromSecret": secret_name}
                        for env_name, secret_name in secrets_refs.items()
                    ],
                    "inputs": [
                        {"from": inp.frm, "path": inp.path} for inp in task.inputs
                    ],
                    "outputs": [{"name": task.output.name, "path": task.output.path}],
                }
            )

        return {
            "name": self.name,
            "tasks": tasks,
        }

    def run(self, secrets_refs: dict[str, str] = {}) -> None:
        task_id = os.getenv("FLOWMIUM_FRAMEWORK_TASK_ID", default=None)

        if task_id is not None:
            flowctx = FlowContext(
                task_id=int(os.environ["FLOWMIUM_FRAMEWORK_TASK_ID"]),
            )

            self.run_task(flowctx)
        else:
            self.submit_flow(secrets_refs)

    def submit_flow(self, secrets_refs: dict[str, str] = {}) -> None:
        import requests

        parser = argparse.ArgumentParser()
        parser.add_argument("--cmd", required=True, type=str, nargs="+")
        parser.add_argument("--image", required=True, type=str)
        parser.add_argument("--flowmium-server", required=True, type=str)

        args = parser.parse_args()

        resp = requests.post(
            urljoin(args.flowmium_server, "/api/v1/job"),
            json=self.get_dag_dict(args.image, args.cmd, secrets_refs),
        )

        if resp.status_code != 200:
            print(resp.status_code)
            print(resp.text)
            exit(1)

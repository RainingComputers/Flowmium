import os
import inspect
import json
from typing import Callable, Any
from dataclasses import dataclass
from flowmium.serialize import dump, load


@dataclass
class Input:
    depends: str
    frm: str
    path: str


@dataclass
class Output:
    name: str
    path: str


@dataclass
class Task:
    name: str
    func: Callable
    arg_names: list[str]
    inputs: list[Input]
    output: Output


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

    @staticmethod
    def _get_task_name(task_func: Callable) -> str:
        return task_func.__name__.replace("_", "-")

    @staticmethod
    def _parse_inputs_dict_tuple(
        arg_names_list: list[str], input_dict_tuple: tuple[str, Callable]
    ) -> Input:
        arg_name, inp_task_func = input_dict_tuple

        if arg_name not in arg_names_list:
            raise ArgDoesNotExist(arg_name=arg_name)

        input_task_name = Flow._get_task_name(inp_task_func)

        return Input(
            depends=input_task_name,
            frm=Flow.OUTPUT_NAME_TEMPLATE.format(input_task_name),
            path=Flow.INPUT_PATH_TEMPLATE.format(arg_name),
        )

    def task(self, inputs: dict[str, Callable] = {}) -> Callable:
        # TODO: Choose default serailize and add a way to override the serializer
        # TODO: Add flowctx context

        def task_decorator(task_func: Callable) -> Callable:
            task_name = Flow._get_task_name(task_func)

            task_output = Output(
                name=Flow.OUTPUT_NAME_TEMPLATE.format(task_name),
                path=Flow.OUTPUT_PATH_TEMPLATE.format(task_name),
            )

            arg_names_list = inspect.getfullargspec(task_func).args

            task_inputs = [
                self._parse_inputs_dict_tuple(arg_names_list, item)
                for item in inputs.items()
            ]

            task_def = Task(
                name=task_name,
                func=task_func,
                inputs=task_inputs,
                output=task_output,
                arg_names=arg_names_list,
            )

            self.tasks.append(task_def)

            return task_func

        return task_decorator

    def run_task(self, task_id: int) -> None:
        task_def = self.tasks[task_id]

        args_dict = {}

        for arg_name in task_def.arg_names:
            input_file_path = Flow.INPUT_PATH_TEMPLATE.format(arg_name)
            args_dict[arg_name] = load(input_file_path)

        output = task_def.func(**args_dict)

        if output is not None:
            output_file_path = Flow.OUTPUT_PATH_TEMPLATE.format(task_def.name)
            dump(output, output_file_path)

        pass

    def get_dag_dict(self, image: str) -> dict[str, Any]:
        return {
            "name": self.name,
            "tasks": [
                {
                    "name": task.name,
                    "image": image,
                    "depends": [inp.depends for inp in task.inputs],
                    "cmd": [],
                    "env": [
                        {"name": "FLOWMIUM_FRAMEWORK_IN_EXECUTOR", "value": "true"},
                        {"name": "FLOWMIUM_FRAMEWORK_TASK_ID", "value": f"{task_id}"},
                    ],
                    "inputs": [
                        {"from": inp.frm, "path": inp.path} for inp in task.inputs
                    ],
                    "outputs": [{"name": task.output.name, "path": task.output.path}],
                }
                for task_id, task in enumerate(self.tasks)
            ],
        }

    def run(self) -> None:
        print(json.dumps(self.get_dag_dict("localhost:5000/framework-test:latest")))
        # task_id = int(os.environ["FLOWMIUM_FRAMEWORK_TASK_ID"])
        # self.run_task(task_id)

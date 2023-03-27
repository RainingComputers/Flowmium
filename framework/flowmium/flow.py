import os
import inspect
import json
import argparse
from typing import Callable, Any
from dataclasses import dataclass
from flowmium import default_serialize


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


@dataclass
class Serializer:
    dump: Callable[[Any, str], None]
    load: Callable[[str], Any]


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

    def task(
        self,
        inputs: dict[str, Callable] = {},
        serializer: Serializer = Serializer(
            dump=default_serialize.dump, load=default_serialize.load
        ),
    ) -> Callable:
        # TODO: Add flowctx context

        def task_decorator(task_func: Callable) -> Callable:
            task_name = Flow._get_task_name(task_func)
            self.serializers[task_name] = serializer

            task_output = Output(
                name=Flow.OUTPUT_NAME_TEMPLATE.format(task_name),
                path=Flow.OUTPUT_PATH_TEMPLATE.format(task_name),
                dump=serializer.dump,
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
            )

            self.tasks.append(task_def)

            return task_func

        return task_decorator

    def run_task(self, task_id: int) -> None:
        task_def = self.tasks[task_id]

        args_dict = dict(
            [(inp.arg_name, inp.load(inp.path)) for inp in task_def.inputs]
        )

        task_output = task_def.func(**args_dict)

        if task_output is not None:
            task_def.output.dump(task_output, task_def.output.path)

    def get_dag_dict(self, image: str, cmd: list[str]) -> dict[str, Any]:
        return {
            "name": self.name,
            "tasks": [
                {
                    "name": task.name,
                    "image": image,
                    "depends": [inp.depends for inp in task.inputs],
                    "cmd": cmd,
                    "env": [
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
        try:
            task_id = int(os.environ["FLOWMIUM_FRAMEWORK_TASK_ID"])
            self.run_task(task_id)
        except KeyError:
            parser = argparse.ArgumentParser()
            parser.add_argument("--cmd", required=True, type=list[str])
            parser.add_argument("--image", required=True, type=str)
            args = parser.parse_args()

            print(json.dumps(self.get_dag_dict(args.image, args.cmd)))

import inspect
from typing import Callable
from dataclasses import dataclass


@dataclass
class Input:
    frm: str
    path: str


@dataclass
class Output:
    name: str
    path: str


@dataclass
class Task:
    name: str
    inputs: list[Input]
    output: Output


class ArgDoesNotExist(Exception):
    def __init__(self, arg_name: str) -> None:
        super().__init__(f"Arg {arg_name} does not exist")
        self.arg_name = arg_name


class Flow:
    INPUT_PATH_TEMPLATE = "task-inputs/{}"
    OUTPUT_PATH_TEMPLATE = "task-output/{}"
    OUTPUT_NAME_TEMPLATE = "{}-output"

    def __init__(self) -> None:
        self.tasks: list[Task] = []

    @staticmethod
    def _parse_inputs_dict_tuple(
        arg_names_list: list[str], input_dict_tuple: tuple[str, Callable]
    ) -> Input:
        arg_name, inp_task_func = input_dict_tuple

        if arg_name not in arg_names_list:
            raise ArgDoesNotExist(arg_name=arg_name)

        return Input(
            frm=Flow.OUTPUT_NAME_TEMPLATE.format(inp_task_func.__name__),
            path=Flow.INPUT_PATH_TEMPLATE.format(arg_name),
        )

    def task(self, inputs: dict[str, Callable] = {}) -> Callable:
        def task_decorator(task_func: Callable) -> Callable:
            task_output = Output(
                name=Flow.OUTPUT_NAME_TEMPLATE.format(task_func.__name__),
                path=Flow.OUTPUT_PATH_TEMPLATE.format(task_func.__name__),
            )

            arg_names_list = inspect.getfullargspec(task_func).args

            task_inputs = list(
                map(
                    lambda item: self._parse_inputs_dict_tuple(arg_names_list, item),
                    inputs.items(),
                )
            )

            task_def = Task(
                name=task_func.__name__, inputs=task_inputs, output=task_output
            )

            self.tasks.append(task_def)

            return task_func

        return task_decorator

    def run_task(self, task_def: Task) -> None:
        # Deserialize inputs from file into memory

        # Run the function

        # Serailize the output to file

        pass

    def print_dag_def(self) -> None:
        print(self.tasks)

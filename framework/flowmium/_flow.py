import os
import inspect
import json
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
    """
    Defines a flow and provides decorator to register python functions as a task in that flow.

    Args:
        name: Name of the flow, this name will be used when you list flows using
            the :code:`flowctl` CLI tool.
    """

    INPUT_PATH_TEMPLATE = "task-inputs-{}.{}"
    OUTPUT_PATH_TEMPLATE = "task-output-{}.{}"
    OUTPUT_NAME_TEMPLATE = "{}-output.{}"

    def __init__(self, name: str) -> None:
        """ """
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

        input_file_ext = self.serializers[input_task_name].ext

        return Input(
            depends=input_task_name,
            arg_name=arg_name,
            frm=Flow.OUTPUT_NAME_TEMPLATE.format(input_task_name, input_file_ext),
            path=Flow.INPUT_PATH_TEMPLATE.format(arg_name, input_file_ext),
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
        """
        Decorator to register a function as a task in the flow by marking them as :code:`@flow.task`

        Args:
            inputs: Dictionary the holds dependent tasks of this function. The keys
                of the dictionary should the name of one of the function's argument and
                the value is another function that has been marked with this decorator as
                :code:`@flow.task`. The output of the other function will be passed as an argument
                to this function.

            serializer: Each task (function marked as :code:`@flow.task`) runs in its own pod.
                To pass the return output of one task to another, the orchestrator serializes the
                output and uploads it to s3 like storage (like MinIO). This argument accepts the
                serializer to be used for the output of the task.
                The default serializer is pickle. There are other serializers available and you can
                also define your own. See :ref:`serializers` section for more details.
        """

        def task_decorator(task_func: Callable) -> Callable:
            task_name = Flow._get_task_name(task_func)
            self.serializers[task_name] = serializer

            task_output = Output(
                name=Flow.OUTPUT_NAME_TEMPLATE.format(task_name, serializer.ext),
                path=Flow.OUTPUT_PATH_TEMPLATE.format(task_name, serializer.ext),
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

    def _run_task(self, flowctx: FlowContext) -> None:
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
        """
        Construct the flow definition that will be submitted to the orchestrator.

        Args:
            image: All python flows (packages and modules that call :code:`flow.run()` when
                executed) are packaged into a docker image and uploaded to registry that is
                accessible to the kubernetes cluster that the executor is running in.
                This argument is the name of the uploaded image,
                example :code:`localhost:5180/py-flow-test:latest`.

            cmd: This command to execute the module or package which inturn will run
                :code:`flow.run()`. For example :code:`['python3' 'test.py']`.

            secrets_refs: Dictionary that maps secrets held by the orchestrator to environment
                variables for the task. The key is the name of the environment variable for the
                task and the value if the name of the secret registered in the orchestrator.
        """
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
        """
        Runs the flow. Your module or package should call this function when executed
        using :code:`python3` or :code:`python3 -m` to be able to run as a task from the
        orchestrator.

        Args:
            secrets_refs: Dictionary that maps secrets held by the orchestrator to environment
                variables for the task. The key is the name of the environment variable for the
                task and the value if the name of the secret registered in the orchestrator.
        """

        task_id = os.getenv("FLOWMIUM_FRAMEWORK_TASK_ID", default=None)

        if task_id is not None:
            flowctx = FlowContext(
                task_id=int(os.environ["FLOWMIUM_FRAMEWORK_TASK_ID"]),
            )

            self._run_task(flowctx)
        else:
            self._submit_flow(secrets_refs)

    def _submit_flow(self, secrets_refs: dict[str, str] = {}) -> None:
        import requests

        parser = argparse.ArgumentParser()
        parser.add_argument("--cmd", required=True, type=str)
        parser.add_argument("--image", required=True, type=str)
        parser.add_argument("--flowmium-server", required=True, type=str)
        parser.add_argument("-m", required=False, type=str)
        parser.add_argument(
            "--dry-run", required=False, default=False, action="store_true"
        )

        args = parser.parse_args()

        dag = self.get_dag_dict(args.image, args.cmd.split(" "), secrets_refs)

        if args.dry_run:
            print(json.dumps(dag, indent=4))
            return

        resp = requests.post(
            urljoin(args.flowmium_server, "/api/v1/job"),
            json=dag,
        )

        if resp.status_code != 200:
            print(resp.status_code)
            print(resp.text)
            exit(1)

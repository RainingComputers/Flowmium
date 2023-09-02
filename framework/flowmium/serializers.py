"""
Contains built in serializers

.. data:: pkl

    Uses python pickle to serialize data.

.. data:: plain_text

    Uses the :code:`str()` function in python to serialize to plain text.

.. data:: json_text

    Uses the :code:`json` module to serialize data to json string.
"""

import pickle
import json
from typing import Any, Callable, IO
from pathlib import Path

from dataclasses import dataclass


__all__ = ["pkl", "plain_text", "json_text"]


@dataclass
class Serializer:
    """
    Define a custom serializer

    Args:
        dump_func: A functions that takes in any object and a file pointer and
            writes the serialized form of the object to the file pointer.
        load_func: A function that takes in a file pointer and returns a python
            object or type by deserializing the contents of the file pointer.
        ext: Name of the file extension used by the serialization format, example :code:`json`
        write_mode: The mode of the file pointer for :code:`dump_func`. For example :code:`'wb'`
            or :code:`'w'`.
        read_mode: The mode of the file pointer  for :code:`load_func`. For example :code:`'rb'`
            or :code:`'r'`.
    """

    dump_func: Callable[[Any, IO[Any]], Any]
    load_func: Callable[[IO[Any]], Any]
    ext: str
    write_mode: str
    read_mode: str

    def dump(self, obj: Any, path: str) -> None:
        Path(path).parent.mkdir(parents=True, exist_ok=True)

        with open(path, self.write_mode) as output_file:
            self.dump_func(obj, output_file)

    def load(self, path: str) -> Any:
        with open(path, self.read_mode) as output_file:
            return self.load_func(output_file)


pkl = Serializer(
    dump_func=pickle.dump,
    load_func=pickle.load,
    ext="pkl",
    write_mode="wb",
    read_mode="rb",
)

plain_text = Serializer(
    dump_func=lambda obj, fp: fp.write(str(obj)),
    load_func=lambda fp: fp.read(),
    ext="txt",
    write_mode="w",
    read_mode="r",
)

json_text = Serializer(
    dump_func=json.dump,
    load_func=json.load,
    ext="json",
    write_mode="w",
    read_mode="r",
)

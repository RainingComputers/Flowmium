import pickle
from typing import Any, Callable
from pathlib import Path

from dataclasses import dataclass


__all__ = ["pkl", "plain_text"]


@dataclass
class Serializer:
    dump: Callable[[Any, str], None]
    load: Callable[[str], Any]


def dump_pkl(obj: Any, path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)

    with open(path, "wb") as output_file:
        pickle.dump(obj, output_file)


def load_pkl(path: str) -> Any:
    with open(path, "rb") as output_file:
        return pickle.load(output_file)


pkl = Serializer(
    dump=dump_pkl, load=load_pkl
)


def dump_plain_text(obj: Any, path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)

    with open(path, "w") as output_file:
        output_file.write(str(obj))


def load_plain_text(path: str) -> str:
    with open(path, "r") as output_file:
        return output_file.read()


plain_text = Serializer(
    dump=dump_plain_text, load=load_plain_text
)

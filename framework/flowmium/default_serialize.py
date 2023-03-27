import pickle
from typing import Any
from pathlib import Path


def dump(obj: Any, path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)

    with open(path, "wb") as output_file:
        pickle.dump(obj, output_file)


def load(path: str) -> Any:
    with open(path, "rb") as output_file:
        return pickle.load(output_file)

from flowmium import Flow


flow = Flow("testing")


@flow.task()
def foo() -> str:
    return "Hallo worlt"


@flow.task({"input_str": foo})
def replace_letter_a(input_str: str) -> str:
    return input_str.replace("a", "e")


@flow.task({"input_str": foo})
def replace_letter_t(input_str: str) -> str:
    return input_str.replace("t", "d")


@flow.task({"first": replace_letter_t, "second": replace_letter_a})
def concat(first: str, second: str) -> str:
    return f"{first} {second}"


flow.run()

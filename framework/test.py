from flowmium import Flow, FlowContext
from flowmium.serializers import plain_text


flow = Flow("testing")


@flow.task(serializer=plain_text)
def foo() -> str:
    return "Hallo world"


@flow.task({"input_str": foo}, serializer=plain_text)
def replace_letter_a(input_str: str, flowctx: FlowContext) -> str:
    return input_str.replace("a", "e") + str(flowctx.task_id)


@flow.task({"input_str": foo}, serializer=plain_text)
def replace_letter_t(input_str: str) -> str:
    return input_str.replace("t", "d")


@flow.task(
    {"first": replace_letter_t, "second": replace_letter_a},
    serializer=plain_text
)
def concat(first: str, second: str) -> str:
    return f"{first} {second}"


flow.run()

flowmium
========

This is the documentation for Python client and flow definition framework for the Flowmium Kubernetes Workflow orchestrator.

Installation
------------

.. code-block:: bash

   python3 -m pip install flowmium


Defining flows
--------------

A flow is made of multiple tasks that are dependent on each other. Each task is a Python function and runs in its own
Kubernetes pod. Two tasks are dependent on each other if one tasks uses the output of another task. A function can be turned into
a task and the dependencies between tasks can also be marked using the :code:`@flow.task` decorator.

.. autoclass:: flowmium.Flow

    .. automethod:: task

    .. automethod:: run

    .. automethod:: get_dag_dict

Example
^^^^^^^

An example flow would look like this. 

.. literalinclude:: ../test.py

To run this, first you would package it as a docker image and upload it to a registry that
is accessible from the Kubernetes cluster that the orchestrator is running on. Then you can run something like
below to submit and run the flow

.. code-block::

    python3 flow.py --image registry:5000/py-flow-test:latest --cmd python3 flow.py --flowmium-server http://localhost:8080

.. _serializers:

Serializers
-----------

Each task (function marked as :code:`@flow.task`) runs in its own pod. To pass the return output of one task to another,
the orchestrator serializes the output and uploads it to s3 like storage (like MinIO).
The default serializer is pickle. There are other serializers available and you can also define your own.

Included serializers
^^^^^^^^^^^^^^^^^^^^

.. automodule:: flowmium.serializers


Defining a custom serializer
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

To define a custom serializer, you instantiate the :code:`Serializer` class and pass it as an argument to
the :code:`@flow.task` decorator.

.. autoclass:: flowmium.serializers.Serializer


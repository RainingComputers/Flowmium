# Getting started with python script workflow

-   Deploy flowmium on local by following these [steps](../deployment/)

-   Build python flow and push it to the registry (NOTE: It is recommended to change version with each build instead of latest)

    ```
    docker build . -t python-script-workflow-test
    docker tag python-script-workflow-test localhost:5180/python-script-workflow-test:latest
    docker push localhost:5180/python-script-workflow-test:latest
    ```

-   Install flowmium

    ```
    python3 -m pip install flowmium
    ```

-   Submit flow to executor (NOTE: Add `--dry-run` to see YAML definition without submitting the flow)

    ```
    python3 my_flow.py --image registry:5000/python-script-workflow-test:latest --cmd 'python3 my_flow.py' --flowmium-server http://localhost:8080
    ```

-   Download output artefact

    ```
    flowctl download <flow-id> concat-output.txt .
    ```

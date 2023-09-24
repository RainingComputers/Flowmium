# Getting started with python module workflow

-   Deploy flowmium on local be following these [steps](../deployment/)

-   Build python flow and push it to the registry (NOTE: It is recommended to change version with each build instead of latest)

    ```
    docker build . -t python-package-workflow-test
    docker tag python-package-workflow-test localhost:5000/python-package-workflow-test:latest
    docker push localhost:5000/python-package-workflow-test:latest
    ```

-   Submit flow to executor (NOTE: Add `--dry-run` to see YAML definition without submitting the flow)

    ```
    python3 -m my_flow --image registry:5000/python-package-workflow-test:latest --cmd 'python3 -m my_flow' --flowmium-server http://localhost:8080
    ```

-   Download output artefact

    ```
    flowctl download <flow-id> concat-output.txt .
    ```

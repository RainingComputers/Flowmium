# Getting started with python script workflow

-   Build python flow and push it to the registry (NOTE: It is recommended to change version with each build instead of latest)

    ```
    docker build . -t python-script-workflow-test
    docker tag python-script-workflow-test localhost:5000/python-script-workflow-test:latest
    docker push localhost:5000/python-script-workflow-test:latest
    ```

-   Submit flow to executor

    ```
    python3 my_flow.py --image registry:5000/python-script-workflow-test:latest --cmd 'python3 my_flow.py' --flowmium-server http://localhost:8080
    ```

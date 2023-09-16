# Getting started with python module workflow

-   Run postgres, minio, registry, kubernetes and flowmium on your machine as docker containers

    ```
    docker-compose -f test-services.yaml up
    ```

-   Build python flow and push it to the registry (NOTE: It is recommended to change version with each build instead of latest)

    ```
    docker build . -t python-package-workflow-test
    docker tag python-package-workflow-test localhost:5000/python-package-workflow-test:latest
    docker push localhost:5000/python-package-workflow-test:latest
    ```

-   Watch kubernetes pods

    ```
    KUBECONFIG=./kubeconfig.yaml watch kubectl get pods
    ```

-   Watch for flows status

    ```
    watch flowctl list
    ```

-   Submit flow to executor

    ```
    python3 -m my_flow --image registry:5000/python-package-workflow-test:latest --cmd 'python3 -m my_flow' --flowmium-server http://localhost:8080
    ```

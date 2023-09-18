# Deploying flowmium

You can deploy flowmium on your local machine for testing and for production

## Local testing using docker compose

-   Install flowctl

    ```
    cargo install flowmium  # TODO: confirm this
    ```

-   Run a registry and kubernetes on your machine as docker containers

    ```
    docker-compose up
    ```

-   Deploy flowmium onto the cluster

    ```
    KUBECONFIG=./kubeconfig.yaml kubectl apply -f kubernetes.yaml
    ```

-   Watch kubernetes pods

    ```
    KUBECONFIG=./kubeconfig.yaml watch kubectl get pods
    ```

-   Expose flowmium server

    ```
    KUBECONFIG=./kubeconfig.yaml svc/flowmium-server-service 8080
    ```

-   Watch for flows status

    ```
    watch flowctl list
    ```

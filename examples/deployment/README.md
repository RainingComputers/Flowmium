# Deploying flowmium

You can deploy flowmium on your local machine for testing and for production

## Local testing using docker compose

-   Install flowctl

    ```
    cargo install flowmium
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
    KUBECONFIG=./kubeconfig.yaml kubectl port-forward  svc/flowmium-server-service 8080
    ```

-   Watch for flows status

    ```
    watch flowctl list
    ```

## For production

You can use [`kubernetes.yaml`](kubernetes.yaml) in this example folder for deploying flowmium **but replace the postgres and minio deployments with other helm charts**. Flowmium is also not designed to be run as multiple instances or replicas. You may also want to configure the server by setting following environment variables

| Name                          | Description                                                                                                                                                                  | Example                                           |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| FLOWMIUM_POSTGRES_URL         | URL to postgres database                                                                                                                                                     | `postgres://flowmium:flowmium@localhost/flowmium` |
| FLOWMIUM_STORE_URL            | URL to s3 compatible storage like MinIO                                                                                                                                      | `http://172.16.238.4:9000`                        |
| FLOWMIUM_TASK_STORE_URL       | URL to s3 from within the cluster, this will be the same as `FLOWMIUM_STORE_URL` for most cases, this would be diffrent if s3 and the server are running outside the cluster | `http://172.16.238.4:9000`                        |
| FLOWMIUM_BUCKET_NAME          | Name of the bucket to store artefact in                                                                                                                                      | `flowmium-test`                                   |
| FLOWMIUM_ACCESS_KEY           | Access key for s3                                                                                                                                                            | `minio`                                           |
| FLOWMIUM_SECRET_KEY           | Secret key for s3                                                                                                                                                            | `password`                                        |
| FLOWMIUM_INIT_CONTAINER_IMAGE | Image to use for the init container                                                                                                                                          | `docker.io/shnoo28/flowmium:latest`               |
| FLOWMIUM_NAMESPACE            | Namespace to spawn or deploy jobs in                                                                                                                                         | `default`                                         |
| KUBECONFIG                    | Path to kubeconfig, not required if a kubernetes service account is attached                                                                                                 | `./kubeconfig.yaml`                               |

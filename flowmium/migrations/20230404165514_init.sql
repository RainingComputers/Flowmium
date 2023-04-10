CREATE TYPE flow_status AS ENUM ('pending', 'running', 'success', 'failed');

CREATE TABLE flows (
    id SERIAL PRIMARY KEY,
    plan JSON NOT NULL,
    current_stage INTEGER NOT NULL,
    running_tasks INTEGER[] NOT NULL,
    finished_tasks INTEGER[] NOT NULL,
    failed_tasks INTEGER[] NOT NULL,
    task_definitions JSON NOT NULL,
    flow_name TEXT NOT NULL,
    status flow_status NOT NULL
);

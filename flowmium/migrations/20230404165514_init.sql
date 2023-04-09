CREATE TYPE flow_status AS ENUM ('pending', 'running', 'success', 'failed');

CREATE TABLE flows (
    id SERIAL PRIMARY KEY,
    plan JSON,
    current_stage INTEGER,
    running_tasks INTEGER[],
    finished_tasks INTEGER[],
    failed_tasks INTEGER[],
    task_definitions JSON,
    flow_name TEXT,
    status flow_status
);

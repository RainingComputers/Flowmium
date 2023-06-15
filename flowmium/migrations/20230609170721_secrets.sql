CREATE TABLE secrets (
    id SERIAL PRIMARY KEY,
    secret_key TEXT UNIQUE NOT NULL,
    secret_value TEXT NOT NULL
)
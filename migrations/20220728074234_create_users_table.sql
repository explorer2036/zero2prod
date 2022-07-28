CREATE TABLE users (
    id uuid NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);
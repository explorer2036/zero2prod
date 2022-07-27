CREATE TABLE subscription_tokens (
    token TEXT NOT NULL,
    id uuid NOT NULL REFERENCES subscriptions (id),
    PRIMARY KEY (token)
);
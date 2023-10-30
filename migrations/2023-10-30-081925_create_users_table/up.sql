CREATE TABLE users (
      id SERIAL NOT NULL PRIMARY KEY,
      merchant_id VARCHAR(255) NOT NULL,
      name BYTEA NOT NULL,
      created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
        email VARCHAR(255) NOT NULL,
    password BYTEA NOT NULL
);

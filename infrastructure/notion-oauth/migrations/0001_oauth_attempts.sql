CREATE TABLE oauth_attempts (
  state TEXT PRIMARY KEY,
  verifier_hash TEXT NOT NULL,
  encrypted_token TEXT,
  expires_at INTEGER NOT NULL
);

CREATE INDEX oauth_attempts_expiry ON oauth_attempts (expires_at);

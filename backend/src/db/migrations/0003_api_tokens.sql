-- Long-lived API tokens (e.g. for the MCP server). Only a hash of the token is
-- stored, never the raw value -- it is shown to the user once, at creation.
CREATE TABLE api_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  token_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_used_at TEXT
);

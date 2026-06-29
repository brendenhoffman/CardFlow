-- OAuth 2.1 authorization code + token storage, one row per grant. A single
-- row transitions: code issued -> exchanged for access+refresh -> refreshed
-- (in place) -> ... The PKCE challenge and the redirect_uri used at /authorize
-- must be re-validated at /token, so both are stored alongside the code (the
-- code itself is single-use and is cleared, along with the challenge/uri,
-- once exchanged).
CREATE TABLE oauth_tokens (
  id                    TEXT PRIMARY KEY,
  user_id               TEXT NOT NULL REFERENCES users(id),
  code                  TEXT,                  -- authorization code, null after exchange
  code_expires_at       TEXT,
  code_challenge        TEXT,                  -- PKCE challenge from /authorize, null after exchange
  code_challenge_method TEXT,                  -- always "S256"
  redirect_uri          TEXT,                  -- must match at /token, null after exchange
  access_token          TEXT,
  access_expires_at     TEXT,
  refresh_token         TEXT,
  refresh_expires_at    TEXT,
  client_id             TEXT NOT NULL,         -- the MCP server client id
  created_at            TEXT NOT NULL
);

CREATE UNIQUE INDEX oauth_tokens_code_idx ON oauth_tokens(code) WHERE code IS NOT NULL;
CREATE UNIQUE INDEX oauth_tokens_access_token_idx ON oauth_tokens(access_token) WHERE access_token IS NOT NULL;
CREATE UNIQUE INDEX oauth_tokens_refresh_token_idx ON oauth_tokens(refresh_token) WHERE refresh_token IS NOT NULL;

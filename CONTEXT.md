# Cardflow — Project Context

## What is this?

A self-hosted, Dockerized project management tool built around a card game metaphor. Designed for solo developers managing game development projects. The UI should feel tactile and fun — drawing cards, holding a hand, playing cards to complete work.

## Core Concepts

| Term | Maps To | Description |
|------|---------|-------------|
| Game | Project | A game being developed. Users are added to games to see them. |
| Deck | Epic/Sprint | A themed body of work within a game (e.g. "Renderer", "World Gen"). Has its own hand. |
| Card | User Story | A single unit of work. Lives in the pile, hand, or history. |
| Pile | Backlog | All unstarted cards for a deck. |
| Hand | Active Stories | Up to 5 cards actively being worked on, ranked 1-5 by priority. |

## Stack

- **Backend:** Rust, Axum, SQLx, SQLite
- **Frontend:** Svelte + TypeScript
- **Auth:** JWT + TOTP (RFC 6238) MFA
- **Deployment:** Docker Compose, single SQLite file on a named volume

## Project Structure

```
cardflow/
  backend/
    src/
      main.rs
      db/
        mod.rs
        migrations/
      models/
        mod.rs
        game.rs
        deck.rs
        card.rs
        user.rs
      routes/
        mod.rs
        games.rs
        decks.rs
        cards.rs
        auth.rs
      errors.rs
    Cargo.toml
  frontend/
    src/
      lib/
        components/
          Game.svelte
          Deck.svelte
          Card.svelte
        api.ts
      routes/
        +page.svelte
    package.json
    svelte.config.js
  docker-compose.yml
  Dockerfile.backend
  Dockerfile.frontend
```

## Database Schema

```sql
-- Users
CREATE TABLE users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  totp_secret TEXT,                  -- nullable, null = MFA not configured
  role TEXT NOT NULL DEFAULT 'user', -- 'admin' | 'user'
  created_at TEXT NOT NULL
);

-- Games (projects)
CREATE TABLE games (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'active', -- 'active' | 'archived'
  created_at TEXT NOT NULL
);

-- Game membership (groups = games)
CREATE TABLE game_members (
  game_id TEXT NOT NULL REFERENCES games(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  role TEXT NOT NULL DEFAULT 'member',   -- 'owner' | 'member'
  joined_at TEXT NOT NULL,
  PRIMARY KEY (game_id, user_id)
);

-- Decks (epics/sprints)
CREATE TABLE decks (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES games(id),
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'active', -- 'active' | 'archived'
  created_at TEXT NOT NULL
);

-- Cards (stories)
CREATE TABLE cards (
  id TEXT PRIMARY KEY,
  deck_id TEXT NOT NULL REFERENCES decks(id),
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'pile',   -- 'pile' | 'hand' | 'done'
  priority INTEGER,                      -- 1-5, only set when status = 'hand', unique per deck
  created_at TEXT NOT NULL,
  completed_at TEXT                      -- null until done
);

-- Jokers (dependency tree)
-- A card can have multiple joker parents, each must be completed before the card is unblocked.
-- Jokers can have jokers, forming an arbitrary depth tree. Don't abuse it.
CREATE TABLE card_jokers (
  id TEXT PRIMARY KEY,
  card_id TEXT NOT NULL REFERENCES cards(id),   -- the card that has a dependency
  joker_id TEXT NOT NULL REFERENCES cards(id),  -- the card that must be completed first
  order INTEGER NOT NULL,                        -- sequence within this card's joker list
  UNIQUE(card_id, order)
);

-- Long-lived API tokens (e.g. for the MCP server). Only an argon2 hash is
-- stored; the raw value is shown to the user once, at creation.
CREATE TABLE api_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  token_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_used_at TEXT
);

-- OAuth 2.1 (authorization code + PKCE) grants, one row per grant in flight
-- or active. code/code_challenge/redirect_uri are cleared once exchanged;
-- access_token/refresh_token are rotated in place on each refresh.
CREATE TABLE oauth_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  code TEXT,                            -- null after exchange
  code_expires_at TEXT,
  code_challenge TEXT,                  -- PKCE S256 challenge, null after exchange
  code_challenge_method TEXT,
  redirect_uri TEXT,                    -- re-validated at /token, null after exchange
  access_token TEXT,
  access_expires_at TEXT,
  refresh_token TEXT,
  refresh_expires_at TEXT,
  client_id TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

### Constraints to enforce in application logic:
- Max 5 cards per deck where `status = 'hand'`
- `priority` must be 1-5 and unique within a deck's hand
- When reordering, step priorities in a single transaction (no gaps, no conflicts)

### Joker (dependency) rules:
- A card can have multiple joker parents via `card_jokers`, each with an explicit order
- Jokers can have jokers — forms an arbitrary depth tree, not just a chain
- Drawing a card walks the entire dependency tree and pulls all unresolved nodes into the hand as a single stack occupying one hand slot
- Stack is completed top-down — when all jokers for a card are done, that card becomes active
- When the tree root is completed, the hand slot frees and optionally triggers a draw prompt
- Returning a stack to the pile preserves it exactly — order and joker relationships unchanged
- Stacks can be reordered in the hand like a single card (priority applies to the root)
- UI should show a collapsed stack by default with expand to see the full tree

## API Surface

### Auth
```
POST /auth/login                   -- returns JWT + sets refresh token cookie
POST /auth/refresh                 -- rotate JWT using refresh token
POST /auth/logout
POST /auth/mfa/setup               -- generate TOTP secret for user
POST /auth/mfa/verify              -- confirm TOTP code to activate MFA
```

### Users (admin only)
```
GET    /users
POST   /users                      -- admin creates users, no public signup
PATCH  /users/:id
DELETE /users/:id
```

### Games
```
GET    /games                      -- only games the user is a member of
POST   /games
PATCH  /games/:id
DELETE /games/:id
POST   /games/:id/members          -- add user to game
DELETE /games/:id/members/:user_id -- remove user from game
```

### Decks
```
GET    /games/:game_id/decks
POST   /games/:game_id/decks
PATCH  /decks/:id
DELETE /decks/:id
```

### Cards
```
GET    /decks/:deck_id/cards
POST   /decks/:deck_id/cards
PATCH  /cards/:id
DELETE /cards/:id
```

### Hand Actions
```
POST   /decks/:deck_id/deal        -- randomly fill hand up to 5 from pile
POST   /decks/:deck_id/draw/:card_id -- manually draw a specific card from pile
POST   /cards/:id/complete         -- mark done, completed_at set to now
POST   /cards/:id/return           -- send hand card back to pile, clears priority
PATCH  /decks/:deck_id/reorder     -- body: { order: [card_id, ...] } length 1-5, steps priorities 1-N in transaction
```

### API tokens
```
GET    /api-tokens                 -- list current user's tokens (never returns the raw token)
POST   /api-tokens                 -- create; raw token returned once, in this response only
DELETE /api-tokens/:id             -- revoke
```

### OAuth 2.1 (authorization code + PKCE)
```
GET    /oauth/authorize            -- public. No/invalid Bearer JWT -> 302 to the frontend
                                       login page with the request forwarded; valid Bearer JWT
                                       -> mints a code, returns { redirect_to } as JSON (the
                                       frontend navigates the browser there itself)
POST   /oauth/token                -- public, client_secret_post auth. grant_type=authorization_code
                                       (+ code_verifier, redirect_uri) or grant_type=refresh_token
```
Only enabled when `OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET`/`CARDFLOW_PUBLIC_URL` are set (errors at
request time, not startup, if missing — these routes always exist). This is what lets the MCP
server (see below) be added to claude.ai web as a remote connector.

## Auth Design

- JWT expiry (2 hours), refresh token in HttpOnly cookie (7 days), both stored/validated against SQLite. Frontend silently refreshes the access token shortly before it expires so an open tab is never logged out before the refresh token's 7-day boundary.
- TOTP via RFC 6238 (compatible with Google Authenticator, Aegis, etc.)
- No public signup — admin creates users via `POST /users` or a CLI command
- MFA is optional per user but can be made required by admin policy (future)
- Permissions:
  - `admin` — can manage users, see all games
  - `owner` (per game) — can edit/delete the game, manage members, full card control
  - `member` (per game) — can view game, move/complete cards, add cards
- `require_auth` middleware accepts any of three bearer-token flavors on the same
  `Authorization: Bearer <value>` header, tried cheapest-first: session JWT (no extra DB hit
  beyond the user lookup) → OAuth access token (`oauth_tokens`, exact indexed lookup, 2h TTL) →
  API token (`api_tokens`, argon2-verify against every stored hash, since it's salted/non-deterministic
  and can't be indexed — fine at this app's scale).
- There is no dynamic OAuth client registration (RFC 7591) yet — exactly one client is
  registered, statically, via the `OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET` env vars (set
  identically on both `cardflow-backend` and `cardflow-mcp`). A connecting OAuth client
  (e.g. claude.ai's custom connector setup) needs a way to be configured with that exact
  client_id/secret.

## Cargo.toml Dependencies

```toml
[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "migrate", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.5", features = ["cors"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = "0.3"
argon2 = "0.5"
totp-rs = { version = "5", features = ["gen_secret", "qr"] }
axum-extra = { version = "0.9", features = ["cookie"] }
jsonwebtoken = "9"
```

## Docker Setup

- Three containers: `cardflow-backend`, `cardflow-frontend`, `cardflow-mcp`
- SQLite file mounted as a named volume at `/data/cardflow.db`
- Backend exposes port 3001, frontend exposes port 3000, mcp exposes `MCP_PORT` (default 3778)
- Frontend proxies `/api` to backend so no CORS issues in production
- `.env` file for secrets: `JWT_SECRET`, `DATABASE_URL`, etc.
- `cardflow-mcp` is a separate Rust crate (`mcp/`) implementing the MCP protocol over the
  classic SSE transport (`/sse` + `/message`), calling the backend over the internal Docker
  network. `CARDFLOW_TOKEN` is the simple path (one static API token = one fixed identity for
  every connection); OAuth (`OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET`/`MCP_PUBLIC_URL`/
  `CARDFLOW_PUBLIC_URL`, all-or-nothing) lets each connecting client authenticate as its own
  Cardflow user instead — both can be configured at once, and a per-connection bearer token
  always takes priority over the static fallback.

## UI Notes

- The hand should feel like holding cards — visual card components, draggable to reorder
- Completing a card should feel satisfying — animation out, optional auto-draw prompt
- Deal button for fresh decks — randomly fills hand, good for getting started fast
- Pile shows a count and a scrollable list to manually pick from
- Archived games/decks accessible but visually de-emphasized
- History view per deck showing completed cards with timestamps

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

## Auth Design

- JWT expiry (2 hours), refresh token in HttpOnly cookie (7 days), both stored/validated against SQLite. Frontend silently refreshes the access token shortly before it expires so an open tab is never logged out before the refresh token's 7-day boundary.
- TOTP via RFC 6238 (compatible with Google Authenticator, Aegis, etc.)
- No public signup — admin creates users via `POST /users` or a CLI command
- MFA is optional per user but can be made required by admin policy (future)
- Permissions:
  - `admin` — can manage users, see all games
  - `owner` (per game) — can edit/delete the game, manage members, full card control
  - `member` (per game) — can view game, move/complete cards, add cards

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

- Two containers: `cardflow-backend` and `cardflow-frontend`
- SQLite file mounted as a named volume at `/data/cardflow.db`
- Backend exposes port 3001, frontend exposes port 3000
- Frontend proxies `/api` to backend so no CORS issues in production
- `.env` file for secrets: `JWT_SECRET`, `DATABASE_URL`, etc.

## UI Notes

- The hand should feel like holding cards — visual card components, draggable to reorder
- Completing a card should feel satisfying — animation out, optional auto-draw prompt
- Deal button for fresh decks — randomly fills hand, good for getting started fast
- Pile shows a count and a scrollable list to manually pick from
- Archived games/decks accessible but visually de-emphasized
- History view per deck showing completed cards with timestamps

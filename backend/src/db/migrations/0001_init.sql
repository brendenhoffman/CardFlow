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
  "order" INTEGER NOT NULL,                     -- sequence within this card's joker list
  UNIQUE(card_id, "order")
);

# Cardflow

A self-hosted project manager built around a card game. Designed for solo developers who want something lightweight, satisfying to use, and free of the ceremony that makes team-oriented tools painful for one person.

## The idea

Most project management tools are built for teams. They come with sprints, standups, velocity charts, and a dozen concepts you have to ignore to get anything done alone. Cardflow throws that out and replaces it with something more honest: a hand of cards.

You have **games** (projects), **decks** (areas of focus like "renderer" or "world gen"), and **cards** (the actual work). Your active cards sit in your hand — up to five per deck, ranked by priority. When you finish one, you draw another. That's it.

The one twist is **jokers** — a dependency system that lets you attach prerequisite cards to any card in the pile. Draw a card that has jokers and the whole dependency stack comes with it, occupying one slot in your hand until the chain is resolved. Useful when you actually need it. Evil and abusable like all "good" card games.

## Stack

- **Backend** — Rust, Axum, SQLx, SQLite
- **Frontend** — Svelte, TypeScript
- **Auth** — JWT with TOTP MFA support
- Self-hosted, runs in Docker

## Installation

Pull the published images and run with Docker Compose — no cloning the repo, no build step.

```yaml
services:
  cardflow-backend:
    image: ghcr.io/brendenhoffman/cardflow-backend:latest
    container_name: cardflow-backend
    environment:
      DATABASE_URL: sqlite:///data/cardflow.db
      # Uncomment if you are not using HTTPS
      # COOKIE_SECURE: "false"
      # Optional, for claude.ai web (or similar) connecting to the MCP server
      # via OAuth instead of a static API token -- see the cardflow-mcp
      # service below. Leave all three commented out to disable OAuth
      # entirely; existing API token auth is unaffected either way.
      # OAUTH_CLIENT_ID: <your-client-id>
      # OAUTH_CLIENT_SECRET: <your-client-secret>
      # CARDFLOW_PUBLIC_URL: https://cardflow.example.com
    volumes:
      - cardflow-data:/data
    restart: unless-stopped

  cardflow-frontend:
    image: ghcr.io/brendenhoffman/cardflow-frontend:latest
    container_name: cardflow-frontend
    depends_on:
      - cardflow-backend
    ports:
      - "8777:80"
    restart: unless-stopped

  # Optional: MCP server for AI agents (Claude, etc.) to manage games/decks/cards.
  cardflow-mcp:
    image: ghcr.io/brendenhoffman/cardflow-mcp:latest
    container_name: cardflow-mcp
    environment:
      CARDFLOW_URL: http://cardflow-backend:3001
      # Generate this from the Cardflow UI under Settings (profile/settings) after first boot.
      # Used as-is if a connecting client doesn't authenticate itself (e.g. via OAuth below).
      CARDFLOW_TOKEN: <your-api-token>
      MCP_PORT: 3778
      # Optional: lets claude.ai web (or any OAuth-capable MCP client) connect
      # by signing in with its own Cardflow account instead of sharing
      # CARDFLOW_TOKEN. All four must be set together (matching OAUTH_CLIENT_ID/
      # SECRET on cardflow-backend above) or all left commented out -- the
      # server refuses to start if only some are set.
      # OAUTH_CLIENT_ID: <your-client-id>
      # OAUTH_CLIENT_SECRET: <your-client-secret>
      # MCP_PUBLIC_URL: https://mcp.example.com
      # CARDFLOW_PUBLIC_URL: https://cardflow.example.com
    depends_on:
      - cardflow-backend
    ports:
      - "3778:3778"
    restart: unless-stopped

volumes:
  cardflow-data:
```

Save that as `docker-compose.yml`, then:

```sh
docker compose up -d
```

Visit `http://localhost:8777` and follow the first-run setup screen to create your admin account. The JWT signing secret is generated automatically on first boot and persisted in the `cardflow-data` volume alongside the SQLite database, so both survive restarts and upgrades.

### Connecting claude.ai web to the MCP server

The MCP server supports two ways to authenticate a connecting client:

- **Static API token** (simplest): create a token under Settings → API tokens in the app, set it as `CARDFLOW_TOKEN` on `cardflow-mcp`, and any MCP client pointed at the server uses that one fixed identity.
- **OAuth 2.1** (for claude.ai web, or any client that can't be configured with a static bearer header): set the four `OAUTH_*`/`*_PUBLIC_URL` variables above on both services. `OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET` are a single, statically-registered client (there's no dynamic client registration or a client-management UI yet), so the connecting tool needs a way to be configured with that exact client ID and secret — check whether your client's "custom connector"/"advanced" setup exposes those fields. When you add the connector, point it at `https://mcp.example.com/mcp`; it will discover the rest from `/.well-known/oauth-authorization-server` and redirect through your Cardflow login.

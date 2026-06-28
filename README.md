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
      # Generate this from the Cardflow UI under Settings (profile/settings) after first boot
      CARDFLOW_TOKEN: <your-api-token>
      MCP_PORT: 3778
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

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
      # No TLS termination in this default setup — set to "true" once the
      # app is served over HTTPS so the refresh-token cookie requires it.
      COOKIE_SECURE: "false"
    volumes:
      - cardflow-data:/data
    ports:
      - "3001:3001"
    restart: unless-stopped

  cardflow-frontend:
    image: ghcr.io/brendenhoffman/cardflow-frontend:latest
    container_name: cardflow-frontend
    depends_on:
      - cardflow-backend
    ports:
      - "3000:80"
    restart: unless-stopped

volumes:
  cardflow-data:
```

Save that as `docker-compose.yml`, then:

```sh
docker compose up -d
```

Visit `http://localhost:3000` and follow the first-run setup screen to create your admin account. The JWT signing secret is generated automatically on first boot and persisted in the `cardflow-data` volume alongside the SQLite database, so both survive restarts and upgrades.

> Images are published by CI on every push to `main`. If they're still private on GHCR, run `docker login ghcr.io` before pulling, or flip the packages to public under the repo's package settings.

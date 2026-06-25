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

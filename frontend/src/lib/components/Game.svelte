<script lang="ts">
	import { ApiError, listDecks, createDeck, type Game, type Deck } from '$lib/api';
	import DeckComponent from './Deck.svelte';

	interface Props {
		game: Game;
	}

	let { game }: Props = $props();

	let decks: Deck[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let selectedDeckId: string | null = $state(null);
	let newDeckName = $state('');
	let creating = $state(false);

	async function loadDecks() {
		loading = true;
		error = null;
		try {
			decks = await listDecks(game.id);
			if (selectedDeckId && !decks.some((d) => d.id === selectedDeckId)) {
				selectedDeckId = null;
			}
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load decks';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void game.id;
		loadDecks();
	});

	async function handleCreateDeck(event: SubmitEvent) {
		event.preventDefault();
		const name = newDeckName.trim();
		if (!name) return;
		creating = true;
		error = null;
		try {
			const deck = await createDeck(game.id, { name });
			newDeckName = '';
			await loadDecks();
			selectedDeckId = deck.id;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to create deck';
		} finally {
			creating = false;
		}
	}

	const selectedDeck = $derived(decks.find((d) => d.id === selectedDeckId) ?? null);
</script>

<div class="game">
	<header>
		<h1>{game.name}</h1>
		<span class="status status-{game.status}">{game.status}</span>
	</header>
	{#if game.description}
		<p class="description">{game.description}</p>
	{/if}

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<div class="layout">
		<aside class="deck-list">
			<h2>Decks</h2>
			<form onsubmit={handleCreateDeck}>
				<input
					type="text"
					placeholder="New deck name"
					bind:value={newDeckName}
					disabled={creating}
				/>
				<button type="submit" disabled={creating || !newDeckName.trim()}>Add deck</button>
			</form>

			{#if loading}
				<p>Loading…</p>
			{:else if decks.length === 0}
				<p class="empty">No decks yet.</p>
			{:else}
				<ul>
					{#each decks as deck (deck.id)}
						<li>
							<button
								type="button"
								class="deck-item"
								class:selected={deck.id === selectedDeckId}
								class:archived={deck.status === 'archived'}
								onclick={() => (selectedDeckId = deck.id)}
							>
								{deck.name}
								<span class="deck-status">{deck.status}</span>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</aside>

		<main class="deck-view">
			{#if selectedDeck}
				{#key selectedDeck.id}
					<DeckComponent deck={selectedDeck} />
				{/key}
			{:else}
				<p class="empty">Select a deck to see its hand and pile.</p>
			{/if}
		</main>
	</div>
</div>

<style>
	.game {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	header {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	header h1 {
		margin: 0;
	}
	.status {
		font-size: 0.75rem;
		padding: 0.1rem 0.5rem;
		border-radius: 1rem;
		background: #eee;
		text-transform: uppercase;
	}
	.status-archived {
		opacity: 0.6;
	}
	.description {
		color: #555;
		margin: 0;
	}
	.error {
		color: #b91c1c;
	}
	.layout {
		display: grid;
		grid-template-columns: 240px 1fr;
		gap: 1.5rem;
		align-items: start;
	}
	.deck-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.deck-list form {
		display: flex;
		gap: 0.25rem;
	}
	.deck-list input {
		flex: 1;
		min-width: 0;
	}
	.deck-list ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.deck-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		text-align: left;
		background: transparent;
		border: 1px solid #ddd;
	}
	.deck-item.selected {
		border-color: #6366f1;
		background: #eef2ff;
	}
	.deck-item.archived {
		opacity: 0.6;
	}
	.deck-status {
		font-size: 0.7rem;
		color: #888;
		text-transform: uppercase;
	}
	.empty {
		color: #888;
		font-style: italic;
	}
</style>

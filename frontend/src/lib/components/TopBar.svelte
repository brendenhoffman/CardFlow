<script lang="ts">
	import type { Game, Deck } from '$lib/api';
	import type { SessionUser } from '$lib/stores';
	import EntitySelector from './EntitySelector.svelte';

	interface Props {
		games: Game[];
		selectedGameId: string | null;
		decks: Deck[];
		selectedDeckId: string | null;
		deckSelectorDisabled: boolean;
		user: SessionUser;
		onSelectGame: (id: string) => void;
		onCreateGame: (name: string) => Promise<string>;
		onSelectDeck: (id: string) => void;
		onCreateDeck: (name: string) => Promise<string>;
		onLogout: () => void;
	}

	let {
		games,
		selectedGameId,
		decks,
		selectedDeckId,
		deckSelectorDisabled,
		user,
		onSelectGame,
		onCreateGame,
		onSelectDeck,
		onCreateDeck,
		onLogout
	}: Props = $props();
</script>

<header class="top-bar">
	<div class="selectors">
		<span class="brand" aria-hidden="true">🂡</span>
		<EntitySelector
			label="Game"
			placeholder="Select a game…"
			items={games}
			selectedId={selectedGameId}
			onSelect={onSelectGame}
			onCreate={onCreateGame}
		/>
		<EntitySelector
			label="Deck"
			placeholder="Select a deck…"
			items={decks}
			selectedId={selectedDeckId}
			disabled={deckSelectorDisabled}
			onSelect={onSelectDeck}
			onCreate={onCreateDeck}
		/>
	</div>
	<div class="user-info">
		<span>{user.username ?? user.id} &middot; {user.role}</span>
		<a href="/settings" class="settings-link">Settings</a>
		<button type="button" class="logout" onclick={onLogout}>Log out</button>
	</div>
</header>

<style>
	.top-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.6rem 1.25rem;
		background: linear-gradient(180deg, #1f2a4d 0%, #161d38 100%);
		border-bottom: 3px solid #4338ca;
		color: #fff;
		flex-wrap: wrap;
	}
	.selectors {
		display: flex;
		align-items: center;
		gap: 1.25rem;
		flex-wrap: wrap;
	}
	.brand {
		font-size: 1.4rem;
		line-height: 1;
	}
	.user-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.8rem;
		color: #c7d2fe;
	}
	.logout {
		background: transparent;
		border: 1px solid #4338ca;
		color: #fff;
		font-size: 0.8rem;
	}
	.logout:hover {
		background: #312e81;
	}
	.settings-link {
		color: #c7d2fe;
		font-size: 0.8rem;
		text-decoration: none;
	}
	.settings-link:hover {
		color: #fff;
		text-decoration: underline;
	}
</style>

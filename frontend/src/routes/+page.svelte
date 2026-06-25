<script lang="ts">
	import {
		ApiError,
		listGames,
		createGame,
		listDecks,
		createDeck,
		type Game,
		type Deck
	} from '$lib/api';
	import { sessionUser, sessionReady, login, logout } from '$lib/stores';
	import TopBar from '$lib/components/TopBar.svelte';
	import DeckComponent from '$lib/components/Deck.svelte';

	let username = $state('');
	let password = $state('');
	let totpCode = $state('');
	let loginError: string | null = $state(null);
	let loggingIn = $state(false);

	let games: Game[] = $state([]);
	let gamesError: string | null = $state(null);
	let selectedGameId: string | null = $state(null);

	let decks: Deck[] = $state([]);
	let decksLoading = $state(false);
	let decksError: string | null = $state(null);
	let selectedDeckId: string | null = $state(null);

	const selectedDeck = $derived(decks.find((d) => d.id === selectedDeckId) ?? null);

	async function loadGames() {
		gamesError = null;
		try {
			games = await listGames();
		} catch (e) {
			gamesError = e instanceof ApiError ? e.message : 'Failed to load games';
		}
	}

	async function loadDecks(gameId: string) {
		decksLoading = true;
		decksError = null;
		try {
			decks = await listDecks(gameId);
			if (selectedDeckId && !decks.some((d) => d.id === selectedDeckId)) {
				selectedDeckId = null;
			}
		} catch (e) {
			decksError = e instanceof ApiError ? e.message : 'Failed to load decks';
		} finally {
			decksLoading = false;
		}
	}

	$effect(() => {
		if ($sessionUser) {
			loadGames();
		}
	});

	$effect(() => {
		if (selectedGameId) {
			loadDecks(selectedGameId);
		} else {
			decks = [];
			selectedDeckId = null;
		}
	});

	function handleSelectGame(id: string) {
		selectedGameId = id;
	}

	async function handleCreateGame(name: string): Promise<string> {
		const game = await createGame({ name });
		games = [...games, game];
		return game.id;
	}

	function handleSelectDeck(id: string) {
		selectedDeckId = id;
	}

	async function handleCreateDeck(name: string): Promise<string> {
		if (!selectedGameId) {
			throw new Error('Select a game first');
		}
		const deck = await createDeck(selectedGameId, { name });
		decks = [...decks, deck];
		return deck.id;
	}

	async function handleLogin(event: SubmitEvent) {
		event.preventDefault();
		loginError = null;
		loggingIn = true;
		try {
			await login({ username, password, totp_code: totpCode || undefined });
			password = '';
			totpCode = '';
		} catch (e) {
			loginError = e instanceof ApiError ? e.message : 'Login failed';
		} finally {
			loggingIn = false;
		}
	}
</script>

{#if !$sessionReady}
	<main class="centered">
		<p class="loading">Loading…</p>
	</main>
{:else if !$sessionUser}
	<main class="centered">
		<section class="login">
			<h1>CardFlow</h1>
			<form onsubmit={handleLogin}>
				<label>
					Username
					<input type="text" bind:value={username} required autocomplete="username" />
				</label>
				<label>
					Password
					<input
						type="password"
						bind:value={password}
						required
						autocomplete="current-password"
					/>
				</label>
				<label>
					TOTP code <span class="hint">(if MFA is enabled)</span>
					<input type="text" bind:value={totpCode} autocomplete="one-time-code" />
				</label>
				{#if loginError}
					<p class="error">{loginError}</p>
				{/if}
				<button type="submit" disabled={loggingIn}>Log in</button>
			</form>
		</section>
	</main>
{:else}
	<div class="app-shell">
		<TopBar
			{games}
			{selectedGameId}
			{decks}
			{selectedDeckId}
			deckSelectorDisabled={!selectedGameId}
			user={$sessionUser}
			onSelectGame={handleSelectGame}
			onCreateGame={handleCreateGame}
			onSelectDeck={handleSelectDeck}
			onCreateDeck={handleCreateDeck}
			onLogout={() => logout()}
		/>

		<main class="board-area">
			{#if gamesError}
				<p class="error">{gamesError}</p>
			{/if}
			{#if decksError}
				<p class="error">{decksError}</p>
			{/if}

			{#if selectedDeck}
				{#key selectedDeck.id}
					<DeckComponent deck={selectedDeck} />
				{/key}
			{:else if !selectedGameId}
				<p class="empty">Select or create a game above to get started.</p>
			{:else if decksLoading}
				<p class="empty">Loading decks…</p>
			{:else}
				<p class="empty">Select or create a deck above to see its hand and pile.</p>
			{/if}
		</main>
	</div>
{/if}

<style>
	.centered {
		max-width: 1100px;
		margin: 0 auto;
		padding: 1.5rem;
	}
	.loading {
		color: #888;
	}
	.login {
		max-width: 320px;
		margin: 4rem auto;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.login form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.login label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.875rem;
	}
	.hint {
		color: #888;
		font-weight: 400;
	}
	.error {
		color: #b91c1c;
		margin: 0;
	}
	.app-shell {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}
	.board-area {
		flex: 1;
		padding: 1.5rem;
	}
	.empty {
		color: #888;
		font-style: italic;
	}
</style>

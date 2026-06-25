<script lang="ts">
	import { ApiError, listGames, createGame, type Game } from '$lib/api';
	import { sessionUser, sessionReady, login, logout } from '$lib/stores';
	import GameComponent from '$lib/components/Game.svelte';

	let username = $state('');
	let password = $state('');
	let totpCode = $state('');
	let loginError: string | null = $state(null);
	let loggingIn = $state(false);

	let games: Game[] = $state([]);
	let gamesLoading = $state(true);
	let gamesError: string | null = $state(null);
	let selectedGameId: string | null = $state(null);
	let newGameName = $state('');
	let creatingGame = $state(false);

	const selectedGame = $derived(games.find((g) => g.id === selectedGameId) ?? null);

	async function loadGames() {
		gamesLoading = true;
		gamesError = null;
		try {
			games = await listGames();
		} catch (e) {
			gamesError = e instanceof ApiError ? e.message : 'Failed to load games';
		} finally {
			gamesLoading = false;
		}
	}

	$effect(() => {
		if ($sessionUser) {
			loadGames();
		}
	});

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

	async function handleCreateGame(event: SubmitEvent) {
		event.preventDefault();
		const name = newGameName.trim();
		if (!name) return;
		creatingGame = true;
		gamesError = null;
		try {
			const game = await createGame({ name });
			newGameName = '';
			await loadGames();
			selectedGameId = game.id;
		} catch (e) {
			gamesError = e instanceof ApiError ? e.message : 'Failed to create game';
		} finally {
			creatingGame = false;
		}
	}
</script>

<main>
	{#if !$sessionReady}
		<p class="loading">Loading…</p>
	{:else if !$sessionUser}
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
	{:else}
		<header class="app-header">
			<h1>CardFlow</h1>
			<div class="user-info">
				<span>{$sessionUser.username ?? $sessionUser.id} &middot; {$sessionUser.role}</span>
				<button type="button" onclick={() => logout()}>Log out</button>
			</div>
		</header>

		<div class="layout">
			<aside class="game-list">
				<h2>Games</h2>
				<form onsubmit={handleCreateGame}>
					<input
						type="text"
						placeholder="New game name"
						bind:value={newGameName}
						disabled={creatingGame}
					/>
					<button type="submit" disabled={creatingGame || !newGameName.trim()}>Add game</button>
				</form>

				{#if gamesError}
					<p class="error">{gamesError}</p>
				{/if}
				{#if gamesLoading}
					<p>Loading…</p>
				{:else if games.length === 0}
					<p class="empty">No games yet.</p>
				{:else}
					<ul>
						{#each games as game (game.id)}
							<li>
								<button
									type="button"
									class="game-item"
									class:selected={game.id === selectedGameId}
									class:archived={game.status === 'archived'}
									onclick={() => (selectedGameId = game.id)}
								>
									{game.name}
									<span class="game-status">{game.status}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</aside>

			<section class="game-view">
				{#if selectedGame}
					{#key selectedGame.id}
						<GameComponent game={selectedGame} />
					{/key}
				{:else}
					<p class="empty">Select a game to see its decks.</p>
				{/if}
			</section>
		</div>
	{/if}
</main>

<style>
	main {
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
	.app-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}
	.user-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.875rem;
	}
	.layout {
		display: grid;
		grid-template-columns: 240px 1fr;
		gap: 1.5rem;
		align-items: start;
	}
	.game-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.game-list form {
		display: flex;
		gap: 0.25rem;
	}
	.game-list input {
		flex: 1;
		min-width: 0;
	}
	.game-list ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.game-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		text-align: left;
		background: transparent;
		border: 1px solid #ddd;
		color: inherit;
	}
	.game-item.selected {
		border-color: #6366f1;
		background: #eef2ff;
	}
	.game-item.archived {
		opacity: 0.6;
	}
	.game-status {
		font-size: 0.7rem;
		color: #888;
		text-transform: uppercase;
	}
	.empty {
		color: #888;
		font-style: italic;
	}
</style>

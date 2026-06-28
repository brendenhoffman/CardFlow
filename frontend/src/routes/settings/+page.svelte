<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		ApiError,
		listApiTokens,
		createApiToken,
		deleteApiToken,
		type ApiTokenView
	} from '$lib/api';
	import { sessionUser, sessionReady } from '$lib/stores';
	import RevealTokenModal from '$lib/components/RevealTokenModal.svelte';

	let tokens: ApiTokenView[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);

	let newTokenName = $state('');
	let creating = $state(false);

	let revealing: { name: string; token: string } | null = $state(null);

	async function loadTokens() {
		loading = true;
		error = null;
		try {
			tokens = await listApiTokens();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load tokens';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if ($sessionReady && !$sessionUser) {
			goto('/');
		} else if ($sessionUser) {
			loadTokens();
		}
	});

	async function handleCreate(event: SubmitEvent) {
		event.preventDefault();
		const name = newTokenName.trim();
		if (!name) return;
		creating = true;
		error = null;
		try {
			const created = await createApiToken(name);
			newTokenName = '';
			revealing = { name: created.name, token: created.token };
			await loadTokens();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to create token';
		} finally {
			creating = false;
		}
	}

	async function handleRevoke(id: string, name: string) {
		if (!confirm(`Revoke the token "${name}"? Anything using it will stop working immediately.`)) {
			return;
		}
		error = null;
		try {
			await deleteApiToken(id);
			tokens = tokens.filter((t) => t.id !== id);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to revoke token';
		}
	}

	function formatDate(value: string | null): string {
		return value ? new Date(value).toLocaleString() : 'Never';
	}
</script>

<main>
	{#if !$sessionReady || !$sessionUser}
		<p class="loading">Loading…</p>
	{:else}
		<header>
			<a href="/" class="back-link">← Back to board</a>
			<h1>Settings</h1>
		</header>

		<section class="tokens-section">
			<h2>API tokens</h2>
			<p class="hint">
				Long-lived tokens for connecting external tools (like the MCP server) to your Cardflow
				account. Treat them like passwords — anyone with the token can act as you.
			</p>

			{#if error}
				<p class="error">{error}</p>
			{/if}

			<form class="create-form" onsubmit={handleCreate}>
				<input
					type="text"
					placeholder='Token name, e.g. "MCP server"'
					bind:value={newTokenName}
					disabled={creating}
				/>
				<button type="submit" disabled={creating || !newTokenName.trim()}>Create token</button>
			</form>

			{#if loading}
				<p>Loading…</p>
			{:else if tokens.length === 0}
				<p class="empty">No API tokens yet.</p>
			{:else}
				<table>
					<thead>
						<tr>
							<th>Name</th>
							<th>Created</th>
							<th>Last used</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each tokens as token (token.id)}
							<tr>
								<td>{token.name}</td>
								<td>{formatDate(token.created_at)}</td>
								<td>{formatDate(token.last_used_at)}</td>
								<td class="actions-cell">
									<button
										type="button"
										class="revoke"
										onclick={() => handleRevoke(token.id, token.name)}
									>
										Revoke
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</section>
	{/if}
</main>

{#if revealing}
	<RevealTokenModal name={revealing.name} token={revealing.token} onClose={() => (revealing = null)} />
{/if}

<style>
	main {
		max-width: 760px;
		margin: 0 auto;
		padding: 1.5rem;
	}
	.loading {
		color: #888;
	}
	header {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 2rem;
	}
	.back-link {
		color: #6366f1;
		font-size: 0.85rem;
		text-decoration: none;
	}
	.back-link:hover {
		text-decoration: underline;
	}
	header h1 {
		margin: 0;
	}
	.tokens-section h2 {
		margin: 0 0 0.25rem;
		font-size: 1.1rem;
	}
	.hint {
		margin: 0 0 1rem;
		color: #666;
		font-size: 0.85rem;
		max-width: 56ch;
	}
	.error {
		color: #b91c1c;
	}
	.create-form {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.25rem;
	}
	.create-form input {
		flex: 1;
		max-width: 320px;
	}
	.empty {
		color: #888;
		font-style: italic;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
	}
	th {
		text-align: left;
		font-weight: 600;
		color: #555;
		font-size: 0.78rem;
		text-transform: uppercase;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid #ddd;
	}
	td {
		padding: 0.6rem;
		border-bottom: 1px solid #eee;
	}
	.actions-cell {
		text-align: right;
	}
	.revoke {
		background: transparent;
		border: 1px solid #fca5a5;
		color: #b91c1c;
		font-size: 0.78rem;
	}
	.revoke:hover {
		background: #fef2f2;
	}
</style>

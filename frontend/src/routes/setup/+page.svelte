<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { ApiError, getSetupStatus, runSetup } from '$lib/api';
	import { bootstrapSession } from '$lib/stores';

	let checking = $state(true);
	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let error: string | null = $state(null);
	let submitting = $state(false);

	onMount(async () => {
		try {
			const status = await getSetupStatus();
			if (!status.required) {
				await goto('/');
				return;
			}
		} finally {
			checking = false;
		}
	});

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		error = null;

		if (password !== confirmPassword) {
			error = 'Passwords do not match.';
			return;
		}

		submitting = true;
		try {
			await runSetup({ username: username.trim(), password });
			// The root layout's setup-status check only runs once on initial load, so
			// this client-side navigation back to "/" wouldn't otherwise re-trigger it.
			await bootstrapSession();
			await goto('/');
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Setup failed';
		} finally {
			submitting = false;
		}
	}
</script>

<main>
	{#if checking}
		<p class="loading">Loading…</p>
	{:else}
		<section class="setup">
			<h1>Welcome to CardFlow</h1>
			<p class="subtitle">Create your admin account to get started.</p>
			<form onsubmit={handleSubmit}>
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
						minlength="8"
						autocomplete="new-password"
					/>
				</label>
				<label>
					Confirm password
					<input
						type="password"
						bind:value={confirmPassword}
						required
						minlength="8"
						autocomplete="new-password"
					/>
				</label>
				{#if error}
					<p class="error">{error}</p>
				{/if}
				<button type="submit" disabled={submitting}>Create admin account</button>
			</form>
		</section>
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
	.setup {
		max-width: 320px;
		margin: 4rem auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.setup h1 {
		margin: 0;
	}
	.subtitle {
		color: #555;
		margin: 0 0 0.5rem;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.875rem;
	}
	.error {
		color: #b91c1c;
		margin: 0;
	}
</style>

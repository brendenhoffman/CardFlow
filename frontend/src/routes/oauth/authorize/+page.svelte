<script lang="ts">
	import { page } from '$app/state';
	import { ApiError, completeOAuthAuthorize } from '$lib/api';
	import { sessionUser, sessionReady } from '$lib/stores';
	import LoginForm from '$lib/components/LoginForm.svelte';

	type Phase = 'checking' | 'login' | 'completing' | 'error';
	let phase: Phase = $state('checking');
	let completeError: string | null = $state(null);

	async function complete() {
		phase = 'completing';
		completeError = null;
		try {
			const result = await completeOAuthAuthorize(page.url.search);
			window.location.href = result.redirect_to;
		} catch (e) {
			completeError = e instanceof ApiError ? e.message : 'Failed to complete authorization';
			phase = 'error';
		}
	}

	$effect(() => {
		if (!$sessionReady) {
			phase = 'checking';
		} else if ($sessionUser) {
			void complete();
		} else {
			phase = 'login';
		}
	});
</script>

<main class="centered">
	<section class="login">
		<h1>CardFlow</h1>
		<p class="subtitle">Sign in to connect this application to your Cardflow account.</p>

		{#if phase === 'checking' || phase === 'completing'}
			<p class="loading">Loading…</p>
		{:else if phase === 'login'}
			<LoginForm onSuccess={complete} />
		{/if}

		{#if phase === 'error' && completeError}
			<p class="error">{completeError}</p>
		{/if}
	</section>
</main>

<style>
	.centered {
		max-width: 1100px;
		margin: 0 auto;
		padding: 1.5rem;
	}
	.login {
		max-width: 320px;
		margin: 4rem auto;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.subtitle {
		margin: 0;
		color: #555;
		font-size: 0.875rem;
	}
	.loading {
		color: #888;
	}
	.error {
		color: #b91c1c;
		margin: 0;
	}
</style>

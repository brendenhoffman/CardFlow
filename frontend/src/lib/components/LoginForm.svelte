<script lang="ts">
	import { ApiError } from '$lib/api';
	import { login } from '$lib/stores';

	interface Props {
		onSuccess: () => void;
	}

	let { onSuccess }: Props = $props();

	let username = $state('');
	let password = $state('');
	let totpCode = $state('');
	let loginError: string | null = $state(null);
	let loggingIn = $state(false);

	async function handleLogin(event: SubmitEvent) {
		event.preventDefault();
		loginError = null;
		loggingIn = true;
		try {
			await login({ username, password, totp_code: totpCode || undefined });
			password = '';
			totpCode = '';
			onSuccess();
		} catch (e) {
			loginError = e instanceof ApiError ? e.message : 'Login failed';
		} finally {
			loggingIn = false;
		}
	}
</script>

<form onsubmit={handleLogin}>
	<label>
		Username
		<input type="text" bind:value={username} required autocomplete="username" />
	</label>
	<label>
		Password
		<input type="password" bind:value={password} required autocomplete="current-password" />
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

<style>
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
	.hint {
		color: #888;
		font-weight: 400;
	}
	.error {
		color: #b91c1c;
		margin: 0;
	}
</style>

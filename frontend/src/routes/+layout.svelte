<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { getSetupStatus } from '$lib/api';
	import { bootstrapSession } from '$lib/stores';

	let { children } = $props();

	onMount(async () => {
		try {
			const status = await getSetupStatus();
			if (status.required) {
				if (page.url.pathname !== '/setup') {
					await goto('/setup');
				}
				return;
			}
		} catch (e) {
			// Status check failed (e.g. backend unreachable) — fall through to the
			// normal session bootstrap so the login screen still renders.
			console.error('setup status check failed', e);
		}
		bootstrapSession();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

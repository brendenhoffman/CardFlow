<script lang="ts">
	interface Props {
		name: string;
		token: string;
		onClose: () => void;
	}

	let { name, token, onClose }: Props = $props();

	let copied = $state(false);

	async function handleCopy() {
		try {
			await navigator.clipboard.writeText(token);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			// Clipboard API unavailable (e.g. insecure context) — the token is
			// still selectable/readable in the field below, so just no-op.
		}
	}

	function handleBackdropClick() {
		onClose();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="modal-backdrop"
	onclick={handleBackdropClick}
	onkeydown={handleKeydown}
	role="presentation"
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="modal-panel"
		role="dialog"
		aria-modal="true"
		aria-label="API token created"
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
	>
		<div class="modal-header">
			<h2>Token created: {name}</h2>
			<button type="button" class="icon-button" onclick={onClose} aria-label="Close">×</button>
		</div>

		<div class="modal-body">
			<p class="warning">
				Copy this token now — it will not be shown again. If you lose it, revoke it and create a
				new one.
			</p>
			<div class="token-row">
				<input type="text" readonly value={token} onclick={(e) => e.currentTarget.select()} />
				<button type="button" onclick={handleCopy}>{copied ? 'Copied!' : 'Copy'}</button>
			</div>
			<button type="button" class="secondary done-button" onclick={onClose}>Done</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(15, 18, 35, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		z-index: 100;
	}
	.modal-panel {
		background: #fff;
		border-radius: 0.75rem;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
		width: min(480px, 100%);
		display: flex;
		flex-direction: column;
	}
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.25rem;
		border-bottom: 1px solid #eee;
	}
	.modal-header h2 {
		margin: 0;
		font-size: 1.1rem;
	}
	.icon-button {
		background: transparent;
		color: #888;
		border: none;
		font-size: 1.25rem;
		line-height: 1;
		padding: 0.1rem 0.4rem;
	}
	.icon-button:hover {
		color: #1f2430;
	}
	.modal-body {
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	.warning {
		margin: 0;
		padding: 0.6rem 0.75rem;
		background: #fff7ed;
		border: 1px solid #fdba74;
		border-radius: 0.5rem;
		color: #9a3412;
		font-size: 0.85rem;
	}
	.token-row {
		display: flex;
		gap: 0.5rem;
	}
	.token-row input {
		flex: 1;
		min-width: 0;
		font-family: ui-monospace, monospace;
		font-size: 0.8rem;
	}
	.secondary {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
	.done-button {
		align-self: flex-end;
	}
</style>

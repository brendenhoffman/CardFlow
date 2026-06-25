<script lang="ts">
	import type { Card } from '$lib/api';

	interface Props {
		card: Card;
		onClose: () => void;
	}

	let { card, onClose }: Props = $props();

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
		aria-label={card.title}
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
	>
		<div class="modal-header">
			<h2>{card.title}</h2>
			<button type="button" class="icon-button" onclick={onClose} aria-label="Close">×</button>
		</div>

		<div class="modal-body">
			{#if card.description}
				<textarea class="description-area" readonly value={card.description}></textarea>
			{:else}
				<p class="empty">No description.</p>
			{/if}
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
		max-height: 90vh;
		overflow-y: auto;
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
	}
	.description-area {
		width: 100%;
		min-height: 260px;
		resize: vertical;
		font-family: inherit;
		font-size: 0.95rem;
		line-height: 1.5;
		color: #333;
		border: 1px solid #ddd;
		border-radius: 0.5rem;
		padding: 0.75rem;
		background: #fafafa;
	}
	.description-area:focus {
		outline: 2px solid #a5b4fc;
		outline-offset: 1px;
	}
	.empty {
		margin: 0;
		color: #888;
		font-style: italic;
	}
</style>

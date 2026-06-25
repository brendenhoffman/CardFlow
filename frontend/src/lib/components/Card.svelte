<script lang="ts">
	import type { Stack } from '$lib/api';
	import Card from './Card.svelte';

	interface Props {
		stack: Stack;
		/** Hand stacks can be dragged to reorder; pile items and nested jokers cannot. */
		draggable?: boolean;
		/** Show complete/return controls (hand) vs a draw control (pile). */
		context?: 'hand' | 'pile';
		isDragging?: boolean;
		isDragOver?: boolean;
		onComplete?: (cardId: string) => void;
		onReturn?: (cardId: string) => void;
		onDraw?: (cardId: string) => void;
		onDragStart?: () => void;
		onDragOverCard?: () => void;
		onDragLeaveCard?: () => void;
		onDropOnCard?: () => void;
		onDragEnd?: () => void;
	}

	let {
		stack,
		draggable = false,
		context = 'hand',
		isDragging = false,
		isDragOver = false,
		onComplete,
		onReturn,
		onDraw,
		onDragStart,
		onDragOverCard,
		onDragLeaveCard,
		onDropOnCard,
		onDragEnd
	}: Props = $props();

	let expanded = $state(false);

	const jokerCount = $derived(stack.jokers.length);
	// A card is blocked while any direct joker dependency isn't done yet.
	const isBlocked = $derived(stack.jokers.some((j) => j.card.status !== 'done'));
	const isRoot = $derived(stack.card.priority !== null);
	const canComplete = $derived(context === 'hand' && stack.card.status === 'hand' && !isBlocked);
	const canReturn = $derived(context === 'hand' && isRoot && stack.card.status === 'hand');

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		onDragOverCard?.();
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		onDropOnCard?.();
	}
</script>

<div
	class="card status-{stack.card.status}"
	class:dragging={isDragging}
	class:drag-over={isDragOver}
	draggable={draggable}
	ondragstart={() => onDragStart?.()}
	ondragover={draggable ? handleDragOver : undefined}
	ondragleave={() => onDragLeaveCard?.()}
	ondrop={draggable ? handleDrop : undefined}
	ondragend={() => onDragEnd?.()}
	role="group"
	aria-label={stack.card.title}
>
	<div class="card-header">
		{#if stack.card.priority !== null}
			<span class="priority">#{stack.card.priority}</span>
		{/if}
		<h3 class="title">{stack.card.title}</h3>
		{#if jokerCount > 0}
			<button
				type="button"
				class="joker-toggle"
				onclick={() => (expanded = !expanded)}
				aria-expanded={expanded}
			>
				{expanded ? '▾' : '▸'} {jokerCount} joker{jokerCount === 1 ? '' : 's'}
			</button>
		{/if}
	</div>

	{#if stack.card.description}
		<p class="description">{stack.card.description}</p>
	{/if}

	{#if stack.card.status === 'done' && stack.card.completed_at}
		<p class="completed-at">Completed {new Date(stack.card.completed_at).toLocaleString()}</p>
	{/if}

	<div class="actions">
		{#if context === 'hand'}
			<button type="button" disabled={!canComplete} onclick={() => onComplete?.(stack.card.id)}>
				Complete
			</button>
			{#if canReturn}
				<button type="button" class="secondary" onclick={() => onReturn?.(stack.card.id)}>
					Return to pile
				</button>
			{/if}
		{:else if context === 'pile'}
			<button type="button" onclick={() => onDraw?.(stack.card.id)}>Draw</button>
		{/if}
	</div>

	{#if expanded && jokerCount > 0}
		<div class="jokers">
			{#each stack.jokers as joker (joker.card.id)}
				<Card stack={joker} context="hand" {onComplete} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.card {
		border: 1px solid var(--card-border, #ccc);
		border-radius: 0.5rem;
		padding: 0.75rem;
		background: var(--card-bg, #fff);
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		transition:
			transform 0.15s ease,
			box-shadow 0.15s ease;
	}
	.card.dragging {
		opacity: 0.5;
	}
	.card.drag-over {
		box-shadow: 0 0 0 2px #6366f1;
	}
	.card.status-done {
		opacity: 0.7;
	}
	.card-header {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	.priority {
		font-weight: 700;
		color: #6366f1;
	}
	.title {
		flex: 1;
		margin: 0;
		font-size: 1rem;
	}
	.joker-toggle {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 0.8rem;
		color: #6366f1;
		padding: 0;
	}
	.description {
		margin: 0.25rem 0;
		font-size: 0.875rem;
		color: #555;
	}
	.completed-at {
		margin: 0.25rem 0;
		font-size: 0.75rem;
		color: #888;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}
	.actions button {
		font-size: 0.8rem;
	}
	.secondary {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
	.jokers {
		margin-top: 0.5rem;
		padding-left: 1rem;
		border-left: 2px dashed #ddd;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
</style>

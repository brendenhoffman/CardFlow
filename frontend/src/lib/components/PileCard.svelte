<script lang="ts">
	import type { Card as CardModel } from '$lib/api';

	interface Props {
		card: CardModel;
		isDragging?: boolean;
		onDraw?: (cardId: string) => void;
		onEdit?: (cardId: string) => void;
		onDragStart?: () => void;
		onDragEnd?: () => void;
	}

	let { card, isDragging = false, onDraw, onEdit, onDragStart, onDragEnd }: Props = $props();
</script>

<div
	class="pile-card"
	class:dragging={isDragging}
	draggable={true}
	ondragstart={() => onDragStart?.()}
	ondragend={() => onDragEnd?.()}
	role="group"
	aria-label={card.title}
>
	{#if onEdit}
		<button
			type="button"
			class="edit-button"
			onclick={() => onEdit?.(card.id)}
			aria-label="Edit card"
			title="Edit card"
		>
			✎
		</button>
	{/if}
	<h3 class="title">{card.title}</h3>
	{#if card.description}
		<button
			type="button"
			class="description"
			onclick={() => onEdit?.(card.id)}
			title="View full description"
		>
			{card.description}
		</button>
	{/if}
	<button type="button" class="draw-button" onclick={() => onDraw?.(card.id)}>Draw</button>
</div>

<style>
	.pile-card {
		position: relative;
		width: 140px;
		height: 140px;
		border: 1px solid #ddd;
		border-radius: 0.5rem;
		background: #fff;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		padding: 0.5rem 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		cursor: grab;
	}
	.pile-card.dragging {
		opacity: 0.5;
	}
	.title {
		margin: 0 1.1rem 0 0;
		font-size: 0.85rem;
		font-weight: 700;
		overflow: hidden;
		text-overflow: ellipsis;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
	}
	.description {
		flex: 1;
		background: none;
		border: none;
		padding: 0;
		margin: 0;
		font: inherit;
		font-size: 0.7rem;
		color: #666;
		text-align: left;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
	}
	.description:hover {
		color: #4338ca;
	}
	.edit-button {
		position: absolute;
		top: 0.3rem;
		right: 0.4rem;
		background: none;
		border: none;
		cursor: pointer;
		font-size: 0.8rem;
		color: #aaa;
		padding: 0.1rem 0.2rem;
		line-height: 1;
	}
	.edit-button:hover {
		color: #6366f1;
	}
	.draw-button {
		font-size: 0.72rem;
		padding: 0.25rem 0.5rem;
		align-self: flex-start;
	}
</style>

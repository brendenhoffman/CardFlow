<script lang="ts">
	import type { Stack } from '$lib/api';
	import { findStackNode, flattenStackPreorder, frontOfStack, isStackNodeBlocked } from '$lib/stack';

	interface Props {
		stack: Stack;
		draggable?: boolean;
		isDragging?: boolean;
		isDragOver?: boolean;
		onComplete?: (cardId: string) => void;
		onReturn?: (cardId: string) => void;
		onEdit?: (cardId: string) => void;
		onViewDescription?: (cardId: string) => void;
		onDragStart?: () => void;
		onDragOverCard?: () => void;
		onDragLeaveCard?: () => void;
		onDropOnCard?: () => void;
		onDragEnd?: () => void;
	}

	let {
		stack,
		draggable = false,
		isDragging = false,
		isDragOver = false,
		onComplete,
		onReturn,
		onEdit,
		onViewDescription,
		onDragStart,
		onDragOverCard,
		onDragLeaveCard,
		onDropOnCard,
		onDragEnd
	}: Props = $props();

	let pinnedViewId: string | null = $state(null);
	let showStackMenu = $state(false);
	let stackMenuEl: HTMLDivElement | undefined = $state();

	const allNodes = $derived(flattenStackPreorder(stack));
	const hasDepth = $derived(stack.jokers.length > 0);
	const front = $derived(
		(pinnedViewId ? findStackNode(stack, pinnedViewId) : null) ?? frontOfStack(stack)
	);
	const isJokerFront = $derived(front.card.id !== stack.card.id);
	const canComplete = $derived(front.card.status === 'hand' && !isStackNodeBlocked(front));
	const canReturn = $derived(stack.card.status === 'hand' && stack.card.priority !== null);

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		onDragOverCard?.();
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		onDropOnCard?.();
	}

	function toggleStackMenu() {
		showStackMenu = !showStackMenu;
		if (showStackMenu) {
			queueMicrotask(() => stackMenuEl?.focus());
		}
	}

	function selectView(cardId: string) {
		pinnedViewId = cardId;
		showStackMenu = false;
	}

	function handleMenuFocusOut(event: FocusEvent) {
		const container = event.currentTarget as HTMLElement;
		const next = event.relatedTarget as Node | null;
		if (next && container.contains(next)) return;
		showStackMenu = false;
	}
</script>

<div class="hand-card-wrapper">
	{#if hasDepth}
		<div class="stack-peek-shadow" aria-hidden="true"></div>
		<button
			type="button"
			class="stack-peek-hotspot"
			aria-label="View stack order"
			title="View stack order"
			onclick={toggleStackMenu}
		></button>
	{/if}

	<div
		class="hand-card"
		class:joker={isJokerFront}
		class:dragging={isDragging}
		class:drag-over={isDragOver}
		draggable={draggable}
		ondragstart={() => onDragStart?.()}
		ondragover={draggable ? handleDragOver : undefined}
		ondragleave={() => onDragLeaveCard?.()}
		ondrop={draggable ? handleDrop : undefined}
		ondragend={() => onDragEnd?.()}
		role="group"
		aria-label={front.card.title}
	>
		<div class="corner">
			{#if isJokerFront}
				<span class="joker-label">JOKER</span>
			{:else if stack.card.priority !== null}
				<span class="rank">{stack.card.priority}</span>
			{/if}
		</div>

		{#if onEdit}
			<button
				type="button"
				class="edit-button"
				onclick={() => onEdit?.(front.card.id)}
				aria-label="Edit card"
				title="Edit card"
			>
				✎
			</button>
		{/if}

		<div class="face">
			<h3 class="title">{front.card.title}</h3>
			{#if front.card.description}
				<button
					type="button"
					class="description"
					onclick={() => onViewDescription?.(front.card.id)}
					title="View full description"
				>
					{front.card.description}
				</button>
			{/if}
		</div>
	</div>

	{#if showStackMenu}
		<div
			class="stack-menu"
			bind:this={stackMenuEl}
			tabindex="-1"
			onfocusout={handleMenuFocusOut}
		>
			<h4>Stack order</h4>
			<ol>
				{#each allNodes as node, i (node.card.id)}
					<li class:current={node.card.id === front.card.id}>
						<div class="node-row">
							<span class="idx">{i + 1}</span>
							<span class="node-title">{node.card.title}</span>
							<span class="node-status status-{node.card.status}">{node.card.status}</span>
						</div>
						<div class="node-actions">
							<button type="button" class="view-btn" onclick={() => selectView(node.card.id)}>
								View
							</button>
							{#if node.card.status === 'hand' && !isStackNodeBlocked(node)}
								<button
									type="button"
									class="complete-btn"
									onclick={() => {
										showStackMenu = false;
										onComplete?.(node.card.id);
									}}
								>
									Complete
								</button>
							{/if}
						</div>
					</li>
				{/each}
			</ol>
		</div>
	{/if}
</div>

<div class="actions-row">
	<button type="button" disabled={!canComplete} onclick={() => onComplete?.(front.card.id)}>
		Complete
	</button>
	{#if canReturn}
		<button type="button" class="secondary" onclick={() => onReturn?.(stack.card.id)}>
			Return to pile
		</button>
	{/if}
</div>

<style>
	.hand-card-wrapper {
		position: relative;
		width: 194px;
		height: 266px;
	}
	/* Purely decorative — sits behind the front card, offset down-right so a sliver
	   peeks out to suggest depth. Never receives pointer events: the front card
	   covers most of its area, so a click here would silently land on the wrong
	   target. The peeking sliver itself is clickable via .stack-peek-hotspot,
	   which is positioned to exactly the uncovered strip below the front card. */
	.stack-peek-shadow {
		position: absolute;
		top: 14px;
		left: 14px;
		width: 180px;
		height: 252px;
		border-radius: 0.85rem;
		background: #e2e8f0;
		border: 1px solid #cbd5e1;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
		z-index: 0;
		pointer-events: none;
	}
	.stack-peek-hotspot {
		position: absolute;
		top: 252px;
		left: 14px;
		width: 180px;
		height: 14px;
		border-radius: 0 0 0.85rem 0.85rem;
		background: transparent;
		border: none;
		z-index: 2;
		cursor: pointer;
		padding: 0;
	}
	.hand-card {
		position: absolute;
		top: 0;
		left: 0;
		width: 180px;
		height: 252px;
		z-index: 1;
		border: 2px solid #a5b4fc;
		border-radius: 0.85rem;
		background: #fff;
		box-shadow: 0 3px 8px rgba(0, 0, 0, 0.18);
		transition:
			transform 0.15s ease,
			box-shadow 0.15s ease;
		display: flex;
		flex-direction: column;
	}
	.hand-card.joker {
		border-color: #9f1239;
		background: #fff1f2;
	}
	.hand-card.dragging {
		opacity: 0.5;
	}
	.hand-card.drag-over {
		box-shadow: 0 0 0 2px #6366f1;
	}
	.corner {
		position: absolute;
		top: 0.5rem;
		left: 0.5rem;
		bottom: 0.5rem;
		display: flex;
		align-items: flex-start;
	}
	.rank {
		font-size: 1.5rem;
		font-weight: 800;
		color: #4338ca;
		line-height: 1;
	}
	.joker-label {
		writing-mode: vertical-rl;
		text-orientation: mixed;
		font-weight: 700;
		font-size: 0.8rem;
		letter-spacing: 0.2em;
		color: #9f1239;
		height: 100%;
	}
	.edit-button {
		position: absolute;
		top: 0.4rem;
		right: 0.5rem;
		background: none;
		border: none;
		cursor: pointer;
		font-size: 0.9rem;
		color: #aaa;
		padding: 0.1rem 0.3rem;
		line-height: 1;
	}
	.edit-button:hover {
		color: #6366f1;
	}
	.face {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 0.4rem;
		padding: 1rem 1.75rem;
	}
	.title {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	.description {
		background: none;
		border: none;
		padding: 0;
		margin: 0;
		font: inherit;
		font-size: 0.78rem;
		color: #666;
		cursor: pointer;
		display: -webkit-box;
		-webkit-line-clamp: 4;
		line-clamp: 4;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.description:hover {
		color: #4338ca;
	}
	.actions-row {
		width: 180px;
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}
	.actions-row button {
		font-size: 0.8rem;
	}
	.secondary {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
	.stack-menu {
		position: absolute;
		top: 100%;
		left: 0;
		margin-top: 0.4rem;
		width: 300px;
		background: #fff;
		border: 1px solid #ddd;
		border-radius: 0.5rem;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
		padding: 0.6rem;
		z-index: 30;
	}
	.stack-menu h4 {
		margin: 0 0 0.4rem;
		font-size: 0.8rem;
		color: #555;
	}
	.stack-menu ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.stack-menu li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		font-size: 0.75rem;
		padding: 0.3rem 0.4rem;
		border-radius: 0.35rem;
	}
	.stack-menu li.current {
		background: #eef2ff;
	}
	.node-row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		flex: 1;
		min-width: 0;
	}
	.node-actions {
		display: flex;
		gap: 0.3rem;
		flex-shrink: 0;
	}
	.idx {
		font-weight: 700;
		color: #888;
		width: 1.1rem;
		flex-shrink: 0;
	}
	.node-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.node-status {
		font-size: 0.62rem;
		padding: 0.05rem 0.35rem;
		border-radius: 1rem;
		background: #e5e7eb;
		color: #555;
		text-transform: uppercase;
		flex-shrink: 0;
	}
	.view-btn,
	.complete-btn {
		font-size: 0.65rem;
		padding: 0.15rem 0.4rem;
	}
	.view-btn {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
</style>

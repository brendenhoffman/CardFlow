<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import {
		ApiError,
		dealHand,
		drawCard,
		completeCard,
		returnCard,
		reorderHand,
		listCards,
		getStack,
		type Deck,
		type Card as CardModel,
		type Stack
	} from '$lib/api';
	import Card from './Card.svelte';

	interface Props {
		deck: Deck;
	}

	let { deck }: Props = $props();

	let hand: Stack[] = $state([]);
	let pile: CardModel[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let showDrawPrompt = $state(false);

	let draggedCardId: string | null = $state(null);
	let dragOverCardId: string | null = $state(null);

	async function loadDeckState() {
		loading = true;
		error = null;
		try {
			const cards = await listCards(deck.id);
			const roots = cards
				.filter((c) => c.status === 'hand' && c.priority !== null)
				.sort((a, b) => (a.priority ?? 0) - (b.priority ?? 0));
			hand = await Promise.all(roots.map((root) => getStack(root.id)));
			pile = cards.filter((c) => c.status === 'pile');
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load deck';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void deck.id;
		loadDeckState();
	});

	async function handleDeal() {
		error = null;
		try {
			hand = await dealHand(deck.id);
			await loadDeckState();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to deal';
		}
	}

	async function handleDraw(cardId: string) {
		error = null;
		try {
			await drawCard(deck.id, cardId);
			await loadDeckState();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to draw card';
		}
	}

	async function handleComplete(cardId: string) {
		error = null;
		const wasRoot = hand.some((s) => s.card.id === cardId);
		try {
			await completeCard(cardId);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to complete card';
			return;
		}
		await loadDeckState();
		if (wasRoot && pile.length > 0) {
			showDrawPrompt = true;
		}
	}

	async function handleReturn(cardId: string) {
		error = null;
		try {
			await returnCard(cardId);
			await loadDeckState();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to return card';
		}
	}

	function handleDragStart(cardId: string) {
		draggedCardId = cardId;
	}

	function handleDragOverCard(cardId: string) {
		if (cardId !== draggedCardId) {
			dragOverCardId = cardId;
		}
	}

	function handleDragLeaveCard() {
		dragOverCardId = null;
	}

	async function handleDropOnCard(targetCardId: string) {
		dragOverCardId = null;
		const dragged = draggedCardId;
		draggedCardId = null;
		if (!dragged || dragged === targetCardId) {
			return;
		}

		const ids = hand.map((s) => s.card.id);
		const fromIndex = ids.indexOf(dragged);
		const toIndex = ids.indexOf(targetCardId);
		if (fromIndex === -1 || toIndex === -1) {
			return;
		}
		const [moved] = ids.splice(fromIndex, 1);
		ids.splice(toIndex, 0, moved);

		try {
			hand = await reorderHand(deck.id, ids);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to reorder hand';
			await loadDeckState();
		}
	}

	function handleDragEnd() {
		draggedCardId = null;
		dragOverCardId = null;
	}

	async function drawAfterComplete() {
		showDrawPrompt = false;
		await handleDeal();
	}
</script>

<div class="deck">
	<header>
		<h2>{deck.name}</h2>
		<span class="status status-{deck.status}">{deck.status}</span>
	</header>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<section class="hand-section">
		<div class="section-header">
			<h3>Hand ({hand.length}/5)</h3>
			<button type="button" onclick={handleDeal} disabled={hand.length >= 5 || pile.length === 0}>
				Deal
			</button>
		</div>

		{#if loading}
			<p>Loading…</p>
		{:else if hand.length === 0}
			<p class="empty">No cards in hand yet. Deal, or draw from the pile below.</p>
		{:else}
			<div class="hand">
				{#each hand as stack (stack.card.id)}
					<div class="hand-slot" out:fly={{ y: -30, duration: 250 }} animate:flip={{ duration: 200 }}>
						<Card
							{stack}
							draggable
							context="hand"
							isDragging={draggedCardId === stack.card.id}
							isDragOver={dragOverCardId === stack.card.id}
							onComplete={handleComplete}
							onReturn={handleReturn}
							onDragStart={() => handleDragStart(stack.card.id)}
							onDragOverCard={() => handleDragOverCard(stack.card.id)}
							onDragLeaveCard={handleDragLeaveCard}
							onDropOnCard={() => handleDropOnCard(stack.card.id)}
							onDragEnd={handleDragEnd}
						/>
					</div>
				{/each}
			</div>
		{/if}

		{#if showDrawPrompt}
			<div class="draw-prompt" transition:fly={{ y: -10, duration: 150 }}>
				<p>Card completed! Draw another?</p>
				<button type="button" onclick={drawAfterComplete}>Draw</button>
				<button type="button" class="secondary" onclick={() => (showDrawPrompt = false)}>
					Dismiss
				</button>
			</div>
		{/if}
	</section>

	<section class="pile-section">
		<h3>Pile ({pile.length})</h3>
		<div class="pile-list">
			{#each pile as card (card.id)}
				<Card stack={{ card, jokers: [] }} context="pile" onDraw={handleDraw} />
			{:else}
				<p class="empty">Pile is empty.</p>
			{/each}
		</div>
	</section>
</div>

<style>
	.deck {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	header {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	header h2 {
		margin: 0;
	}
	.status {
		font-size: 0.75rem;
		padding: 0.1rem 0.5rem;
		border-radius: 1rem;
		background: #eee;
		text-transform: uppercase;
	}
	.status-archived {
		opacity: 0.6;
	}
	.error {
		color: #b91c1c;
	}
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.hand {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 0.75rem;
	}
	.empty {
		color: #888;
		font-style: italic;
	}
	.draw-prompt {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: #eef2ff;
		border: 1px solid #c7d2fe;
		border-radius: 0.5rem;
		padding: 0.5rem 0.75rem;
	}
	.draw-prompt p {
		margin: 0;
		flex: 1;
	}
	.secondary {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
	.pile-section {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.pile-list {
		max-height: 320px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.25rem;
		border: 1px solid #eee;
		border-radius: 0.5rem;
	}
</style>

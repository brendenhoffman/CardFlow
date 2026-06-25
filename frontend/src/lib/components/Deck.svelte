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
	import { flattenStackPreorder } from '$lib/stack';
	import HandCard from './HandCard.svelte';
	import PileCard from './PileCard.svelte';
	import CardEditor from './CardEditor.svelte';

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
	let draggedFrom: 'hand' | 'pile' | null = $state(null);
	let dragOverCardId: string | null = $state(null);
	let handDropActive = $state(false);
	let pileDropActive = $state(false);

	let creatingCard = $state(false);
	let editingCard: CardModel | null = $state(null);

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

	function clearDragState() {
		draggedCardId = null;
		draggedFrom = null;
		dragOverCardId = null;
		handDropActive = false;
		pileDropActive = false;
	}

	function handleHandDragStart(cardId: string) {
		draggedCardId = cardId;
		draggedFrom = 'hand';
	}

	function handlePileDragStart(cardId: string) {
		draggedCardId = cardId;
		draggedFrom = 'pile';
	}

	function handleDragOverCard(cardId: string) {
		if (draggedFrom === 'hand' && cardId !== draggedCardId) {
			dragOverCardId = cardId;
		}
	}

	function handleDragLeaveCard() {
		dragOverCardId = null;
	}

	async function handleDropOnCard(targetCardId: string) {
		if (draggedFrom !== 'hand') return;
		dragOverCardId = null;
		const dragged = draggedCardId;
		draggedCardId = null;
		draggedFrom = null;
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
		clearDragState();
	}

	function handleHandAreaDragOver(event: DragEvent) {
		if (draggedFrom === 'pile') {
			event.preventDefault();
			handDropActive = true;
		}
	}

	function handleHandAreaDragLeave() {
		handDropActive = false;
	}

	async function handleHandAreaDrop(event: DragEvent) {
		if (draggedFrom !== 'pile' || !draggedCardId) return;
		event.preventDefault();
		const cardId = draggedCardId;
		clearDragState();
		await handleDraw(cardId);
	}

	function handlePileAreaDragOver(event: DragEvent) {
		if (draggedFrom === 'hand') {
			event.preventDefault();
			pileDropActive = true;
		}
	}

	function handlePileAreaDragLeave() {
		pileDropActive = false;
	}

	async function handlePileAreaDrop(event: DragEvent) {
		if (draggedFrom !== 'hand' || !draggedCardId) return;
		event.preventDefault();
		const cardId = draggedCardId;
		clearDragState();
		await handleReturn(cardId);
	}

	async function drawAfterComplete() {
		showDrawPrompt = false;
		await handleDeal();
	}

	function openEditor(cardId: string) {
		const found =
			hand
				.flatMap((s) => flattenStackPreorder(s))
				.map((s) => s.card)
				.find((c) => c.id === cardId) ?? pile.find((c) => c.id === cardId);
		if (found) {
			editingCard = found;
		}
	}

	function handleCardCreated() {
		void loadDeckState();
	}

	function handleCardUpdated() {
		void loadDeckState();
	}
</script>

<div class="deck">
	{#if error}
		<p class="error">{error}</p>
	{/if}

	<section class="hand-section">
		<div class="section-header">
			<h3>Hand <span class="count">{hand.length}/5</span></h3>
			<button type="button" onclick={handleDeal} disabled={hand.length >= 5 || pile.length === 0}>
				Deal
			</button>
		</div>

		<div
			class="hand"
			role="list"
			class:drop-active={handDropActive}
			ondragover={handleHandAreaDragOver}
			ondragleave={handleHandAreaDragLeave}
			ondrop={handleHandAreaDrop}
		>
			{#if loading}
				<p>Loading…</p>
			{:else if hand.length === 0}
				<p class="empty">No cards in hand yet. Deal, or drag a card here from the pile below.</p>
			{:else}
				{#each hand as stack (stack.card.id)}
					<div class="hand-slot" out:fly={{ y: -30, duration: 250 }} animate:flip={{ duration: 200 }}>
						<HandCard
							{stack}
							draggable
							isDragging={draggedFrom === 'hand' && draggedCardId === stack.card.id}
							isDragOver={dragOverCardId === stack.card.id}
							onComplete={handleComplete}
							onReturn={handleReturn}
							onEdit={openEditor}
							onDragStart={() => handleHandDragStart(stack.card.id)}
							onDragOverCard={() => handleDragOverCard(stack.card.id)}
							onDragLeaveCard={handleDragLeaveCard}
							onDropOnCard={() => handleDropOnCard(stack.card.id)}
							onDragEnd={handleDragEnd}
						/>
					</div>
				{/each}
			{/if}
		</div>

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
		<div class="section-header">
			<h3>Pile <span class="count">{pile.length}</span></h3>
			<button type="button" onclick={() => (creatingCard = true)}>+ New card</button>
		</div>
		<div
			class="pile-grid"
			role="list"
			class:drop-active={pileDropActive}
			ondragover={handlePileAreaDragOver}
			ondragleave={handlePileAreaDragLeave}
			ondrop={handlePileAreaDrop}
		>
			{#each pile as card (card.id)}
				<PileCard
					{card}
					isDragging={draggedFrom === 'pile' && draggedCardId === card.id}
					onDraw={handleDraw}
					onEdit={openEditor}
					onDragStart={() => handlePileDragStart(card.id)}
					onDragEnd={handleDragEnd}
				/>
			{:else}
				<p class="empty">Pile is empty. Add a card to get started.</p>
			{/each}
		</div>
	</section>
</div>

{#if creatingCard}
	<CardEditor mode="create" deckId={deck.id} onClose={() => (creatingCard = false)} onCreated={handleCardCreated} />
{/if}

{#if editingCard}
	{#key editingCard.id}
		<CardEditor
			mode="edit"
			deckId={deck.id}
			card={editingCard}
			onClose={() => (editingCard = null)}
			onUpdated={handleCardUpdated}
		/>
	{/key}
{/if}

<style>
	.deck {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}
	.error {
		color: #b91c1c;
	}
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}
	.section-header h3 {
		margin: 0;
		font-size: 1.15rem;
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	.count {
		font-size: 0.8rem;
		font-weight: 400;
		color: #888;
	}
	.pile-section {
		display: flex;
		flex-direction: column;
	}
	.hand {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 1.25rem;
		width: fit-content;
		max-width: 100%;
		min-height: 280px;
		padding: 0.75rem;
		border-radius: 0.75rem;
		border: 2px dashed transparent;
		transition: border-color 0.15s ease, background-color 0.15s ease;
	}
	.hand.drop-active {
		border-color: #6366f1;
		background: #eef2ff;
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
		margin-top: 0.75rem;
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
	.pile-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 0.85rem;
		align-content: start;
		min-height: 100px;
		padding: 0.75rem;
		border-radius: 0.75rem;
		border: 2px dashed transparent;
		transition: border-color 0.15s ease, background-color 0.15s ease;
	}
	.pile-grid.drop-active {
		border-color: #6366f1;
		background: #eef2ff;
	}
</style>

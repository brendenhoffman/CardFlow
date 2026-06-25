<script lang="ts">
	import {
		ApiError,
		createCard,
		updateCard,
		listCards,
		listJokers,
		listDeckJokers,
		addJoker,
		removeJoker,
		type Card,
		type CardJoker
	} from '$lib/api';

	interface Props {
		mode: 'create' | 'edit';
		deckId: string;
		card?: Card;
		onClose: () => void;
		onCreated?: (card: Card) => void;
		onUpdated?: (card: Card) => void;
	}

	let { mode, deckId, card, onClose, onCreated, onUpdated }: Props = $props();

	let title = $state(card?.title ?? '');
	let description = $state(card?.description ?? '');
	let saving = $state(false);
	let saveError: string | null = $state(null);
	let titleInput: HTMLInputElement | undefined = $state();

	let allCards: Card[] = $state([]);
	let assignedJokers: Card[] = $state([]);
	let edges: CardJoker[] = $state([]);
	let jokersLoading = $state(mode === 'edit');
	let jokerError: string | null = $state(null);
	let search = $state('');

	/** IDs reachable from `startId` by following `card_id -> joker_id` edges in `direction`. */
	function reachableIds(startId: string, direction: 'forward' | 'backward'): Set<string> {
		const adjacency = new Map<string, string[]>();
		for (const edge of edges) {
			const [from, to] = direction === 'forward' ? [edge.card_id, edge.joker_id] : [edge.joker_id, edge.card_id];
			const list = adjacency.get(from) ?? [];
			list.push(to);
			adjacency.set(from, list);
		}
		const visited = new Set<string>();
		const queue = [...(adjacency.get(startId) ?? [])];
		while (queue.length > 0) {
			const next = queue.shift()!;
			if (visited.has(next)) continue;
			visited.add(next);
			queue.push(...(adjacency.get(next) ?? []));
		}
		return visited;
	}

	// Cards this card already depends on (directly or transitively) — adding one of
	// these again as a direct joker would be redundant, since it's already "below"
	// this card in the tree.
	const descendantIds = $derived(card ? reachableIds(card.id, 'forward') : new Set<string>());
	// Cards that already depend on this card (directly or transitively) — adding one
	// of these as a joker would create a dependency cycle, since it's already
	// "above" this card in the tree.
	const ancestorIds = $derived(card ? reachableIds(card.id, 'backward') : new Set<string>());

	const availableCards = $derived(
		allCards.filter((c) => {
			if (card && c.id === card.id) return false;
			if (descendantIds.has(c.id)) return false;
			if (ancestorIds.has(c.id)) return false;
			const q = search.trim().toLowerCase();
			return q === '' || c.title.toLowerCase().includes(q);
		})
	);

	async function loadJokerData() {
		if (mode !== 'edit' || !card) return;
		jokersLoading = true;
		jokerError = null;
		try {
			const [cards, jokers, deckEdges] = await Promise.all([
				listCards(deckId),
				listJokers(card.id),
				listDeckJokers(deckId)
			]);
			allCards = cards;
			assignedJokers = jokers;
			edges = deckEdges;
		} catch (e) {
			jokerError = e instanceof ApiError ? e.message : 'Failed to load jokers';
		} finally {
			jokersLoading = false;
		}
	}

	$effect(() => {
		loadJokerData();
	});

	$effect(() => {
		titleInput?.focus();
	});

	async function handleSave() {
		const trimmed = title.trim();
		if (!trimmed) return;
		saving = true;
		saveError = null;
		try {
			if (mode === 'create') {
				const created = await createCard(deckId, {
					title: trimmed,
					description: description.trim() || undefined
				});
				onCreated?.(created);
				onClose();
			} else if (card) {
				const updated = await updateCard(card.id, {
					title: trimmed,
					description: description.trim() || undefined
				});
				onUpdated?.(updated);
			}
		} catch (e) {
			saveError = e instanceof ApiError ? e.message : 'Failed to save card';
		} finally {
			saving = false;
		}
	}

	async function handleAddJoker(candidateId: string) {
		if (!card) return;
		jokerError = null;
		try {
			await addJoker(card.id, candidateId);
			const [jokers, deckEdges] = await Promise.all([listJokers(card.id), listDeckJokers(deckId)]);
			assignedJokers = jokers;
			edges = deckEdges;
			search = '';
		} catch (e) {
			jokerError = e instanceof ApiError ? e.message : 'Failed to add joker';
		}
	}

	async function handleRemoveJoker(jokerId: string) {
		if (!card) return;
		jokerError = null;
		try {
			await removeJoker(card.id, jokerId);
			assignedJokers = assignedJokers.filter((j) => j.id !== jokerId);
			edges = await listDeckJokers(deckId);
		} catch (e) {
			jokerError = e instanceof ApiError ? e.message : 'Failed to remove joker';
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
		aria-label={mode === 'create' ? 'New card' : 'Edit card'}
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
	>
		<div class="modal-header">
			<h2>{mode === 'create' ? 'New card' : 'Edit card'}</h2>
			<button type="button" class="icon-button" onclick={onClose} aria-label="Close">×</button>
		</div>

		<div class="modal-body">
			<label class="field">
				<span>Title</span>
				<input bind:this={titleInput} type="text" bind:value={title} placeholder="Card title" />
			</label>
			<label class="field">
				<span>Description</span>
				<textarea bind:value={description} rows="3" placeholder="Card description"></textarea>
			</label>

			{#if saveError}
				<p class="error">{saveError}</p>
			{/if}

			<div class="modal-actions">
				<button type="button" onclick={handleSave} disabled={saving || !title.trim()}>
					{mode === 'create' ? 'Create card' : 'Save changes'}
				</button>
				<button type="button" class="secondary" onclick={onClose}>
					{mode === 'create' ? 'Cancel' : 'Close'}
				</button>
			</div>

			{#if mode === 'edit' && card}
				<section class="jokers-section">
					<h3>Jokers</h3>
					<p class="hint">
						Other cards in this deck that must be completed before this one unblocks.
					</p>

					{#if jokersLoading}
						<p>Loading…</p>
					{:else}
						{#if assignedJokers.length > 0}
							<ol class="assigned-list">
								{#each assignedJokers as joker, i (joker.id)}
									<li class="assigned-item">
										<span class="order">{i + 1}</span>
										<span class="title">{joker.title}</span>
										<span class="status status-{joker.status}">{joker.status}</span>
										<button
											type="button"
											class="remove"
											onclick={() => handleRemoveJoker(joker.id)}
										>
											Remove
										</button>
									</li>
								{/each}
							</ol>
						{:else}
							<p class="empty">No jokers assigned yet.</p>
						{/if}

						<input
							type="text"
							class="joker-search"
							placeholder="Search cards to add as a joker…"
							bind:value={search}
						/>
						<ul class="available-list">
							{#each availableCards as candidate (candidate.id)}
								<li class="available-item">
									<span class="title">{candidate.title}</span>
									<span class="status status-{candidate.status}">{candidate.status}</span>
									<button type="button" onclick={() => handleAddJoker(candidate.id)}>+ Add</button>
								</li>
							{:else}
								<li class="empty">No matching cards.</li>
							{/each}
						</ul>
					{/if}

					{#if jokerError}
						<p class="error">{jokerError}</p>
					{/if}
				</section>
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
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.85rem;
		font-weight: 600;
		color: #444;
	}
	.field input,
	.field textarea {
		font-family: inherit;
		font-weight: 400;
		resize: vertical;
	}
	.error {
		color: #b91c1c;
		margin: 0;
		font-size: 0.85rem;
	}
	.modal-actions {
		display: flex;
		gap: 0.5rem;
	}
	.secondary {
		background: transparent;
		border: 1px solid #ccc;
		color: #333;
	}
	.jokers-section {
		margin-top: 0.5rem;
		padding-top: 1rem;
		border-top: 1px solid #eee;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.jokers-section h3 {
		margin: 0;
		font-size: 0.95rem;
	}
	.hint {
		margin: 0;
		font-size: 0.78rem;
		color: #888;
	}
	.empty {
		margin: 0;
		font-size: 0.82rem;
		color: #888;
		font-style: italic;
	}
	.assigned-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.assigned-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: #eef2ff;
		border: 1px solid #6366f1;
		border-radius: 0.5rem;
		padding: 0.35rem 0.6rem;
	}
	.assigned-item .order {
		width: 1.4rem;
		height: 1.4rem;
		flex-shrink: 0;
		border-radius: 50%;
		background: #6366f1;
		color: #fff;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 700;
	}
	.assigned-item .title {
		flex: 1;
		font-size: 0.85rem;
		font-weight: 600;
	}
	.assigned-item .remove {
		background: transparent;
		border: 1px solid #fca5a5;
		color: #b91c1c;
		font-size: 0.72rem;
		padding: 0.2rem 0.5rem;
	}
	.joker-search {
		width: 100%;
	}
	.available-list {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 160px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		border: 1px solid #eee;
		border-radius: 0.5rem;
	}
	.available-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid #f1f1f1;
		font-size: 0.82rem;
		color: #444;
	}
	.available-item:last-child {
		border-bottom: none;
	}
	.available-item .title {
		flex: 1;
	}
	.available-item button {
		font-size: 0.72rem;
		padding: 0.2rem 0.5rem;
	}
	.status {
		font-size: 0.65rem;
		padding: 0.05rem 0.4rem;
		border-radius: 1rem;
		background: #e5e7eb;
		color: #555;
		text-transform: uppercase;
	}
	.available-list .empty {
		padding: 0.5rem 0.6rem;
	}
</style>

<script lang="ts">
	interface Item {
		id: string;
		name: string;
		status?: string;
	}

	interface Props {
		items: Item[];
		selectedId: string | null;
		label: string;
		placeholder: string;
		addLabel?: string;
		disabled?: boolean;
		onSelect: (id: string) => void;
		/** Create the item, persist it wherever `items` lives, and resolve with its id. */
		onCreate: (name: string) => Promise<string>;
	}

	let {
		items,
		selectedId,
		label,
		placeholder,
		addLabel = '+ Add new',
		disabled = false,
		onSelect,
		onCreate
	}: Props = $props();

	const ADD_NEW = '__add_new__';

	let mode: 'select' | 'adding' = $state('select');
	let draftName = $state('');
	let creating = $state(false);
	let createError: string | null = $state(null);
	let inputEl: HTMLInputElement | undefined = $state();

	function handleChange(event: Event) {
		const value = (event.target as HTMLSelectElement).value;
		if (value === ADD_NEW) {
			startAdding();
			return;
		}
		onSelect(value);
	}

	function startAdding() {
		mode = 'adding';
		draftName = '';
		createError = null;
		queueMicrotask(() => inputEl?.focus());
	}

	function cancelAdding() {
		mode = 'select';
		draftName = '';
		createError = null;
	}

	async function confirmAdding() {
		const name = draftName.trim();
		if (!name) {
			cancelAdding();
			return;
		}
		creating = true;
		createError = null;
		try {
			const id = await onCreate(name);
			mode = 'select';
			draftName = '';
			onSelect(id);
		} catch (e) {
			createError = e instanceof Error ? e.message : 'Failed to create';
		} finally {
			creating = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			confirmAdding();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			cancelAdding();
		}
	}

	function handleFocusOut(event: FocusEvent) {
		const container = event.currentTarget as HTMLElement;
		const next = event.relatedTarget as Node | null;
		if (next && container.contains(next)) {
			return;
		}
		if (mode === 'adding' && !creating) {
			cancelAdding();
		}
	}
</script>

<div class="entity-selector" class:adding={mode === 'adding'} onfocusout={handleFocusOut}>
	<span class="label">{label}</span>
	{#if mode === 'select'}
		<select value={selectedId ?? ''} onchange={handleChange} {disabled}>
			{#if !selectedId}
				<option value="" disabled hidden>{placeholder}</option>
			{/if}
			{#each items as item (item.id)}
				<option value={item.id}>
					{item.name}{item.status === 'archived' ? ' (archived)' : ''}
				</option>
			{/each}
			<option value={ADD_NEW}>{addLabel}</option>
		</select>
	{:else}
		<div class="add-inline">
			<input
				bind:this={inputEl}
				type="text"
				bind:value={draftName}
				onkeydown={handleKeydown}
				placeholder={label}
				disabled={creating}
			/>
			<button type="button" onclick={confirmAdding} disabled={creating || !draftName.trim()}>
				✓
			</button>
		</div>
		{#if createError}
			<span class="add-error">{createError}</span>
		{/if}
	{/if}
</div>

<style>
	.entity-selector {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		position: relative;
	}
	.label {
		font-size: 0.7rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #818cf8;
		white-space: nowrap;
	}
	select {
		appearance: none;
		border: 1px solid #4338ca;
		border-radius: 0.5rem;
		padding: 0.4rem 1.75rem 0.4rem 0.65rem;
		font-size: 0.9rem;
		font-weight: 600;
		background: #fff
			url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%234338ca'/%3E%3C/svg%3E")
			no-repeat right 0.65rem center;
		color: #1f2430;
		min-width: 150px;
		cursor: pointer;
	}
	select:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.add-inline {
		display: flex;
		gap: 0.25rem;
	}
	.add-inline input {
		min-width: 150px;
		border-color: #4338ca;
	}
	.add-inline button {
		padding: 0.4rem 0.6rem;
	}
	.add-error {
		position: absolute;
		top: 100%;
		left: 0;
		font-size: 0.7rem;
		color: #fca5a5;
		background: #1f2430;
		padding: 0.15rem 0.4rem;
		border-radius: 0.25rem;
		margin-top: 0.2rem;
		white-space: nowrap;
		z-index: 5;
	}
</style>

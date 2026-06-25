import type { Stack } from './api';

/** The deepest unresolved joker dependency, or `stack` itself if nothing blocks it. */
export function frontOfStack(stack: Stack): Stack {
	const blocking = stack.jokers.find((j) => j.card.status !== 'done');
	return blocking ? frontOfStack(blocking) : stack;
}

/** Every node in the tree, root first, depth-first. */
export function flattenStackPreorder(stack: Stack): Stack[] {
	return [stack, ...stack.jokers.flatMap(flattenStackPreorder)];
}

/** Whether any direct joker dependency of this node isn't done yet. */
export function isStackNodeBlocked(node: Stack): boolean {
	return node.jokers.some((j) => j.card.status !== 'done');
}

export function findStackNode(stack: Stack, cardId: string): Stack | null {
	if (stack.card.id === cardId) return stack;
	for (const joker of stack.jokers) {
		const found = findStackNode(joker, cardId);
		if (found) return found;
	}
	return null;
}

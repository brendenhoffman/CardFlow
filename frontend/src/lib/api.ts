// Typed fetch wrappers for every CardFlow backend endpoint.
// All paths are relative to /api, which Vite (dev) and nginx (prod) proxy to the backend.

const API_BASE = '/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Game {
	id: string;
	name: string;
	description: string | null;
	status: string;
	created_at: string;
}

export interface CreateGame {
	name: string;
	description?: string;
}

export interface UpdateGame {
	name?: string;
	description?: string;
	status?: string;
}

export interface Deck {
	id: string;
	game_id: string;
	name: string;
	description: string | null;
	status: string;
	created_at: string;
}

export interface CreateDeck {
	name: string;
	description?: string;
}

export interface UpdateDeck {
	name?: string;
	description?: string;
	status?: string;
}

export type CardStatus = 'pile' | 'hand' | 'done';

export interface Card {
	id: string;
	deck_id: string;
	title: string;
	description: string | null;
	status: CardStatus;
	priority: number | null;
	created_at: string;
	completed_at: string | null;
}

export interface CreateCard {
	title: string;
	description?: string;
}

export interface UpdateCard {
	title?: string;
	description?: string;
	status?: CardStatus;
	priority?: number;
}

/** A card together with its joker dependency subtree, nested in dependency order. */
export interface Stack {
	card: Card;
	jokers: Stack[];
}

export interface CardJoker {
	id: string;
	card_id: string;
	joker_id: string;
	order: number;
}

export interface CompleteResult {
	card: Card;
	unblocked: Card[];
}

export type UserRole = 'admin' | 'user';

export interface UserView {
	id: string;
	username: string;
	role: UserRole;
	mfa_enabled: boolean;
	created_at: string;
}

export interface CreateUser {
	username: string;
	password: string;
	role?: UserRole;
}

export interface UpdateUser {
	username?: string;
	password?: string;
	role?: UserRole;
}

export interface ApiTokenView {
	id: string;
	name: string;
	created_at: string;
	last_used_at: string | null;
}

export interface CreateApiTokenResponse {
	id: string;
	name: string;
	token: string;
	created_at: string;
}

export interface LoginRequest {
	username: string;
	password: string;
	totp_code?: string;
}

export interface SessionResponse {
	access_token: string;
	token_type: string;
	expires_in: number;
}

export interface MfaSetupResponse {
	secret: string;
	otpauth_url: string;
}

export interface MfaVerifyRequest {
	secret: string;
	code: string;
}

export interface SetupStatus {
	required: boolean;
}

export interface SetupRequest {
	username: string;
	password: string;
}

// ---------------------------------------------------------------------------
// Core fetch plumbing: auth header injection, JSON (de)serialization,
// {error} body surfacing, and one automatic refresh-and-retry on a 401.
// ---------------------------------------------------------------------------

export class ApiError extends Error {
	status: number;
	constructor(status: number, message: string) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

let accessToken: string | null = null;

export function getAccessToken(): string | null {
	return accessToken;
}

export function setAccessToken(token: string | null): void {
	accessToken = token;
}

function withBody(method: string, data?: unknown): RequestInit {
	return { method, body: data === undefined ? undefined : JSON.stringify(data) };
}

async function rawFetch(path: string, init: RequestInit = {}): Promise<Response> {
	const headers = new Headers(init.headers);
	if (init.body && !headers.has('Content-Type')) {
		headers.set('Content-Type', 'application/json');
	}
	if (accessToken) {
		headers.set('Authorization', `Bearer ${accessToken}`);
	}
	// credentials: 'include' so the HttpOnly refresh_token cookie is sent to /auth/*.
	return fetch(`${API_BASE}${path}`, { ...init, headers, credentials: 'include' });
}

async function parseBody<T>(res: Response): Promise<T> {
	if (res.status === 204) {
		return undefined as T;
	}
	const text = await res.text();
	return (text ? JSON.parse(text) : undefined) as T;
}

/** Dispatched when a request comes back 401 even after a refresh attempt. */
export const SESSION_EXPIRED_EVENT = 'cardflow:session-expired';

/** Dispatched whenever the access token is rotated, with the new token as detail. */
export const TOKEN_REFRESHED_EVENT = 'cardflow:token-refreshed';

async function throwApiError(res: Response): Promise<never> {
	let message = res.statusText || `request failed with status ${res.status}`;
	try {
		const body = await res.json();
		if (body && typeof body.error === 'string') {
			message = body.error;
		}
	} catch {
		// no JSON body to read; fall back to statusText
	}
	if (res.status === 401 && typeof window !== 'undefined') {
		window.dispatchEvent(new Event(SESSION_EXPIRED_EVENT));
	}
	throw new ApiError(res.status, message);
}

let refreshPromise: Promise<boolean> | null = null;

/**
 * De-dupes concurrent refresh attempts so a token rotation never races itself —
 * the refresh token is rotated server-side on every call, so two in-flight
 * refreshes would have the second fail against an already-revoked cookie.
 * Shared by the reactive (401-triggered) and proactive (pre-expiry timer) paths.
 */
export function tryRefresh(): Promise<boolean> {
	if (!refreshPromise) {
		refreshPromise = (async () => {
			try {
				const res = await rawFetch('/auth/refresh', { method: 'POST' });
				if (!res.ok) {
					if (res.status === 401 && typeof window !== 'undefined') {
						window.dispatchEvent(new Event(SESSION_EXPIRED_EVENT));
					}
					return false;
				}
				const data = await parseBody<SessionResponse>(res);
				setAccessToken(data.access_token);
				if (typeof window !== 'undefined') {
					window.dispatchEvent(new CustomEvent(TOKEN_REFRESHED_EVENT, { detail: data.access_token }));
				}
				return true;
			} catch {
				return false;
			} finally {
				refreshPromise = null;
			}
		})();
	}
	return refreshPromise;
}

async function apiFetch<T>(path: string, init: RequestInit = {}, isRetry = false): Promise<T> {
	const res = await rawFetch(path, init);
	if (res.ok) {
		return parseBody<T>(res);
	}
	const canRetry = !isRetry && res.status === 401 && path !== '/auth/login' && path !== '/auth/refresh';
	if (canRetry && (await tryRefresh())) {
		return apiFetch<T>(path, init, true);
	}
	return throwApiError(res);
}

// ---------------------------------------------------------------------------
// First-run setup
// ---------------------------------------------------------------------------

export function getSetupStatus(): Promise<SetupStatus> {
	return apiFetch('/setup/status');
}

export function runSetup(payload: SetupRequest): Promise<UserView> {
	return apiFetch('/setup', withBody('POST', payload));
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export function login(payload: LoginRequest): Promise<SessionResponse> {
	return apiFetch('/auth/login', withBody('POST', payload));
}

export function refreshSession(): Promise<SessionResponse> {
	return apiFetch('/auth/refresh', { method: 'POST' });
}

export function logout(): Promise<void> {
	return apiFetch('/auth/logout', { method: 'POST' });
}

export function mfaSetup(): Promise<MfaSetupResponse> {
	return apiFetch('/auth/mfa/setup', { method: 'POST' });
}

export function mfaVerify(payload: MfaVerifyRequest): Promise<void> {
	return apiFetch('/auth/mfa/verify', withBody('POST', payload));
}

// ---------------------------------------------------------------------------
// Users (admin only)
// ---------------------------------------------------------------------------

export function listUsers(): Promise<UserView[]> {
	return apiFetch('/users');
}

export function createUser(payload: CreateUser): Promise<UserView> {
	return apiFetch('/users', withBody('POST', payload));
}

export function updateUser(id: string, payload: UpdateUser): Promise<UserView> {
	return apiFetch(`/users/${encodeURIComponent(id)}`, withBody('PATCH', payload));
}

export function deleteUser(id: string): Promise<void> {
	return apiFetch(`/users/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

// ---------------------------------------------------------------------------
// Games
// ---------------------------------------------------------------------------

export function listGames(): Promise<Game[]> {
	return apiFetch('/games');
}

export function getGame(id: string): Promise<Game> {
	return apiFetch(`/games/${encodeURIComponent(id)}`);
}

export function createGame(payload: CreateGame): Promise<Game> {
	return apiFetch('/games', withBody('POST', payload));
}

export function updateGame(id: string, payload: UpdateGame): Promise<Game> {
	return apiFetch(`/games/${encodeURIComponent(id)}`, withBody('PATCH', payload));
}

export function deleteGame(id: string): Promise<void> {
	return apiFetch(`/games/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

export function listDecks(gameId: string): Promise<Deck[]> {
	return apiFetch(`/games/${encodeURIComponent(gameId)}/decks`);
}

export function getDeck(id: string): Promise<Deck> {
	return apiFetch(`/decks/${encodeURIComponent(id)}`);
}

export function createDeck(gameId: string, payload: CreateDeck): Promise<Deck> {
	return apiFetch(`/games/${encodeURIComponent(gameId)}/decks`, withBody('POST', payload));
}

export function updateDeck(id: string, payload: UpdateDeck): Promise<Deck> {
	return apiFetch(`/decks/${encodeURIComponent(id)}`, withBody('PATCH', payload));
}

export function deleteDeck(id: string): Promise<void> {
	return apiFetch(`/decks/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

export function listCards(deckId: string): Promise<Card[]> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/cards`);
}

export function getCard(id: string): Promise<Card> {
	return apiFetch(`/cards/${encodeURIComponent(id)}`);
}

export function createCard(deckId: string, payload: CreateCard): Promise<Card> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/cards`, withBody('POST', payload));
}

export function updateCard(id: string, payload: UpdateCard): Promise<Card> {
	return apiFetch(`/cards/${encodeURIComponent(id)}`, withBody('PATCH', payload));
}

export function deleteCard(id: string): Promise<void> {
	return apiFetch(`/cards/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

// ---------------------------------------------------------------------------
// Jokers (card dependency tree)
// ---------------------------------------------------------------------------

export function listJokers(cardId: string): Promise<Card[]> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/jokers`);
}

/** Every joker dependency edge for cards in this deck, for building the full dependency graph client-side. */
export function listDeckJokers(deckId: string): Promise<CardJoker[]> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/jokers`);
}

export function addJoker(cardId: string, jokerId: string): Promise<CardJoker> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/jokers`, withBody('POST', { joker_id: jokerId }));
}

export function removeJoker(cardId: string, jokerId: string): Promise<void> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/jokers/${encodeURIComponent(jokerId)}`, {
		method: 'DELETE'
	});
}

export function getStack(cardId: string): Promise<Stack> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/stack`);
}

// ---------------------------------------------------------------------------
// Hand actions
// ---------------------------------------------------------------------------

export function dealHand(deckId: string): Promise<Stack[]> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/deal`, { method: 'POST' });
}

export function drawCard(deckId: string, cardId: string): Promise<Stack> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/draw/${encodeURIComponent(cardId)}`, {
		method: 'POST'
	});
}

export function completeCard(cardId: string): Promise<CompleteResult> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/complete`, { method: 'POST' });
}

export function returnCard(cardId: string): Promise<Card[]> {
	return apiFetch(`/cards/${encodeURIComponent(cardId)}/return`, { method: 'POST' });
}

export function reorderHand(deckId: string, order: string[]): Promise<Stack[]> {
	return apiFetch(`/decks/${encodeURIComponent(deckId)}/reorder`, withBody('PATCH', { order }));
}

// ---------------------------------------------------------------------------
// API tokens (long-lived, e.g. for the MCP server)
// ---------------------------------------------------------------------------

export function listApiTokens(): Promise<ApiTokenView[]> {
	return apiFetch('/api-tokens');
}

/** The raw token is only ever present in this response — it cannot be retrieved again. */
export function createApiToken(name: string): Promise<CreateApiTokenResponse> {
	return apiFetch('/api-tokens', withBody('POST', { name }));
}

export function deleteApiToken(id: string): Promise<void> {
	return apiFetch(`/api-tokens/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

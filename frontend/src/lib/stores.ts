import { get, writable } from 'svelte/store';
import {
	login as apiLogin,
	logout as apiLogout,
	refreshSession,
	setAccessToken,
	getAccessToken,
	tryRefresh,
	SESSION_EXPIRED_EVENT,
	TOKEN_REFRESHED_EVENT,
	type LoginRequest
} from './api';

interface JwtClaims {
	sub: string;
	role: string;
	exp: number;
	iat: number;
}

export interface SessionUser {
	id: string;
	role: string;
	username: string | null;
}

const USERNAME_STORAGE_KEY = 'cardflow.username';

// How long before expiry to refresh: 10% of the remaining lifetime, with a
// floor so short-lived tokens still get a sane margin and a minimum delay so
// we never schedule a near-immediate retry loop.
const MIN_REFRESH_MARGIN_MS = 30_000;
const MIN_REFRESH_DELAY_MS = 5_000;
// If a refresh attempt fails for a reason other than a definitive 401 (e.g. a
// transient network blip), try again shortly rather than giving up silently.
const RETRY_AFTER_FAILURE_MS = 30_000;

function decodeClaims(token: string): JwtClaims | null {
	try {
		const payload = token.split('.')[1];
		const normalized = payload.replace(/-/g, '+').replace(/_/g, '/');
		return JSON.parse(atob(normalized));
	} catch {
		return null;
	}
}

function rememberedUsername(): string | null {
	return typeof localStorage !== 'undefined' ? localStorage.getItem(USERNAME_STORAGE_KEY) : null;
}

function userFromToken(token: string): SessionUser | null {
	const claims = decodeClaims(token);
	if (!claims) return null;
	return { id: claims.sub, role: claims.role, username: rememberedUsername() };
}

export const sessionUser = writable<SessionUser | null>(null);
/** True once the initial silent-refresh attempt on page load has resolved. */
export const sessionReady = writable<boolean>(false);

let refreshTimer: ReturnType<typeof setTimeout> | null = null;

function clearRefreshTimer(): void {
	if (refreshTimer !== null) {
		clearTimeout(refreshTimer);
		refreshTimer = null;
	}
}

/**
 * Schedules a silent refresh shortly before `token` expires. This is purely
 * time-based against the token's own `exp` claim — there is no activity
 * tracking here. The real session boundary is the refresh token's 7-day
 * expiry, enforced server-side; this timer just keeps the access token
 * topped up so an open tab never has to surface that boundary early.
 */
function scheduleRefresh(token: string): void {
	clearRefreshTimer();
	const claims = decodeClaims(token);
	if (!claims) return;

	const msUntilExpiry = claims.exp * 1000 - Date.now();
	const margin = Math.max(msUntilExpiry * 0.1, MIN_REFRESH_MARGIN_MS);
	const delay = Math.max(msUntilExpiry - margin, MIN_REFRESH_DELAY_MS);

	refreshTimer = setTimeout(() => {
		void runSilentRefresh();
	}, delay);
}

async function runSilentRefresh(): Promise<void> {
	const ok = await tryRefresh();
	if (ok) {
		const token = getAccessToken();
		if (token) scheduleRefresh(token);
		return;
	}
	// A definitive 401 already dispatched SESSION_EXPIRED_EVENT (handled below),
	// which clears sessionUser. If it's still set, this was a transient failure
	// (e.g. network blip) — retry shortly instead of leaving the user stranded.
	if (get(sessionUser)) {
		refreshTimer = setTimeout(() => {
			void runSilentRefresh();
		}, RETRY_AFTER_FAILURE_MS);
	}
}

export async function login(payload: LoginRequest): Promise<void> {
	const session = await apiLogin(payload);
	setAccessToken(session.access_token);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(USERNAME_STORAGE_KEY, payload.username);
	}
	sessionUser.set(userFromToken(session.access_token));
	scheduleRefresh(session.access_token);
}

export async function logout(): Promise<void> {
	try {
		await apiLogout();
	} finally {
		clearSession();
	}
}

function clearSession(): void {
	clearRefreshTimer();
	setAccessToken(null);
	if (typeof localStorage !== 'undefined') {
		localStorage.removeItem(USERNAME_STORAGE_KEY);
	}
	sessionUser.set(null);
}

/** Attempts to restore a session from the refresh_token cookie on page load. */
export async function bootstrapSession(): Promise<void> {
	try {
		const session = await refreshSession();
		setAccessToken(session.access_token);
		sessionUser.set(userFromToken(session.access_token));
		scheduleRefresh(session.access_token);
	} catch {
		clearSession();
	} finally {
		sessionReady.set(true);
	}
}

if (typeof window !== 'undefined') {
	window.addEventListener(SESSION_EXPIRED_EVENT, clearSession);
	// Keeps sessionUser/the refresh timer in sync when the reactive,
	// 401-triggered refresh path (inside apiFetch) rotates the token instead
	// of this module's own proactive timer.
	window.addEventListener(TOKEN_REFRESHED_EVENT, (event) => {
		const token = (event as CustomEvent<string>).detail;
		sessionUser.set(userFromToken(token));
		scheduleRefresh(token);
	});
}

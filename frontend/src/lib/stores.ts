import { writable } from 'svelte/store';
import {
	login as apiLogin,
	logout as apiLogout,
	refreshSession,
	setAccessToken,
	SESSION_EXPIRED_EVENT,
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

export async function login(payload: LoginRequest): Promise<void> {
	const session = await apiLogin(payload);
	setAccessToken(session.access_token);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(USERNAME_STORAGE_KEY, payload.username);
	}
	sessionUser.set(userFromToken(session.access_token));
}

export async function logout(): Promise<void> {
	try {
		await apiLogout();
	} finally {
		clearSession();
	}
}

function clearSession(): void {
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
	} catch {
		clearSession();
	} finally {
		sessionReady.set(true);
	}
}

if (typeof window !== 'undefined') {
	window.addEventListener(SESSION_EXPIRED_EVENT, clearSession);
}

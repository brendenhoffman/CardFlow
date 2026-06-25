import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// Builds a static SPA bundle that nginx serves directly in Dockerfile.frontend,
			// falling back to index.html for client-side routes (see +layout.ts: ssr = false).
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		// Mirrors the production nginx proxy described in CONTEXT.md so the
		// frontend can always call same-origin /api/* paths.
		proxy: {
			'/api': {
				target: 'http://localhost:3001',
				changeOrigin: true,
				rewrite: (path) => path.replace(/^\/api/, '')
			}
		}
	}
});

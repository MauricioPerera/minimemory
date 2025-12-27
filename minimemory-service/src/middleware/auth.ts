/**
 * API Key Authentication Middleware
 */

import { Context, Next } from 'hono';

export interface AuthConfig {
	/** Header name for API key (default: X-API-Key) */
	headerName?: string;
	/** Query parameter name (fallback) */
	queryParam?: string;
	/** Skip auth for these paths */
	publicPaths?: string[];
	/** Validate API key function */
	validateKey: (key: string) => Promise<AuthResult | null>;
}

export interface AuthResult {
	valid: boolean;
	userId?: string;
	namespace?: string;
	permissions?: string[];
	rateLimit?: {
		limit: number;
		window: number; // in seconds
	};
}

/**
 * Simple in-memory API key store
 * In production, use a database or KV store
 */
export class ApiKeyStore {
	private keys: Map<string, AuthResult> = new Map();

	/**
	 * Add an API key
	 */
	addKey(apiKey: string, config: Omit<AuthResult, 'valid'>): void {
		this.keys.set(apiKey, { valid: true, ...config });
	}

	/**
	 * Remove an API key
	 */
	removeKey(apiKey: string): boolean {
		return this.keys.delete(apiKey);
	}

	/**
	 * Validate an API key
	 */
	async validate(apiKey: string): Promise<AuthResult | null> {
		const result = this.keys.get(apiKey);
		return result || null;
	}

	/**
	 * List all keys (for admin)
	 */
	listKeys(): string[] {
		return Array.from(this.keys.keys());
	}

	/**
	 * Generate a new API key
	 */
	static generateKey(): string {
		const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
		let key = 'mm_';
		for (let i = 0; i < 32; i++) {
			key += chars.charAt(Math.floor(Math.random() * chars.length));
		}
		return key;
	}
}

// Default key store instance
export const defaultKeyStore = new ApiKeyStore();

// Add a default development key
defaultKeyStore.addKey('mm_dev_key_12345', {
	userId: 'dev',
	namespace: 'default',
	permissions: ['read', 'write', 'admin'],
	rateLimit: { limit: 1000, window: 60 },
});

/**
 * Create authentication middleware
 */
export function createAuthMiddleware(config?: Partial<AuthConfig>) {
	const headerName = config?.headerName || 'X-API-Key';
	const queryParam = config?.queryParam || 'api_key';
	const publicPaths = config?.publicPaths || ['/', '/health'];
	const validateKey = config?.validateKey || ((key: string) => defaultKeyStore.validate(key));

	return async (c: Context, next: Next) => {
		const path = c.req.path;

		// Skip auth for public paths
		if (publicPaths.some(p => path === p || path.startsWith(p + '/'))) {
			await next();
			return;
		}

		// Skip if JWT auth already succeeded
		const authMethod = c.get('authMethod');
		if (authMethod === 'jwt') {
			await next();
			return;
		}

		// Get API key from header or query
		const apiKey = c.req.header(headerName) || c.req.query(queryParam);

		if (!apiKey) {
			return c.json({
				error: 'Unauthorized',
				message: `API key required. Provide via ${headerName} header or ${queryParam} query parameter.`,
			}, 401);
		}

		// Validate the key
		const authResult = await validateKey(apiKey);

		if (!authResult || !authResult.valid) {
			return c.json({
				error: 'Unauthorized',
				message: 'Invalid API key',
			}, 401);
		}

		// Store auth info in context
		c.set('auth', authResult);
		c.set('userId', authResult.userId || null);
		c.set('authMethod', 'apikey');

		// Set namespace from auth if not provided in header
		if (authResult.namespace && !c.req.header('X-Namespace')) {
			c.set('namespace', authResult.namespace);
		}

		await next();
	};
}

/**
 * Require specific permissions
 */
export function requirePermission(...permissions: string[]) {
	return async (c: Context, next: Next) => {
		const auth = c.get('auth') as AuthResult | undefined;

		if (!auth) {
			return c.json({
				error: 'Unauthorized',
				message: 'Authentication required',
			}, 401);
		}

		const userPermissions = auth.permissions || [];
		const hasPermission = permissions.every(p =>
			userPermissions.includes(p) || userPermissions.includes('admin')
		);

		if (!hasPermission) {
			return c.json({
				error: 'Forbidden',
				message: `Required permissions: ${permissions.join(', ')}`,
			}, 403);
		}

		await next();
	};
}

/**
 * API Key Authentication Middleware
 */
/**
 * Simple in-memory API key store
 * In production, use a database or KV store
 */
export class ApiKeyStore {
    keys = new Map();
    /**
     * Add an API key
     */
    addKey(apiKey, config) {
        this.keys.set(apiKey, { valid: true, ...config });
    }
    /**
     * Remove an API key
     */
    removeKey(apiKey) {
        return this.keys.delete(apiKey);
    }
    /**
     * Validate an API key
     */
    async validate(apiKey) {
        const result = this.keys.get(apiKey);
        return result || null;
    }
    /**
     * List all keys (for admin)
     */
    listKeys() {
        return Array.from(this.keys.keys());
    }
    /**
     * Generate a new API key
     */
    static generateKey() {
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
export function createAuthMiddleware(config) {
    const headerName = config?.headerName || 'X-API-Key';
    const queryParam = config?.queryParam || 'api_key';
    const publicPaths = config?.publicPaths || ['/', '/health'];
    const validateKey = config?.validateKey || ((key) => defaultKeyStore.validate(key));
    return async (c, next) => {
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
export function requirePermission(...permissions) {
    return async (c, next) => {
        const auth = c.get('auth');
        if (!auth) {
            return c.json({
                error: 'Unauthorized',
                message: 'Authentication required',
            }, 401);
        }
        const userPermissions = auth.permissions || [];
        const hasPermission = permissions.every(p => userPermissions.includes(p) || userPermissions.includes('admin'));
        if (!hasPermission) {
            return c.json({
                error: 'Forbidden',
                message: `Required permissions: ${permissions.join(', ')}`,
            }, 403);
        }
        await next();
    };
}
//# sourceMappingURL=auth.js.map
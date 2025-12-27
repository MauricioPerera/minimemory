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
        window: number;
    };
}
/**
 * Simple in-memory API key store
 * In production, use a database or KV store
 */
export declare class ApiKeyStore {
    private keys;
    /**
     * Add an API key
     */
    addKey(apiKey: string, config: Omit<AuthResult, 'valid'>): void;
    /**
     * Remove an API key
     */
    removeKey(apiKey: string): boolean;
    /**
     * Validate an API key
     */
    validate(apiKey: string): Promise<AuthResult | null>;
    /**
     * List all keys (for admin)
     */
    listKeys(): string[];
    /**
     * Generate a new API key
     */
    static generateKey(): string;
}
export declare const defaultKeyStore: ApiKeyStore;
/**
 * Create authentication middleware
 */
export declare function createAuthMiddleware(config?: Partial<AuthConfig>): (c: Context, next: Next) => Promise<(Response & import("hono").TypedResponse<{
    error: string;
    message: string;
}, 401, "json">) | undefined>;
/**
 * Require specific permissions
 */
export declare function requirePermission(...permissions: string[]): (c: Context, next: Next) => Promise<(Response & import("hono").TypedResponse<{
    error: string;
    message: string;
}, 401, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
    message: string;
}, 403, "json">) | undefined>;
//# sourceMappingURL=auth.d.ts.map
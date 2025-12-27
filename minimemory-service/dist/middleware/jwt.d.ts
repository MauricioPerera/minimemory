import { Context, Next } from 'hono';
import { AccessTokenPayload, TenantInfo } from '../utils/tokens';
declare module 'hono' {
    interface ContextVariableMap {
        user: AccessTokenPayload | null;
        userId: string | null;
        userEmail: string | null;
        userTenants: TenantInfo[];
        authMethod: 'jwt' | 'apikey' | null;
    }
}
export interface Env {
    DB: D1Database;
    JWT_SECRET: string;
    ALLOWED_ORIGINS?: string;
}
/**
 * JWT Authentication Middleware
 * Validates JWT tokens from Authorization header
 * Falls back to API key auth if no JWT is present
 */
export declare function jwtAuth(c: Context<{
    Bindings: Env;
}>, next: Next): Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 401, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
}, 500, "json">)>;
/**
 * Require JWT authentication
 * Use this middleware on routes that require a logged-in user
 */
export declare function requireJwtAuth(c: Context<{
    Bindings: Env;
}>, next: Next): Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 401, "json">)>;
/**
 * Require any authentication (JWT or API key)
 * Use this middleware on routes that accept both auth methods
 */
export declare function requireAuth(c: Context<{
    Bindings: Env;
}>, next: Next): Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 401, "json">)>;
/**
 * Check if user has access to a specific tenant
 */
export declare function hasAccessToTenant(c: Context, tenantId: string): boolean;
/**
 * Check if user has a specific role in a tenant
 */
export declare function hasRoleInTenant(c: Context, tenantId: string, roles: Array<'owner' | 'admin' | 'member' | 'viewer'>): boolean;
/**
 * Get user's role in a specific tenant
 */
export declare function getUserRoleInTenant(c: Context, tenantId: string): 'owner' | 'admin' | 'member' | 'viewer' | null;
//# sourceMappingURL=jwt.d.ts.map
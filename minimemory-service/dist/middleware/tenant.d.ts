import { Context, Next } from 'hono';
declare module 'hono' {
    interface ContextVariableMap {
        tenantId: string | null;
        tenantName: string | null;
        tenantRole: 'owner' | 'admin' | 'member' | 'viewer' | null;
        tenantPlan: string | null;
    }
}
interface Env {
    DB: D1Database;
    JWT_SECRET: string;
}
/**
 * Tenant extraction middleware
 * Extracts tenant ID from X-Tenant-Id header or API key binding
 */
export declare function extractTenant(c: Context<{
    Bindings: Env;
}>, next: Next): Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 403, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
}, 404, "json">)>;
/**
 * Require tenant context
 * Use this middleware on routes that need a tenant selected
 */
export declare function requireTenant(c: Context<{
    Bindings: Env;
}>, next: Next): Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 400, "json">)>;
/**
 * Require specific role in tenant
 * Use this factory to create middleware that requires minimum role
 */
export declare function requireTenantRole(minRoles: Array<'owner' | 'admin' | 'member' | 'viewer'>): (c: Context<{
    Bindings: Env;
}>, next: Next) => Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 400, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
    required: ("admin" | "owner" | "member" | "viewer")[];
    current: "admin" | "owner" | "member" | "viewer" | null;
}, 403, "json">)>;
/**
 * Check if user can write (owner, admin, member)
 */
export declare const requireWriteAccess: (c: Context<{
    Bindings: Env;
}>, next: Next) => Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 400, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
    required: ("admin" | "owner" | "member" | "viewer")[];
    current: "admin" | "owner" | "member" | "viewer" | null;
}, 403, "json">)>;
/**
 * Check if user can manage (owner, admin)
 */
export declare const requireManageAccess: (c: Context<{
    Bindings: Env;
}>, next: Next) => Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 400, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
    required: ("admin" | "owner" | "member" | "viewer")[];
    current: "admin" | "owner" | "member" | "viewer" | null;
}, 403, "json">)>;
/**
 * Check if user is owner
 */
export declare const requireOwnerAccess: (c: Context<{
    Bindings: Env;
}>, next: Next) => Promise<void | (Response & import("hono").TypedResponse<{
    error: string;
}, 400, "json">) | (Response & import("hono").TypedResponse<{
    error: string;
    required: ("admin" | "owner" | "member" | "viewer")[];
    current: "admin" | "owner" | "member" | "viewer" | null;
}, 403, "json">)>;
/**
 * Validate namespace belongs to tenant
 */
export declare function validateNamespaceAccess(c: Context<{
    Bindings: Env;
}>, namespace: string): Promise<{
    valid: boolean;
    error?: string;
}>;
/**
 * Check tenant limits (memories, namespaces)
 */
export declare function checkTenantLimits(c: Context<{
    Bindings: Env;
}>, type: 'memories' | 'namespaces'): Promise<{
    allowed: boolean;
    current: number;
    max: number;
}>;
export {};
//# sourceMappingURL=tenant.d.ts.map
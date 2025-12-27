import { verifyAccessToken } from '../utils/tokens';
/**
 * JWT Authentication Middleware
 * Validates JWT tokens from Authorization header
 * Falls back to API key auth if no JWT is present
 */
export async function jwtAuth(c, next) {
    const authHeader = c.req.header('Authorization');
    // Initialize context variables
    c.set('user', null);
    c.set('userId', null);
    c.set('userEmail', null);
    c.set('userTenants', []);
    c.set('authMethod', null);
    if (!authHeader) {
        // No auth header - continue to allow API key middleware to handle
        return next();
    }
    // Check for Bearer token
    if (authHeader.startsWith('Bearer ')) {
        const token = authHeader.slice(7);
        if (!token) {
            return c.json({ error: 'Invalid authorization header' }, 401);
        }
        const secret = c.env.JWT_SECRET;
        if (!secret) {
            console.error('JWT_SECRET not configured');
            return c.json({ error: 'Server configuration error' }, 500);
        }
        const payload = await verifyAccessToken(token, secret);
        if (!payload) {
            return c.json({ error: 'Invalid or expired token' }, 401);
        }
        // Set user context
        c.set('user', payload);
        c.set('userId', payload.sub);
        c.set('userEmail', payload.email);
        c.set('userTenants', payload.tenants || []);
        c.set('authMethod', 'jwt');
    }
    return next();
}
/**
 * Require JWT authentication
 * Use this middleware on routes that require a logged-in user
 */
export async function requireJwtAuth(c, next) {
    const user = c.get('user');
    const authMethod = c.get('authMethod');
    if (!user || authMethod !== 'jwt') {
        return c.json({ error: 'Authentication required' }, 401);
    }
    return next();
}
/**
 * Require any authentication (JWT or API key)
 * Use this middleware on routes that accept both auth methods
 */
export async function requireAuth(c, next) {
    const authMethod = c.get('authMethod');
    if (!authMethod) {
        return c.json({ error: 'Authentication required' }, 401);
    }
    return next();
}
/**
 * Check if user has access to a specific tenant
 */
export function hasAccessToTenant(c, tenantId) {
    const tenants = c.get('userTenants') || [];
    return tenants.some(t => t.id === tenantId);
}
/**
 * Check if user has a specific role in a tenant
 */
export function hasRoleInTenant(c, tenantId, roles) {
    const tenants = c.get('userTenants') || [];
    const tenant = tenants.find(t => t.id === tenantId);
    return tenant ? roles.includes(tenant.role) : false;
}
/**
 * Get user's role in a specific tenant
 */
export function getUserRoleInTenant(c, tenantId) {
    const tenants = c.get('userTenants') || [];
    const tenant = tenants.find(t => t.id === tenantId);
    return tenant ? tenant.role : null;
}
//# sourceMappingURL=jwt.js.map
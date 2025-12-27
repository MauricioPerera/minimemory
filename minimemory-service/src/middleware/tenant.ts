// Tenant Guard Middleware
import { Context, Next } from 'hono';
import { hasAccessToTenant, hasRoleInTenant, getUserRoleInTenant } from './jwt';

// Extend Hono context with tenant info
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
export async function extractTenant(c: Context<{ Bindings: Env }>, next: Next) {
  // Initialize tenant context
  c.set('tenantId', null);
  c.set('tenantName', null);
  c.set('tenantRole', null);
  c.set('tenantPlan', null);

  const tenantIdHeader = c.req.header('X-Tenant-Id');
  const authMethod = c.get('authMethod');

  if (authMethod === 'jwt' && tenantIdHeader) {
    // JWT auth - verify access to tenant
    if (!hasAccessToTenant(c, tenantIdHeader)) {
      return c.json({ error: 'Access denied to this tenant' }, 403);
    }

    const role = getUserRoleInTenant(c, tenantIdHeader);

    // Get tenant details from DB
    const db = c.env.DB;
    const tenant = await db
      .prepare('SELECT id, name, plan FROM tenants WHERE id = ?')
      .bind(tenantIdHeader)
      .first<{ id: string; name: string; plan: string }>();

    if (!tenant) {
      return c.json({ error: 'Tenant not found' }, 404);
    }

    c.set('tenantId', tenant.id);
    c.set('tenantName', tenant.name);
    c.set('tenantRole', role);
    c.set('tenantPlan', tenant.plan);
  } else if (authMethod === 'apikey') {
    // API key auth - tenant is derived from the API key
    // The API key middleware should have set this
    // For now, we'll look it up from the API key
    const apiKey = c.req.header('X-API-Key');
    if (apiKey) {
      const db = c.env.DB;
      const keyData = await db
        .prepare(`
          SELECT ak.tenant_id, t.name, t.plan
          FROM api_keys ak
          LEFT JOIN tenants t ON ak.tenant_id = t.id
          WHERE ak.key = ? AND ak.is_active = 1
        `)
        .bind(apiKey)
        .first<{ tenant_id: string | null; name: string | null; plan: string | null }>();

      if (keyData?.tenant_id) {
        c.set('tenantId', keyData.tenant_id);
        c.set('tenantName', keyData.name);
        c.set('tenantRole', 'admin'); // API keys get admin role
        c.set('tenantPlan', keyData.plan);
      }
    }
  }

  return next();
}

/**
 * Require tenant context
 * Use this middleware on routes that need a tenant selected
 */
export async function requireTenant(c: Context<{ Bindings: Env }>, next: Next) {
  const tenantId = c.get('tenantId');

  if (!tenantId) {
    return c.json({ error: 'Tenant selection required. Provide X-Tenant-Id header.' }, 400);
  }

  return next();
}

/**
 * Require specific role in tenant
 * Use this factory to create middleware that requires minimum role
 */
export function requireTenantRole(minRoles: Array<'owner' | 'admin' | 'member' | 'viewer'>) {
  return async (c: Context<{ Bindings: Env }>, next: Next) => {
    const tenantId = c.get('tenantId');
    const authMethod = c.get('authMethod');

    if (!tenantId) {
      return c.json({ error: 'Tenant selection required' }, 400);
    }

    // API key auth always has admin-level access
    if (authMethod === 'apikey') {
      return next();
    }

    // JWT auth - check role
    const role = c.get('tenantRole');
    if (!role || !minRoles.includes(role)) {
      return c.json({
        error: 'Insufficient permissions',
        required: minRoles,
        current: role,
      }, 403);
    }

    return next();
  };
}

/**
 * Check if user can write (owner, admin, member)
 */
export const requireWriteAccess = requireTenantRole(['owner', 'admin', 'member']);

/**
 * Check if user can manage (owner, admin)
 */
export const requireManageAccess = requireTenantRole(['owner', 'admin']);

/**
 * Check if user is owner
 */
export const requireOwnerAccess = requireTenantRole(['owner']);

/**
 * Validate namespace belongs to tenant
 */
export async function validateNamespaceAccess(
  c: Context<{ Bindings: Env }>,
  namespace: string
): Promise<{ valid: boolean; error?: string }> {
  const tenantId = c.get('tenantId');

  if (!tenantId) {
    return { valid: false, error: 'Tenant selection required' };
  }

  const db = c.env.DB;

  // Check if namespace exists and belongs to tenant
  const ns = await db
    .prepare('SELECT name, tenant_id FROM namespaces WHERE name = ?')
    .bind(namespace)
    .first<{ name: string; tenant_id: string | null }>();

  if (!ns) {
    // Namespace doesn't exist - allow creation if user has write access
    const role = c.get('tenantRole');
    if (!role || role === 'viewer') {
      return { valid: false, error: 'Insufficient permissions to create namespace' };
    }
    return { valid: true };
  }

  // Namespace exists - check if it belongs to the tenant
  if (ns.tenant_id && ns.tenant_id !== tenantId) {
    return { valid: false, error: 'Namespace belongs to another tenant' };
  }

  // Legacy namespace without tenant_id - allow access (migration case)
  if (!ns.tenant_id) {
    return { valid: true };
  }

  return { valid: true };
}

/**
 * Check tenant limits (memories, namespaces)
 */
export async function checkTenantLimits(
  c: Context<{ Bindings: Env }>,
  type: 'memories' | 'namespaces'
): Promise<{ allowed: boolean; current: number; max: number }> {
  const tenantId = c.get('tenantId');

  if (!tenantId) {
    return { allowed: false, current: 0, max: 0 };
  }

  const db = c.env.DB;

  // Get tenant limits
  const tenant = await db
    .prepare('SELECT max_memories, max_namespaces FROM tenants WHERE id = ?')
    .bind(tenantId)
    .first<{ max_memories: number; max_namespaces: number }>();

  if (!tenant) {
    return { allowed: false, current: 0, max: 0 };
  }

  if (type === 'memories') {
    // Count memories in tenant's namespaces
    const count = await db
      .prepare(`
        SELECT COUNT(*) as count
        FROM memories m
        JOIN namespaces n ON m.namespace = n.name
        WHERE n.tenant_id = ?
      `)
      .bind(tenantId)
      .first<{ count: number }>();

    return {
      allowed: (count?.count || 0) < tenant.max_memories,
      current: count?.count || 0,
      max: tenant.max_memories,
    };
  } else {
    // Count namespaces
    const count = await db
      .prepare('SELECT COUNT(*) as count FROM namespaces WHERE tenant_id = ?')
      .bind(tenantId)
      .first<{ count: number }>();

    return {
      allowed: (count?.count || 0) < tenant.max_namespaces,
      current: count?.count || 0,
      max: tenant.max_namespaces,
    };
  }
}

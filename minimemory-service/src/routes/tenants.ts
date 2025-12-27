// Tenant Management Routes
import { Hono } from 'hono';
import { requireJwtAuth } from '../middleware/jwt';
import { generateId } from '../utils/tokens';

interface Env {
  DB: D1Database;
  JWT_SECRET: string;
}

const tenants = new Hono<{ Bindings: Env }>();

// All tenant routes require JWT authentication
tenants.use('/*', requireJwtAuth);

/**
 * GET /tenants
 * List all tenants for current user
 */
tenants.get('/', async (c) => {
  try {
    const userId = c.get('userId');
    const db = c.env.DB;

    const tenantsResult = await db
      .prepare(`
        SELECT
          t.id,
          t.name,
          t.plan,
          t.max_memories,
          t.max_namespaces,
          t.created_at,
          t.updated_at,
          ut.role,
          (SELECT COUNT(*) FROM namespaces WHERE tenant_id = t.id) as namespace_count,
          (SELECT COUNT(*) FROM user_tenants WHERE tenant_id = t.id) as member_count
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
        ORDER BY t.created_at DESC
      `)
      .bind(userId)
      .all<{
        id: string;
        name: string;
        plan: string;
        max_memories: number;
        max_namespaces: number;
        created_at: number;
        updated_at: number;
        role: string;
        namespace_count: number;
        member_count: number;
      }>();

    return c.json({
      tenants: tenantsResult.results.map(t => ({
        id: t.id,
        name: t.name,
        plan: t.plan,
        maxMemories: t.max_memories,
        maxNamespaces: t.max_namespaces,
        role: t.role,
        namespaceCount: t.namespace_count,
        memberCount: t.member_count,
        createdAt: t.created_at,
        updatedAt: t.updated_at,
      })),
    });
  } catch (error) {
    console.error('List tenants error:', error);
    return c.json({ error: 'Failed to list tenants' }, 500);
  }
});

/**
 * POST /tenants
 * Create a new tenant
 */
tenants.post('/', async (c) => {
  try {
    const userId = c.get('userId');
    const body = await c.req.json();
    const { name } = body;

    if (!name || name.trim().length === 0) {
      return c.json({ error: 'Tenant name is required' }, 400);
    }

    if (name.length > 100) {
      return c.json({ error: 'Tenant name must be 100 characters or less' }, 400);
    }

    const db = c.env.DB;
    const tenantId = generateId();
    const now = Date.now();

    // Create tenant
    await db
      .prepare(`
        INSERT INTO tenants (id, name, plan, max_memories, max_namespaces, created_at, updated_at)
        VALUES (?, ?, 'free', 1000, 1, ?, ?)
      `)
      .bind(tenantId, name.trim(), now, now)
      .run();

    // Link user as owner
    await db
      .prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, 'owner', ?)
      `)
      .bind(userId, tenantId, now)
      .run();

    // Create default namespace
    await db
      .prepare(`
        INSERT INTO namespaces (name, tenant_id, dimensions, created_at, updated_at)
        VALUES (?, ?, 1536, ?, ?)
      `)
      .bind(`${tenantId}-default`, tenantId, now, now)
      .run();

    return c.json({
      success: true,
      tenant: {
        id: tenantId,
        name: name.trim(),
        plan: 'free',
        maxMemories: 1000,
        maxNamespaces: 1,
        role: 'owner',
        createdAt: now,
        updatedAt: now,
      },
    }, 201);
  } catch (error) {
    console.error('Create tenant error:', error);
    return c.json({ error: 'Failed to create tenant' }, 500);
  }
});

/**
 * GET /tenants/:id
 * Get tenant details
 */
tenants.get('/:id', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const db = c.env.DB;

    // Check user has access
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership) {
      return c.json({ error: 'Tenant not found or access denied' }, 404);
    }

    // Get tenant details with stats
    const tenant = await db
      .prepare(`
        SELECT
          t.*,
          (SELECT COUNT(*) FROM namespaces WHERE tenant_id = t.id) as namespace_count,
          (SELECT COUNT(*) FROM user_tenants WHERE tenant_id = t.id) as member_count,
          (SELECT COUNT(*) FROM memories m JOIN namespaces n ON m.namespace = n.name WHERE n.tenant_id = t.id) as memory_count
        FROM tenants t
        WHERE t.id = ?
      `)
      .bind(tenantId)
      .first<{
        id: string;
        name: string;
        plan: string;
        max_memories: number;
        max_namespaces: number;
        created_at: number;
        updated_at: number;
        namespace_count: number;
        member_count: number;
        memory_count: number;
      }>();

    if (!tenant) {
      return c.json({ error: 'Tenant not found' }, 404);
    }

    // Get namespaces
    const namespacesResult = await db
      .prepare(`
        SELECT
          n.name,
          n.dimensions,
          n.created_at,
          (SELECT COUNT(*) FROM memories WHERE namespace = n.name) as memory_count
        FROM namespaces n
        WHERE n.tenant_id = ?
        ORDER BY n.created_at DESC
      `)
      .bind(tenantId)
      .all<{ name: string; dimensions: number; created_at: number; memory_count: number }>();

    return c.json({
      tenant: {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
        maxMemories: tenant.max_memories,
        maxNamespaces: tenant.max_namespaces,
        role: membership.role,
        stats: {
          namespaces: tenant.namespace_count,
          members: tenant.member_count,
          memories: tenant.memory_count,
        },
        createdAt: tenant.created_at,
        updatedAt: tenant.updated_at,
      },
      namespaces: namespacesResult.results.map(n => ({
        name: n.name,
        dimensions: n.dimensions,
        memoryCount: n.memory_count,
        createdAt: n.created_at,
      })),
    });
  } catch (error) {
    console.error('Get tenant error:', error);
    return c.json({ error: 'Failed to get tenant' }, 500);
  }
});

/**
 * PUT /tenants/:id
 * Update tenant (owner/admin only)
 */
tenants.put('/:id', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const body = await c.req.json();
    const { name } = body;
    const db = c.env.DB;

    // Check user has admin/owner access
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership || !['owner', 'admin'].includes(membership.role)) {
      return c.json({ error: 'Access denied. Owner or admin role required.' }, 403);
    }

    if (!name || name.trim().length === 0) {
      return c.json({ error: 'Tenant name is required' }, 400);
    }

    if (name.length > 100) {
      return c.json({ error: 'Tenant name must be 100 characters or less' }, 400);
    }

    const now = Date.now();

    await db
      .prepare('UPDATE tenants SET name = ?, updated_at = ? WHERE id = ?')
      .bind(name.trim(), now, tenantId)
      .run();

    return c.json({
      success: true,
      tenant: {
        id: tenantId,
        name: name.trim(),
        updatedAt: now,
      },
    });
  } catch (error) {
    console.error('Update tenant error:', error);
    return c.json({ error: 'Failed to update tenant' }, 500);
  }
});

/**
 * DELETE /tenants/:id
 * Delete tenant (owner only)
 */
tenants.delete('/:id', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const db = c.env.DB;

    // Check user is owner
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership || membership.role !== 'owner') {
      return c.json({ error: 'Access denied. Owner role required.' }, 403);
    }

    // Check this isn't the user's last tenant
    const tenantCount = await db
      .prepare('SELECT COUNT(*) as count FROM user_tenants WHERE user_id = ?')
      .bind(userId)
      .first<{ count: number }>();

    if (tenantCount && tenantCount.count <= 1) {
      return c.json({ error: 'Cannot delete your last tenant' }, 400);
    }

    // Delete tenant (cascade will handle related records)
    await db.prepare('DELETE FROM tenants WHERE id = ?').bind(tenantId).run();

    return c.json({ success: true, message: 'Tenant deleted successfully' });
  } catch (error) {
    console.error('Delete tenant error:', error);
    return c.json({ error: 'Failed to delete tenant' }, 500);
  }
});

// ===============================
// MEMBER MANAGEMENT
// ===============================

/**
 * GET /tenants/:id/members
 * List tenant members
 */
tenants.get('/:id/members', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const db = c.env.DB;

    // Check user has access
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership) {
      return c.json({ error: 'Tenant not found or access denied' }, 404);
    }

    // Get members
    const membersResult = await db
      .prepare(`
        SELECT u.id, u.email, u.name, ut.role, ut.created_at
        FROM users u
        JOIN user_tenants ut ON u.id = ut.user_id
        WHERE ut.tenant_id = ?
        ORDER BY ut.created_at ASC
      `)
      .bind(tenantId)
      .all<{ id: string; email: string; name: string | null; role: string; created_at: number }>();

    return c.json({
      members: membersResult.results.map(m => ({
        id: m.id,
        email: m.email,
        name: m.name,
        role: m.role,
        joinedAt: m.created_at,
      })),
    });
  } catch (error) {
    console.error('List members error:', error);
    return c.json({ error: 'Failed to list members' }, 500);
  }
});

/**
 * POST /tenants/:id/members
 * Invite member by email
 */
tenants.post('/:id/members', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const body = await c.req.json();
    const { email, role = 'member' } = body;
    const db = c.env.DB;

    // Validate role
    const validRoles = ['admin', 'member', 'viewer'];
    if (!validRoles.includes(role)) {
      return c.json({ error: 'Invalid role. Must be admin, member, or viewer.' }, 400);
    }

    // Check user has admin/owner access
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership || !['owner', 'admin'].includes(membership.role)) {
      return c.json({ error: 'Access denied. Owner or admin role required.' }, 403);
    }

    // Find user by email
    const targetUser = await db
      .prepare('SELECT id, email, name FROM users WHERE email = ?')
      .bind(email.toLowerCase())
      .first<{ id: string; email: string; name: string | null }>();

    if (!targetUser) {
      return c.json({ error: 'User not found. They must register first.' }, 404);
    }

    // Check if already a member
    const existingMembership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(targetUser.id, tenantId)
      .first();

    if (existingMembership) {
      return c.json({ error: 'User is already a member of this tenant' }, 409);
    }

    const now = Date.now();

    // Add member
    await db
      .prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, ?, ?)
      `)
      .bind(targetUser.id, tenantId, role, now)
      .run();

    return c.json({
      success: true,
      member: {
        id: targetUser.id,
        email: targetUser.email,
        name: targetUser.name,
        role,
        joinedAt: now,
      },
    }, 201);
  } catch (error) {
    console.error('Invite member error:', error);
    return c.json({ error: 'Failed to invite member' }, 500);
  }
});

/**
 * PUT /tenants/:id/members/:userId
 * Update member role
 */
tenants.put('/:id/members/:memberId', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const memberId = c.req.param('memberId');
    const body = await c.req.json();
    const { role } = body;
    const db = c.env.DB;

    // Validate role
    const validRoles = ['admin', 'member', 'viewer'];
    if (!validRoles.includes(role)) {
      return c.json({ error: 'Invalid role. Must be admin, member, or viewer.' }, 400);
    }

    // Check user has owner access (only owner can change roles)
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership || membership.role !== 'owner') {
      return c.json({ error: 'Access denied. Owner role required.' }, 403);
    }

    // Can't change owner role
    const targetMembership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(memberId, tenantId)
      .first<{ role: string }>();

    if (!targetMembership) {
      return c.json({ error: 'Member not found' }, 404);
    }

    if (targetMembership.role === 'owner') {
      return c.json({ error: 'Cannot change owner role' }, 400);
    }

    // Update role
    await db
      .prepare('UPDATE user_tenants SET role = ? WHERE user_id = ? AND tenant_id = ?')
      .bind(role, memberId, tenantId)
      .run();

    return c.json({
      success: true,
      message: 'Member role updated',
      role,
    });
  } catch (error) {
    console.error('Update member error:', error);
    return c.json({ error: 'Failed to update member' }, 500);
  }
});

/**
 * DELETE /tenants/:id/members/:userId
 * Remove member from tenant
 */
tenants.delete('/:id/members/:memberId', async (c) => {
  try {
    const userId = c.get('userId');
    const tenantId = c.req.param('id');
    const memberId = c.req.param('memberId');
    const db = c.env.DB;

    // Check user has admin/owner access
    const membership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(userId, tenantId)
      .first<{ role: string }>();

    if (!membership || !['owner', 'admin'].includes(membership.role)) {
      return c.json({ error: 'Access denied. Owner or admin role required.' }, 403);
    }

    // Can't remove owner
    const targetMembership = await db
      .prepare('SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(memberId, tenantId)
      .first<{ role: string }>();

    if (!targetMembership) {
      return c.json({ error: 'Member not found' }, 404);
    }

    if (targetMembership.role === 'owner') {
      return c.json({ error: 'Cannot remove tenant owner' }, 400);
    }

    // Admin can only remove members/viewers
    if (membership.role === 'admin' && targetMembership.role === 'admin') {
      return c.json({ error: 'Admins cannot remove other admins' }, 403);
    }

    // Remove member
    await db
      .prepare('DELETE FROM user_tenants WHERE user_id = ? AND tenant_id = ?')
      .bind(memberId, tenantId)
      .run();

    return c.json({ success: true, message: 'Member removed' });
  } catch (error) {
    console.error('Remove member error:', error);
    return c.json({ error: 'Failed to remove member' }, 500);
  }
});

export default tenants;

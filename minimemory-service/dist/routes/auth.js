// Authentication Routes
import { Hono } from 'hono';
import { hashPassword, verifyPassword, validatePassword, validateEmail } from '../utils/password';
import { createAccessToken, createRefreshToken, verifyRefreshToken, hashRefreshToken, generateId, getRefreshTokenExpiry, } from '../utils/tokens';
import { requireJwtAuth } from '../middleware/jwt';
const auth = new Hono();
/**
 * POST /auth/register
 * Create a new user account with initial tenant
 */
auth.post('/register', async (c) => {
    try {
        const body = await c.req.json();
        const { email, password, name } = body;
        // Validate input
        if (!email || !password) {
            return c.json({ error: 'Email and password are required' }, 400);
        }
        if (!validateEmail(email)) {
            return c.json({ error: 'Invalid email format' }, 400);
        }
        const passwordError = validatePassword(password);
        if (passwordError) {
            return c.json({ error: passwordError }, 400);
        }
        const db = c.env.DB;
        // Check if email already exists
        const existingUser = await db
            .prepare('SELECT id FROM users WHERE email = ?')
            .bind(email.toLowerCase())
            .first();
        if (existingUser) {
            return c.json({ error: 'Email already registered' }, 409);
        }
        // Generate IDs
        const userId = generateId();
        const tenantId = generateId();
        const sessionId = generateId();
        const now = Date.now();
        // Hash password
        const passwordHash = await hashPassword(password);
        // Create user
        await db
            .prepare(`
        INSERT INTO users (id, email, password_hash, name, is_active, created_at, last_login)
        VALUES (?, ?, ?, ?, 1, ?, ?)
      `)
            .bind(userId, email.toLowerCase(), passwordHash, name || null, now, now)
            .run();
        // Create personal tenant
        const tenantName = name ? `${name}'s Workspace` : 'My Workspace';
        await db
            .prepare(`
        INSERT INTO tenants (id, name, plan, max_memories, max_namespaces, created_at, updated_at)
        VALUES (?, ?, 'free', 1000, 1, ?, ?)
      `)
            .bind(tenantId, tenantName, now, now)
            .run();
        // Link user to tenant as owner
        await db
            .prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, 'owner', ?)
      `)
            .bind(userId, tenantId, now)
            .run();
        // Create default namespace for tenant
        await db
            .prepare(`
        INSERT INTO namespaces (name, tenant_id, dimensions, created_at, updated_at)
        VALUES (?, ?, 1536, ?, ?)
      `)
            .bind(`${tenantId}-default`, tenantId, now, now)
            .run();
        // Create tokens
        const tenantInfo = [{
                id: tenantId,
                name: tenantName,
                role: 'owner',
            }];
        const accessToken = await createAccessToken({ sub: userId, email: email.toLowerCase(), name: name || '', tenants: tenantInfo }, c.env.JWT_SECRET);
        const refreshToken = await createRefreshToken(userId, sessionId, c.env.JWT_REFRESH_SECRET);
        const refreshTokenHash = await hashRefreshToken(refreshToken);
        // Store session
        await db
            .prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `)
            .bind(sessionId, userId, refreshTokenHash, getRefreshTokenExpiry(), now)
            .run();
        return c.json({
            success: true,
            accessToken,
            refreshToken,
            user: {
                id: userId,
                email: email.toLowerCase(),
                name: name || null,
            },
            tenants: tenantInfo,
        }, 201);
    }
    catch (error) {
        console.error('Registration error:', error);
        const message = error instanceof Error ? error.message : 'Unknown error';
        return c.json({ error: 'Registration failed', details: message }, 500);
    }
});
/**
 * POST /auth/login
 * Authenticate user and return tokens
 */
auth.post('/login', async (c) => {
    try {
        const body = await c.req.json();
        const { email, password } = body;
        if (!email || !password) {
            return c.json({ error: 'Email and password are required' }, 400);
        }
        const db = c.env.DB;
        // Find user
        const user = await db
            .prepare('SELECT id, email, password_hash, name, is_active FROM users WHERE email = ?')
            .bind(email.toLowerCase())
            .first();
        if (!user) {
            return c.json({ error: 'Invalid email or password' }, 401);
        }
        if (!user.is_active) {
            return c.json({ error: 'Account is disabled' }, 403);
        }
        // Verify password
        const isValid = await verifyPassword(password, user.password_hash);
        if (!isValid) {
            return c.json({ error: 'Invalid email or password' }, 401);
        }
        // Get user's tenants
        const tenantsResult = await db
            .prepare(`
        SELECT t.id, t.name, ut.role
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `)
            .bind(user.id)
            .all();
        const tenants = tenantsResult.results.map(t => ({
            id: t.id,
            name: t.name,
            role: t.role,
        }));
        // Create session
        const sessionId = generateId();
        const now = Date.now();
        const accessToken = await createAccessToken({ sub: user.id, email: user.email, name: user.name || '', tenants }, c.env.JWT_SECRET);
        const refreshToken = await createRefreshToken(user.id, sessionId, c.env.JWT_REFRESH_SECRET);
        const refreshTokenHash = await hashRefreshToken(refreshToken);
        // Store session
        await db
            .prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `)
            .bind(sessionId, user.id, refreshTokenHash, getRefreshTokenExpiry(), now)
            .run();
        // Update last login
        await db
            .prepare('UPDATE users SET last_login = ? WHERE id = ?')
            .bind(now, user.id)
            .run();
        return c.json({
            success: true,
            accessToken,
            refreshToken,
            user: {
                id: user.id,
                email: user.email,
                name: user.name,
            },
            tenants,
        });
    }
    catch (error) {
        console.error('Login error:', error);
        return c.json({ error: 'Login failed' }, 500);
    }
});
/**
 * POST /auth/refresh
 * Refresh access token using refresh token
 */
auth.post('/refresh', async (c) => {
    try {
        const body = await c.req.json();
        const { refreshToken } = body;
        if (!refreshToken) {
            return c.json({ error: 'Refresh token is required' }, 400);
        }
        // Verify refresh token
        const payload = await verifyRefreshToken(refreshToken, c.env.JWT_REFRESH_SECRET);
        if (!payload) {
            return c.json({ error: 'Invalid or expired refresh token' }, 401);
        }
        const db = c.env.DB;
        // Hash the token to compare with stored hash
        const tokenHash = await hashRefreshToken(refreshToken);
        // Find session
        const session = await db
            .prepare(`
        SELECT s.id, s.user_id, s.expires_at, u.email, u.name, u.is_active
        FROM sessions s
        JOIN users u ON s.user_id = u.id
        WHERE s.refresh_token_hash = ? AND s.user_id = ?
      `)
            .bind(tokenHash, payload.sub)
            .first();
        if (!session) {
            return c.json({ error: 'Session not found' }, 401);
        }
        if (session.expires_at < Date.now()) {
            // Clean up expired session
            await db.prepare('DELETE FROM sessions WHERE id = ?').bind(session.id).run();
            return c.json({ error: 'Session expired' }, 401);
        }
        if (!session.is_active) {
            return c.json({ error: 'Account is disabled' }, 403);
        }
        // Get user's tenants
        const tenantsResult = await db
            .prepare(`
        SELECT t.id, t.name, ut.role
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `)
            .bind(session.user_id)
            .all();
        const tenants = tenantsResult.results.map(t => ({
            id: t.id,
            name: t.name,
            role: t.role,
        }));
        // Rotate refresh token (create new session, delete old)
        const newSessionId = generateId();
        const now = Date.now();
        const newAccessToken = await createAccessToken({ sub: session.user_id, email: session.email, name: session.name || '', tenants }, c.env.JWT_SECRET);
        const newRefreshToken = await createRefreshToken(session.user_id, newSessionId, c.env.JWT_REFRESH_SECRET);
        const newRefreshTokenHash = await hashRefreshToken(newRefreshToken);
        // Delete old session and create new one
        await db.prepare('DELETE FROM sessions WHERE id = ?').bind(session.id).run();
        await db
            .prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `)
            .bind(newSessionId, session.user_id, newRefreshTokenHash, getRefreshTokenExpiry(), now)
            .run();
        return c.json({
            success: true,
            accessToken: newAccessToken,
            refreshToken: newRefreshToken,
        });
    }
    catch (error) {
        console.error('Refresh error:', error);
        return c.json({ error: 'Token refresh failed' }, 500);
    }
});
/**
 * POST /auth/logout
 * Invalidate current session
 */
auth.post('/logout', async (c) => {
    try {
        const body = await c.req.json();
        const { refreshToken } = body;
        if (!refreshToken) {
            return c.json({ error: 'Refresh token is required' }, 400);
        }
        const db = c.env.DB;
        const tokenHash = await hashRefreshToken(refreshToken);
        // Delete session
        await db.prepare('DELETE FROM sessions WHERE refresh_token_hash = ?').bind(tokenHash).run();
        return c.json({ success: true, message: 'Logged out successfully' });
    }
    catch (error) {
        console.error('Logout error:', error);
        return c.json({ error: 'Logout failed' }, 500);
    }
});
/**
 * GET /auth/me
 * Get current user profile
 */
auth.get('/me', requireJwtAuth, async (c) => {
    try {
        const userId = c.get('userId');
        const db = c.env.DB;
        // Get user details
        const user = await db
            .prepare('SELECT id, email, name, created_at, last_login FROM users WHERE id = ?')
            .bind(userId)
            .first();
        if (!user) {
            return c.json({ error: 'User not found' }, 404);
        }
        // Get user's tenants with stats
        const tenantsResult = await db
            .prepare(`
        SELECT
          t.id,
          t.name,
          t.plan,
          t.max_memories,
          t.max_namespaces,
          ut.role,
          t.created_at
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `)
            .bind(userId)
            .all();
        return c.json({
            user: {
                id: user.id,
                email: user.email,
                name: user.name,
                createdAt: user.created_at,
                lastLogin: user.last_login,
            },
            tenants: tenantsResult.results.map(t => ({
                id: t.id,
                name: t.name,
                plan: t.plan,
                maxMemories: t.max_memories,
                maxNamespaces: t.max_namespaces,
                role: t.role,
                createdAt: t.created_at,
            })),
        });
    }
    catch (error) {
        console.error('Get profile error:', error);
        return c.json({ error: 'Failed to get profile' }, 500);
    }
});
/**
 * PUT /auth/password
 * Change user password
 */
auth.put('/password', requireJwtAuth, async (c) => {
    try {
        const userId = c.get('userId');
        const body = await c.req.json();
        const { currentPassword, newPassword } = body;
        if (!currentPassword || !newPassword) {
            return c.json({ error: 'Current password and new password are required' }, 400);
        }
        const passwordError = validatePassword(newPassword);
        if (passwordError) {
            return c.json({ error: passwordError }, 400);
        }
        const db = c.env.DB;
        // Get current password hash
        const user = await db
            .prepare('SELECT password_hash FROM users WHERE id = ?')
            .bind(userId)
            .first();
        if (!user) {
            return c.json({ error: 'User not found' }, 404);
        }
        // Verify current password
        const isValid = await verifyPassword(currentPassword, user.password_hash);
        if (!isValid) {
            return c.json({ error: 'Current password is incorrect' }, 401);
        }
        // Hash new password and update
        const newPasswordHash = await hashPassword(newPassword);
        await db
            .prepare('UPDATE users SET password_hash = ? WHERE id = ?')
            .bind(newPasswordHash, userId)
            .run();
        // Invalidate all sessions (force re-login)
        await db.prepare('DELETE FROM sessions WHERE user_id = ?').bind(userId).run();
        return c.json({ success: true, message: 'Password changed successfully' });
    }
    catch (error) {
        console.error('Change password error:', error);
        return c.json({ error: 'Failed to change password' }, 500);
    }
});
export default auth;
//# sourceMappingURL=auth.js.map
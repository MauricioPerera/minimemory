/**
 * Agent Token API Routes
 *
 * Endpoints for managing agent tokens for MCP access control
 */

import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
import {
	AgentTokenService,
	type AgentPermission,
	type CreateAgentTokenOptions,
	type UpdateAgentTokenOptions,
} from '../services/AgentTokenService.js';
import { requireJwtAuth } from '../middleware/jwt.js';

type Bindings = {
	DB?: D1Database;
};

const agentTokenRoutes = new Hono<{ Bindings: Bindings }>();

// Apply JWT auth to all routes
agentTokenRoutes.use('*', requireJwtAuth);

/**
 * Validate permissions array
 */
function validatePermissions(permissions: unknown): AgentPermission[] | null {
	if (!Array.isArray(permissions)) return null;
	if (permissions.length === 0) return null;

	const validPermissions: AgentPermission[] = ['read', 'write'];
	for (const perm of permissions) {
		if (typeof perm !== 'string') return null;
		if (!validPermissions.includes(perm as AgentPermission)) return null;
	}

	return permissions as AgentPermission[];
}

/**
 * Validate allowed memories array
 */
function validateAllowedMemories(memories: unknown): string[] | null {
	if (!Array.isArray(memories)) return null;
	if (memories.length === 0) return null;

	for (const mem of memories) {
		if (typeof mem !== 'string') return null;
		if (mem.length === 0) return null;
	}

	return memories as string[];
}

// ============ Token Management ============

/**
 * GET /agent-tokens
 * List all agent tokens for the authenticated user
 */
agentTokenRoutes.get('/', async (c) => {
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const activeOnly = c.req.query('active') === 'true';
	const limit = parseInt(c.req.query('limit') || '100', 10);
	const offset = parseInt(c.req.query('offset') || '0', 10);

	const result = await service.list({ userId, activeOnly, limit, offset });

	return c.json({
		tokens: result.tokens,
		total: result.total,
		hasMore: result.hasMore,
	});
});

/**
 * GET /agent-tokens/stats
 * Get usage statistics for the user's tokens
 */
agentTokenRoutes.get('/stats', async (c) => {
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const stats = await service.getStats(userId);

	return c.json({ stats });
});

/**
 * POST /agent-tokens
 * Create a new agent token
 */
agentTokenRoutes.post('/', async (c) => {
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	try {
		const body = await c.req.json();
		const {
			name,
			description,
			allowedMemories,
			permissions,
			expiresAt,
			tenantId,
		} = body as {
			name?: string;
			description?: string;
			allowedMemories?: unknown;
			permissions?: unknown;
			expiresAt?: number;
			tenantId?: string;
		};

		// Validate name
		if (!name || typeof name !== 'string' || name.trim().length === 0) {
			return c.json({ error: 'name is required' }, 400);
		}

		if (name.length > 100) {
			return c.json({ error: 'name must be 100 characters or less' }, 400);
		}

		// Validate permissions if provided
		let validatedPermissions: AgentPermission[] | undefined;
		if (permissions !== undefined) {
			const result = validatePermissions(permissions);
			if (!result) {
				return c.json({
					error: 'permissions must be a non-empty array of valid permissions',
					validPermissions: ['read', 'write'],
				}, 400);
			}
			validatedPermissions = result;
		}

		// Validate allowed memories if provided
		let validatedMemories: string[] | undefined;
		if (allowedMemories !== undefined) {
			const result = validateAllowedMemories(allowedMemories);
			if (!result) {
				return c.json({
					error: 'allowedMemories must be a non-empty array of memory IDs (or ["*"] for all)',
				}, 400);
			}
			validatedMemories = result;
		}

		// Validate expiration if provided
		if (expiresAt !== undefined) {
			if (typeof expiresAt !== 'number' || expiresAt <= Date.now()) {
				return c.json({ error: 'expiresAt must be a future timestamp' }, 400);
			}
		}

		const service = new AgentTokenService(c.env.DB);

		const options: CreateAgentTokenOptions = {
			userId,
			tenantId,
			name: name.trim(),
			description: description?.trim(),
			allowedMemories: validatedMemories,
			permissions: validatedPermissions,
			expiresAt,
		};

		const token = await service.create(options);

		return c.json({
			token,
			message: 'Agent token created. Use this ID with your API key to authenticate MCP connections.',
		}, 201);
	} catch (error) {
		console.error('Error creating agent token:', error);
		return c.json({ error: 'Failed to create agent token' }, 500);
	}
});

/**
 * GET /agent-tokens/:id
 * Get a specific agent token
 */
agentTokenRoutes.get('/:id', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const token = await service.getByIdAndUser(id, userId);

	if (!token) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	return c.json({ token });
});

/**
 * PATCH /agent-tokens/:id
 * Update an agent token
 */
agentTokenRoutes.patch('/:id', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const existing = await service.getByIdAndUser(id, userId);

	if (!existing) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	try {
		const body = await c.req.json();
		const updates: UpdateAgentTokenOptions = {};

		// Validate and apply updates
		if (body.name !== undefined) {
			if (typeof body.name !== 'string' || body.name.trim().length === 0) {
				return c.json({ error: 'name must be a non-empty string' }, 400);
			}
			if (body.name.length > 100) {
				return c.json({ error: 'name must be 100 characters or less' }, 400);
			}
			updates.name = body.name.trim();
		}

		if (body.description !== undefined) {
			updates.description = body.description?.trim() || undefined;
		}

		if (body.permissions !== undefined) {
			const validatedPermissions = validatePermissions(body.permissions);
			if (!validatedPermissions) {
				return c.json({
					error: 'permissions must be a non-empty array of valid permissions',
					validPermissions: ['read', 'write'],
				}, 400);
			}
			updates.permissions = validatedPermissions;
		}

		if (body.allowedMemories !== undefined) {
			const validatedMemories = validateAllowedMemories(body.allowedMemories);
			if (!validatedMemories) {
				return c.json({
					error: 'allowedMemories must be a non-empty array of memory IDs',
				}, 400);
			}
			updates.allowedMemories = validatedMemories;
		}

		if (body.isActive !== undefined) {
			updates.isActive = Boolean(body.isActive);
		}

		if (body.expiresAt !== undefined) {
			if (body.expiresAt === null) {
				updates.expiresAt = null;
			} else if (typeof body.expiresAt === 'number' && body.expiresAt > Date.now()) {
				updates.expiresAt = body.expiresAt;
			} else {
				return c.json({ error: 'expiresAt must be null or a future timestamp' }, 400);
			}
		}

		const token = await service.update(id, updates);

		return c.json({ token });
	} catch (error) {
		console.error('Error updating agent token:', error);
		return c.json({ error: 'Failed to update agent token' }, 500);
	}
});

/**
 * DELETE /agent-tokens/:id
 * Delete an agent token
 */
agentTokenRoutes.delete('/:id', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const existing = await service.getByIdAndUser(id, userId);

	if (!existing) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	const deleted = await service.delete(id);

	if (!deleted) {
		return c.json({ error: 'Failed to delete agent token' }, 500);
	}

	return c.json({ success: true });
});

/**
 * POST /agent-tokens/:id/toggle
 * Toggle agent token active status
 */
agentTokenRoutes.post('/:id/toggle', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const existing = await service.getByIdAndUser(id, userId);

	if (!existing) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	const token = await service.toggle(id);

	return c.json({
		token,
		message: token?.isActive ? 'Agent token activated' : 'Agent token deactivated',
	});
});

/**
 * POST /agent-tokens/:id/add-memory
 * Add a memory to the allowed list
 */
agentTokenRoutes.post('/:id/add-memory', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const existing = await service.getByIdAndUser(id, userId);

	if (!existing) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	try {
		const body = await c.req.json();
		const { memoryId } = body as { memoryId?: string };

		if (!memoryId || typeof memoryId !== 'string') {
			return c.json({ error: 'memoryId is required' }, 400);
		}

		const token = await service.addAllowedMemory(id, memoryId);

		return c.json({ token });
	} catch (error) {
		console.error('Error adding memory:', error);
		return c.json({ error: 'Failed to add memory' }, 500);
	}
});

/**
 * POST /agent-tokens/:id/remove-memory
 * Remove a memory from the allowed list
 */
agentTokenRoutes.post('/:id/remove-memory', async (c) => {
	const id = c.req.param('id');
	const userId = c.get('userId');

	if (!c.env?.DB) {
		return c.json({ error: 'Database not available' }, 503);
	}

	if (!userId) {
		return c.json({ error: 'User ID not found' }, 401);
	}

	const service = new AgentTokenService(c.env.DB);
	const existing = await service.getByIdAndUser(id, userId);

	if (!existing) {
		return c.json({ error: 'Agent token not found' }, 404);
	}

	try {
		const body = await c.req.json();
		const { memoryId } = body as { memoryId?: string };

		if (!memoryId || typeof memoryId !== 'string') {
			return c.json({ error: 'memoryId is required' }, 400);
		}

		const token = await service.removeAllowedMemory(id, memoryId);

		if (!token) {
			return c.json({
				error: 'Cannot remove memory - token must have at least one allowed memory',
			}, 400);
		}

		return c.json({ token });
	} catch (error) {
		console.error('Error removing memory:', error);
		return c.json({ error: 'Failed to remove memory' }, 500);
	}
});

export default agentTokenRoutes;

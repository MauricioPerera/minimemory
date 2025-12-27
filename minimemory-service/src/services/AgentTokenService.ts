/**
 * Agent Token Service - MCP Access Control
 *
 * Provides:
 * - Agent token CRUD operations
 * - Dual authentication (API Key + Agent Token)
 * - Permission-based access control
 * - Memory access filtering
 * - Usage tracking
 */

import type { D1Database } from '@cloudflare/workers-types';

// Permission types
export type AgentPermission = 'read' | 'write';

// Agent token configuration
export interface AgentToken {
	id: string;
	userId: string;
	tenantId?: string;
	name: string;
	description?: string;
	allowedMemories: string[]; // ["mem_123", "mem_456"] or ["*"] for all
	permissions: AgentPermission[];
	isActive: boolean;
	lastUsedAt?: number;
	useCount: number;
	expiresAt?: number;
	createdAt: number;
	updatedAt: number;
}

// Options for creating an agent token
export interface CreateAgentTokenOptions {
	userId: string;
	tenantId?: string;
	name: string;
	description?: string;
	allowedMemories?: string[]; // Defaults to ["*"]
	permissions?: AgentPermission[]; // Defaults to ["read", "write"]
	expiresAt?: number;
}

// Options for updating an agent token
export interface UpdateAgentTokenOptions {
	name?: string;
	description?: string;
	allowedMemories?: string[];
	permissions?: AgentPermission[];
	isActive?: boolean;
	expiresAt?: number | null; // null to remove expiration
}

// Validation result for MCP authentication
export interface AgentValidationResult {
	valid: boolean;
	error?: string;
	userId?: string;
	tenantId?: string;
	agentTokenId?: string;
	agentName?: string;
	allowedMemories?: string[];
	permissions?: AgentPermission[];
	expiresAt?: number;
}

// Query options for listing tokens
export interface AgentTokenQueryOptions {
	userId?: string;
	tenantId?: string;
	activeOnly?: boolean;
	limit?: number;
	offset?: number;
}

// Query result for listing tokens
export interface AgentTokenQueryResult {
	tokens: AgentToken[];
	total: number;
	hasMore: boolean;
}

/**
 * Generate a unique agent token ID
 */
function generateAgentTokenId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 10);
	return `at_${timestamp}_${random}`;
}

/**
 * Agent Token Service class
 */
export class AgentTokenService {
	private db: D1Database;

	constructor(db: D1Database) {
		this.db = db;
	}

	/**
	 * Create a new agent token
	 */
	async create(options: CreateAgentTokenOptions): Promise<AgentToken> {
		const id = generateAgentTokenId();
		const now = Date.now();

		const token: AgentToken = {
			id,
			userId: options.userId,
			tenantId: options.tenantId,
			name: options.name,
			description: options.description,
			allowedMemories: options.allowedMemories ?? ['*'],
			permissions: options.permissions ?? ['read', 'write'],
			isActive: true,
			useCount: 0,
			expiresAt: options.expiresAt,
			createdAt: now,
			updatedAt: now,
		};

		await this.db
			.prepare(
				`INSERT INTO agent_tokens (
					id, user_id, tenant_id, name, description,
					allowed_memories, permissions, is_active,
					use_count, expires_at, created_at, updated_at
				) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
			)
			.bind(
				token.id,
				token.userId,
				token.tenantId || null,
				token.name,
				token.description || null,
				JSON.stringify(token.allowedMemories),
				JSON.stringify(token.permissions),
				token.isActive ? 1 : 0,
				token.useCount,
				token.expiresAt || null,
				token.createdAt,
				token.updatedAt
			)
			.run();

		return token;
	}

	/**
	 * Get an agent token by ID
	 */
	async get(id: string): Promise<AgentToken | null> {
		const row = await this.db
			.prepare('SELECT * FROM agent_tokens WHERE id = ?')
			.bind(id)
			.first<Record<string, unknown>>();

		if (!row) return null;
		return this.rowToToken(row);
	}

	/**
	 * Get an agent token by ID and verify ownership
	 */
	async getByIdAndUser(id: string, userId: string): Promise<AgentToken | null> {
		const row = await this.db
			.prepare('SELECT * FROM agent_tokens WHERE id = ? AND user_id = ?')
			.bind(id, userId)
			.first<Record<string, unknown>>();

		if (!row) return null;
		return this.rowToToken(row);
	}

	/**
	 * List agent tokens
	 */
	async list(options: AgentTokenQueryOptions = {}): Promise<AgentTokenQueryResult> {
		const conditions: string[] = [];
		const params: unknown[] = [];

		if (options.userId) {
			conditions.push('user_id = ?');
			params.push(options.userId);
		}

		if (options.tenantId) {
			conditions.push('tenant_id = ?');
			params.push(options.tenantId);
		}

		if (options.activeOnly) {
			conditions.push('is_active = 1');
		}

		const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
		const limit = options.limit ?? 100;
		const offset = options.offset ?? 0;

		// Get total count
		const countResult = await this.db
			.prepare(`SELECT COUNT(*) as total FROM agent_tokens ${whereClause}`)
			.bind(...params)
			.first<{ total: number }>();
		const total = countResult?.total ?? 0;

		// Get tokens
		const results = await this.db
			.prepare(
				`SELECT * FROM agent_tokens ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
			)
			.bind(...params, limit, offset)
			.all();

		const tokens = (results.results || []).map((row) => this.rowToToken(row as Record<string, unknown>));

		return {
			tokens,
			total,
			hasMore: offset + tokens.length < total,
		};
	}

	/**
	 * Update an agent token
	 */
	async update(id: string, updates: UpdateAgentTokenOptions): Promise<AgentToken | null> {
		const token = await this.get(id);
		if (!token) return null;

		const now = Date.now();
		const fields: string[] = ['updated_at = ?'];
		const params: unknown[] = [now];

		if (updates.name !== undefined) {
			fields.push('name = ?');
			params.push(updates.name);
		}

		if (updates.description !== undefined) {
			fields.push('description = ?');
			params.push(updates.description || null);
		}

		if (updates.allowedMemories !== undefined) {
			fields.push('allowed_memories = ?');
			params.push(JSON.stringify(updates.allowedMemories));
		}

		if (updates.permissions !== undefined) {
			fields.push('permissions = ?');
			params.push(JSON.stringify(updates.permissions));
		}

		if (updates.isActive !== undefined) {
			fields.push('is_active = ?');
			params.push(updates.isActive ? 1 : 0);
		}

		if (updates.expiresAt !== undefined) {
			fields.push('expires_at = ?');
			params.push(updates.expiresAt || null);
		}

		params.push(id);

		await this.db
			.prepare(`UPDATE agent_tokens SET ${fields.join(', ')} WHERE id = ?`)
			.bind(...params)
			.run();

		return this.get(id);
	}

	/**
	 * Delete an agent token
	 */
	async delete(id: string): Promise<boolean> {
		const result = await this.db.prepare('DELETE FROM agent_tokens WHERE id = ?').bind(id).run();
		return (result.meta.changes || 0) > 0;
	}

	/**
	 * Toggle agent token active status
	 */
	async toggle(id: string): Promise<AgentToken | null> {
		const token = await this.get(id);
		if (!token) return null;

		return this.update(id, { isActive: !token.isActive });
	}

	/**
	 * Add a memory to the allowed list
	 */
	async addAllowedMemory(id: string, memoryId: string): Promise<AgentToken | null> {
		const token = await this.get(id);
		if (!token) return null;

		// If already has wildcard, no need to add
		if (token.allowedMemories.includes('*')) {
			return token;
		}

		// If already in list, no change
		if (token.allowedMemories.includes(memoryId)) {
			return token;
		}

		const newAllowed = [...token.allowedMemories, memoryId];
		return this.update(id, { allowedMemories: newAllowed });
	}

	/**
	 * Remove a memory from the allowed list
	 */
	async removeAllowedMemory(id: string, memoryId: string): Promise<AgentToken | null> {
		const token = await this.get(id);
		if (!token) return null;

		const newAllowed = token.allowedMemories.filter((m) => m !== memoryId);

		// Don't allow empty list - must have at least one memory or wildcard
		if (newAllowed.length === 0) {
			return null;
		}

		return this.update(id, { allowedMemories: newAllowed });
	}

	/**
	 * Validate API key and agent token for MCP authentication
	 */
	async validate(apiKey: string, agentTokenId: string): Promise<AgentValidationResult> {
		// First, validate the API key and get user info
		const apiKeyRow = await this.db
			.prepare(
				`SELECT ak.*, u.id as uid, u.email, u.is_active as user_active
				FROM api_keys ak
				LEFT JOIN users u ON ak.user_id = u.id
				WHERE ak.key = ? AND ak.is_active = 1`
			)
			.bind(apiKey)
			.first<Record<string, unknown>>();

		if (!apiKeyRow) {
			return { valid: false, error: 'Invalid API key' };
		}

		const userId = apiKeyRow.user_id as string | null;

		// If API key is linked to a user, check if user is active
		if (userId && apiKeyRow.user_active === 0) {
			return { valid: false, error: 'User account is inactive' };
		}

		// Now validate the agent token
		const tokenRow = await this.db
			.prepare('SELECT * FROM agent_tokens WHERE id = ?')
			.bind(agentTokenId)
			.first<Record<string, unknown>>();

		if (!tokenRow) {
			return { valid: false, error: 'Invalid agent token' };
		}

		const token = this.rowToToken(tokenRow);

		// Check if token belongs to the same user (if API key has user)
		if (userId && token.userId !== userId) {
			return { valid: false, error: 'Agent token does not belong to this user' };
		}

		// Check if token is active
		if (!token.isActive) {
			return { valid: false, error: 'Agent token is inactive' };
		}

		// Check if token has expired
		if (token.expiresAt && token.expiresAt < Date.now()) {
			return { valid: false, error: 'Agent token has expired' };
		}

		return {
			valid: true,
			userId: token.userId,
			tenantId: token.tenantId,
			agentTokenId: token.id,
			agentName: token.name,
			allowedMemories: token.allowedMemories,
			permissions: token.permissions,
			expiresAt: token.expiresAt,
		};
	}

	/**
	 * Record token usage
	 */
	async recordUsage(id: string): Promise<void> {
		const now = Date.now();
		await this.db
			.prepare('UPDATE agent_tokens SET last_used_at = ?, use_count = use_count + 1 WHERE id = ?')
			.bind(now, id)
			.run();
	}

	/**
	 * Check if a token can access a specific memory
	 */
	canAccessMemory(token: AgentToken | AgentValidationResult, memoryId: string): boolean {
		const allowedMemories = 'allowedMemories' in token ? token.allowedMemories : [];
		if (!allowedMemories) return false;

		// Wildcard access
		if (allowedMemories.includes('*')) return true;

		// Specific memory access
		return allowedMemories.includes(memoryId);
	}

	/**
	 * Check if a token has a specific permission
	 */
	hasPermission(token: AgentToken | AgentValidationResult, permission: AgentPermission): boolean {
		const permissions = 'permissions' in token ? token.permissions : [];
		if (!permissions) return false;
		return permissions.includes(permission);
	}

	/**
	 * Filter a list of memory IDs to only those the token can access
	 */
	filterAllowedMemories(token: AgentToken | AgentValidationResult, memoryIds: string[]): string[] {
		const allowedMemories = 'allowedMemories' in token ? token.allowedMemories : [];
		if (!allowedMemories) return [];

		// Wildcard access
		if (allowedMemories.includes('*')) return memoryIds;

		// Filter to allowed only
		return memoryIds.filter((id) => allowedMemories.includes(id));
	}

	/**
	 * Get usage statistics for a user's tokens
	 */
	async getStats(userId: string): Promise<{
		total: number;
		active: number;
		inactive: number;
		expired: number;
		totalUseCount: number;
	}> {
		const now = Date.now();

		const result = await this.db
			.prepare(
				`SELECT
					COUNT(*) as total,
					SUM(CASE WHEN is_active = 1 AND (expires_at IS NULL OR expires_at > ?) THEN 1 ELSE 0 END) as active,
					SUM(CASE WHEN is_active = 0 THEN 1 ELSE 0 END) as inactive,
					SUM(CASE WHEN expires_at IS NOT NULL AND expires_at <= ? THEN 1 ELSE 0 END) as expired,
					SUM(use_count) as total_use_count
				FROM agent_tokens WHERE user_id = ?`
			)
			.bind(now, now, userId)
			.first<Record<string, unknown>>();

		return {
			total: (result?.total as number) || 0,
			active: (result?.active as number) || 0,
			inactive: (result?.inactive as number) || 0,
			expired: (result?.expired as number) || 0,
			totalUseCount: (result?.total_use_count as number) || 0,
		};
	}

	/**
	 * Cleanup expired tokens (optional - tokens can be kept for audit)
	 */
	async cleanupExpired(): Promise<number> {
		const now = Date.now();

		// Only delete tokens that expired more than 30 days ago
		const cutoff = now - 30 * 24 * 60 * 60 * 1000;

		const result = await this.db
			.prepare('DELETE FROM agent_tokens WHERE expires_at IS NOT NULL AND expires_at < ?')
			.bind(cutoff)
			.run();

		return result.meta.changes || 0;
	}

	/**
	 * Convert a database row to an AgentToken object
	 */
	private rowToToken(row: Record<string, unknown>): AgentToken {
		return {
			id: row.id as string,
			userId: row.user_id as string,
			tenantId: row.tenant_id as string | undefined,
			name: row.name as string,
			description: row.description as string | undefined,
			allowedMemories: JSON.parse(row.allowed_memories as string) as string[],
			permissions: JSON.parse(row.permissions as string) as AgentPermission[],
			isActive: row.is_active === 1,
			lastUsedAt: row.last_used_at as number | undefined,
			useCount: row.use_count as number,
			expiresAt: row.expires_at as number | undefined,
			createdAt: row.created_at as number,
			updatedAt: row.updated_at as number,
		};
	}
}

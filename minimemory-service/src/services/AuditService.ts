/**
 * Audit Service - Logs all operations for traceability
 *
 * Provides:
 * - Operation logging (create, read, update, delete, search, etc.)
 * - Actor tracking (user, API key, tenant)
 * - Change history with before/after states
 * - Query capabilities with filters
 */

import type { D1Database } from '@cloudflare/workers-types';

// Audit action types
export type AuditAction =
	| 'create'
	| 'read'
	| 'update'
	| 'delete'
	| 'search'
	| 'import'
	| 'export'
	| 'clear'
	| 'login'
	| 'logout'
	| 'register';

// Resource types that can be audited
export type ResourceType =
	| 'memory'
	| 'namespace'
	| 'user'
	| 'tenant'
	| 'session'
	| 'api_key'
	| 'knowledge_source';

// Audit log entry
export interface AuditEntry {
	id: string;
	timestamp: number;
	action: AuditAction;
	resourceType: ResourceType;
	resourceId?: string;
	userId?: string;
	tenantId?: string;
	namespace?: string;
	apiKeyPrefix?: string;
	ipAddress?: string;
	userAgent?: string;
	requestId?: string;
	details?: Record<string, unknown>;
	success: boolean;
	errorMessage?: string;
	durationMs?: number;
}

// Options for logging an audit entry
export interface AuditLogOptions {
	action: AuditAction;
	resourceType: ResourceType;
	resourceId?: string;
	userId?: string;
	tenantId?: string;
	namespace?: string;
	apiKey?: string;
	ipAddress?: string;
	userAgent?: string;
	requestId?: string;
	details?: Record<string, unknown>;
	success?: boolean;
	errorMessage?: string;
	durationMs?: number;
}

// Query options for retrieving audit logs
export interface AuditQueryOptions {
	action?: AuditAction;
	resourceType?: ResourceType;
	resourceId?: string;
	userId?: string;
	tenantId?: string;
	namespace?: string;
	startTime?: number;
	endTime?: number;
	success?: boolean;
	requestId?: string;
	limit?: number;
	offset?: number;
}

// Query result
export interface AuditQueryResult {
	entries: AuditEntry[];
	total: number;
	hasMore: boolean;
}

/**
 * Generate a unique ID for audit entries
 */
function generateAuditId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 10);
	return `aud_${timestamp}_${random}`;
}

/**
 * Mask API key for safe storage (show only prefix)
 */
function maskApiKey(apiKey?: string): string | undefined {
	if (!apiKey) return undefined;
	return apiKey.substring(0, 8);
}

/**
 * Audit Service class
 */
export class AuditService {
	private db: D1Database;
	private enabled: boolean;

	constructor(db: D1Database, options?: { enabled?: boolean }) {
		this.db = db;
		this.enabled = options?.enabled ?? true;
	}

	/**
	 * Log an audit entry
	 */
	async log(options: AuditLogOptions): Promise<string | null> {
		if (!this.enabled) return null;

		const id = generateAuditId();
		const timestamp = Date.now();

		try {
			await this.db
				.prepare(
					`INSERT INTO audit_log (
						id, timestamp, action, resource_type, resource_id,
						user_id, tenant_id, namespace, api_key_prefix,
						ip_address, user_agent, request_id,
						details, success, error_message, duration_ms
					) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
				)
				.bind(
					id,
					timestamp,
					options.action,
					options.resourceType,
					options.resourceId || null,
					options.userId || null,
					options.tenantId || null,
					options.namespace || null,
					maskApiKey(options.apiKey),
					options.ipAddress || null,
					options.userAgent || null,
					options.requestId || null,
					options.details ? JSON.stringify(options.details) : null,
					options.success !== false ? 1 : 0,
					options.errorMessage || null,
					options.durationMs || null
				)
				.run();

			return id;
		} catch (error) {
			// Log failure shouldn't break the main operation
			console.error('Failed to write audit log:', error);
			return null;
		}
	}

	/**
	 * Query audit logs
	 */
	async query(options: AuditQueryOptions = {}): Promise<AuditQueryResult> {
		const conditions: string[] = [];
		const params: unknown[] = [];

		if (options.action) {
			conditions.push('action = ?');
			params.push(options.action);
		}

		if (options.resourceType) {
			conditions.push('resource_type = ?');
			params.push(options.resourceType);
		}

		if (options.resourceId) {
			conditions.push('resource_id = ?');
			params.push(options.resourceId);
		}

		if (options.userId) {
			conditions.push('user_id = ?');
			params.push(options.userId);
		}

		if (options.tenantId) {
			conditions.push('tenant_id = ?');
			params.push(options.tenantId);
		}

		if (options.namespace) {
			conditions.push('namespace = ?');
			params.push(options.namespace);
		}

		if (options.startTime) {
			conditions.push('timestamp >= ?');
			params.push(options.startTime);
		}

		if (options.endTime) {
			conditions.push('timestamp <= ?');
			params.push(options.endTime);
		}

		if (options.success !== undefined) {
			conditions.push('success = ?');
			params.push(options.success ? 1 : 0);
		}

		if (options.requestId) {
			conditions.push('request_id = ?');
			params.push(options.requestId);
		}

		const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
		const limit = options.limit || 100;
		const offset = options.offset || 0;

		// Get total count
		const countQuery = `SELECT COUNT(*) as total FROM audit_log ${whereClause}`;
		const countResult = await this.db.prepare(countQuery).bind(...params).first<{ total: number }>();
		const total = countResult?.total || 0;

		// Get entries
		const query = `
			SELECT * FROM audit_log
			${whereClause}
			ORDER BY timestamp DESC
			LIMIT ? OFFSET ?
		`;
		const results = await this.db
			.prepare(query)
			.bind(...params, limit, offset)
			.all();

		const entries: AuditEntry[] = (results.results || []).map((row: Record<string, unknown>) => ({
			id: row.id as string,
			timestamp: row.timestamp as number,
			action: row.action as AuditAction,
			resourceType: row.resource_type as ResourceType,
			resourceId: row.resource_id as string | undefined,
			userId: row.user_id as string | undefined,
			tenantId: row.tenant_id as string | undefined,
			namespace: row.namespace as string | undefined,
			apiKeyPrefix: row.api_key_prefix as string | undefined,
			ipAddress: row.ip_address as string | undefined,
			userAgent: row.user_agent as string | undefined,
			requestId: row.request_id as string | undefined,
			details: row.details ? JSON.parse(row.details as string) : undefined,
			success: row.success === 1,
			errorMessage: row.error_message as string | undefined,
			durationMs: row.duration_ms as number | undefined,
		}));

		return {
			entries,
			total,
			hasMore: offset + entries.length < total,
		};
	}

	/**
	 * Get audit log by ID
	 */
	async getById(id: string): Promise<AuditEntry | null> {
		const row = await this.db
			.prepare('SELECT * FROM audit_log WHERE id = ?')
			.bind(id)
			.first<Record<string, unknown>>();

		if (!row) return null;

		return {
			id: row.id as string,
			timestamp: row.timestamp as number,
			action: row.action as AuditAction,
			resourceType: row.resource_type as ResourceType,
			resourceId: row.resource_id as string | undefined,
			userId: row.user_id as string | undefined,
			tenantId: row.tenant_id as string | undefined,
			namespace: row.namespace as string | undefined,
			apiKeyPrefix: row.api_key_prefix as string | undefined,
			ipAddress: row.ip_address as string | undefined,
			userAgent: row.user_agent as string | undefined,
			requestId: row.request_id as string | undefined,
			details: row.details ? JSON.parse(row.details as string) : undefined,
			success: row.success === 1,
			errorMessage: row.error_message as string | undefined,
			durationMs: row.duration_ms as number | undefined,
		};
	}

	/**
	 * Get history for a specific resource
	 */
	async getResourceHistory(
		resourceType: ResourceType,
		resourceId: string,
		limit: number = 50
	): Promise<AuditEntry[]> {
		const result = await this.query({
			resourceType,
			resourceId,
			limit,
		});
		return result.entries;
	}

	/**
	 * Get activity for a specific user
	 */
	async getUserActivity(
		userId: string,
		options?: { startTime?: number; endTime?: number; limit?: number }
	): Promise<AuditEntry[]> {
		const result = await this.query({
			userId,
			startTime: options?.startTime,
			endTime: options?.endTime,
			limit: options?.limit || 100,
		});
		return result.entries;
	}

	/**
	 * Get failed operations
	 */
	async getFailures(
		options?: { tenantId?: string; namespace?: string; limit?: number }
	): Promise<AuditEntry[]> {
		const result = await this.query({
			success: false,
			tenantId: options?.tenantId,
			namespace: options?.namespace,
			limit: options?.limit || 50,
		});
		return result.entries;
	}

	/**
	 * Get audit stats for a time period
	 */
	async getStats(
		tenantId?: string,
		options?: { startTime?: number; endTime?: number }
	): Promise<{
		totalOperations: number;
		byAction: Record<string, number>;
		byResource: Record<string, number>;
		successRate: number;
		avgDurationMs: number;
	}> {
		const conditions: string[] = [];
		const params: unknown[] = [];

		if (tenantId) {
			conditions.push('tenant_id = ?');
			params.push(tenantId);
		}

		if (options?.startTime) {
			conditions.push('timestamp >= ?');
			params.push(options.startTime);
		}

		if (options?.endTime) {
			conditions.push('timestamp <= ?');
			params.push(options.endTime);
		}

		const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';

		// Total and success rate
		const statsQuery = `
			SELECT
				COUNT(*) as total,
				SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
				AVG(duration_ms) as avg_duration
			FROM audit_log ${whereClause}
		`;
		const stats = await this.db.prepare(statsQuery).bind(...params).first<{
			total: number;
			successful: number;
			avg_duration: number | null;
		}>();

		// By action
		const actionQuery = `
			SELECT action, COUNT(*) as count
			FROM audit_log ${whereClause}
			GROUP BY action
		`;
		const actionResults = await this.db.prepare(actionQuery).bind(...params).all();
		const byAction: Record<string, number> = {};
		for (const row of actionResults.results || []) {
			byAction[(row as { action: string }).action] = (row as { count: number }).count;
		}

		// By resource type
		const resourceQuery = `
			SELECT resource_type, COUNT(*) as count
			FROM audit_log ${whereClause}
			GROUP BY resource_type
		`;
		const resourceResults = await this.db.prepare(resourceQuery).bind(...params).all();
		const byResource: Record<string, number> = {};
		for (const row of resourceResults.results || []) {
			byResource[(row as { resource_type: string }).resource_type] = (row as { count: number }).count;
		}

		return {
			totalOperations: stats?.total || 0,
			byAction,
			byResource,
			successRate: stats?.total ? ((stats.successful || 0) / stats.total) * 100 : 100,
			avgDurationMs: stats?.avg_duration || 0,
		};
	}

	/**
	 * Clean up old audit logs
	 */
	async cleanup(retentionDays: number = 90): Promise<number> {
		const cutoffTime = Date.now() - retentionDays * 24 * 60 * 60 * 1000;

		const result = await this.db
			.prepare('DELETE FROM audit_log WHERE timestamp < ?')
			.bind(cutoffTime)
			.run();

		return result.meta.changes || 0;
	}
}

/**
 * Create an audit service with a request context
 */
export function createAuditLogger(
	db: D1Database,
	context: {
		userId?: string;
		tenantId?: string;
		namespace?: string;
		apiKey?: string;
		ipAddress?: string;
		userAgent?: string;
		requestId?: string;
	}
) {
	const service = new AuditService(db);

	return {
		/**
		 * Log a memory operation
		 */
		async logMemory(
			action: 'create' | 'read' | 'update' | 'delete' | 'search',
			memoryId: string | undefined,
			details?: Record<string, unknown>,
			options?: { success?: boolean; errorMessage?: string; durationMs?: number }
		): Promise<string | null> {
			return service.log({
				action,
				resourceType: 'memory',
				resourceId: memoryId,
				...context,
				details,
				...options,
			});
		},

		/**
		 * Log a namespace operation
		 */
		async logNamespace(
			action: 'create' | 'read' | 'update' | 'delete' | 'clear',
			namespaceName: string,
			details?: Record<string, unknown>,
			options?: { success?: boolean; errorMessage?: string; durationMs?: number }
		): Promise<string | null> {
			return service.log({
				action,
				resourceType: 'namespace',
				resourceId: namespaceName,
				...context,
				details,
				...options,
			});
		},

		/**
		 * Log a bulk operation
		 */
		async logBulk(
			action: 'import' | 'export' | 'clear',
			details?: Record<string, unknown>,
			options?: { success?: boolean; errorMessage?: string; durationMs?: number }
		): Promise<string | null> {
			return service.log({
				action,
				resourceType: 'memory',
				...context,
				details,
				...options,
			});
		},

		/**
		 * Log an auth operation
		 */
		async logAuth(
			action: 'login' | 'logout' | 'register',
			userId: string,
			details?: Record<string, unknown>,
			options?: { success?: boolean; errorMessage?: string }
		): Promise<string | null> {
			return service.log({
				action,
				resourceType: 'user',
				resourceId: userId,
				...context,
				userId,
				details,
				...options,
			});
		},

		/**
		 * Access the underlying service for queries
		 */
		service,
	};
}

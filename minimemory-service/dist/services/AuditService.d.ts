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
export type AuditAction = 'create' | 'read' | 'update' | 'delete' | 'search' | 'import' | 'export' | 'clear' | 'login' | 'logout' | 'register';
export type ResourceType = 'memory' | 'namespace' | 'user' | 'tenant' | 'session' | 'api_key' | 'knowledge_source';
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
export interface AuditQueryResult {
    entries: AuditEntry[];
    total: number;
    hasMore: boolean;
}
/**
 * Audit Service class
 */
export declare class AuditService {
    private db;
    private enabled;
    constructor(db: D1Database, options?: {
        enabled?: boolean;
    });
    /**
     * Log an audit entry
     */
    log(options: AuditLogOptions): Promise<string | null>;
    /**
     * Query audit logs
     */
    query(options?: AuditQueryOptions): Promise<AuditQueryResult>;
    /**
     * Get audit log by ID
     */
    getById(id: string): Promise<AuditEntry | null>;
    /**
     * Get history for a specific resource
     */
    getResourceHistory(resourceType: ResourceType, resourceId: string, limit?: number): Promise<AuditEntry[]>;
    /**
     * Get activity for a specific user
     */
    getUserActivity(userId: string, options?: {
        startTime?: number;
        endTime?: number;
        limit?: number;
    }): Promise<AuditEntry[]>;
    /**
     * Get failed operations
     */
    getFailures(options?: {
        tenantId?: string;
        namespace?: string;
        limit?: number;
    }): Promise<AuditEntry[]>;
    /**
     * Get audit stats for a time period
     */
    getStats(tenantId?: string, options?: {
        startTime?: number;
        endTime?: number;
    }): Promise<{
        totalOperations: number;
        byAction: Record<string, number>;
        byResource: Record<string, number>;
        successRate: number;
        avgDurationMs: number;
    }>;
    /**
     * Clean up old audit logs
     */
    cleanup(retentionDays?: number): Promise<number>;
}
/**
 * Create an audit service with a request context
 */
export declare function createAuditLogger(db: D1Database, context: {
    userId?: string;
    tenantId?: string;
    namespace?: string;
    apiKey?: string;
    ipAddress?: string;
    userAgent?: string;
    requestId?: string;
}): {
    /**
     * Log a memory operation
     */
    logMemory(action: "create" | "read" | "update" | "delete" | "search", memoryId: string | undefined, details?: Record<string, unknown>, options?: {
        success?: boolean;
        errorMessage?: string;
        durationMs?: number;
    }): Promise<string | null>;
    /**
     * Log a namespace operation
     */
    logNamespace(action: "create" | "read" | "update" | "delete" | "clear", namespaceName: string, details?: Record<string, unknown>, options?: {
        success?: boolean;
        errorMessage?: string;
        durationMs?: number;
    }): Promise<string | null>;
    /**
     * Log a bulk operation
     */
    logBulk(action: "import" | "export" | "clear", details?: Record<string, unknown>, options?: {
        success?: boolean;
        errorMessage?: string;
        durationMs?: number;
    }): Promise<string | null>;
    /**
     * Log an auth operation
     */
    logAuth(action: "login" | "logout" | "register", userId: string, details?: Record<string, unknown>, options?: {
        success?: boolean;
        errorMessage?: string;
    }): Promise<string | null>;
    /**
     * Access the underlying service for queries
     */
    service: AuditService;
};
//# sourceMappingURL=AuditService.d.ts.map
/**
 * Webhook Service - Event notification system
 *
 * Provides:
 * - Webhook CRUD operations
 * - HMAC-SHA256 signature for payload verification
 * - Async delivery with retry logic
 * - Delivery tracking and history
 */
import type { D1Database, ExecutionContext } from '@cloudflare/workers-types';
export type WebhookEventType = 'memory.remembered' | 'memory.forgotten' | 'memory.updated' | 'knowledge.ingested' | 'knowledge.deleted';
export declare const WEBHOOK_EVENTS: WebhookEventType[];
export interface Webhook {
    id: string;
    namespace: string;
    tenantId?: string;
    url: string;
    secret: string;
    events: WebhookEventType[];
    isActive: boolean;
    description?: string;
    maxRetries: number;
    retryBackoffMs: number;
    successCount: number;
    failureCount: number;
    lastTriggeredAt?: number;
    createdAt: number;
    updatedAt: number;
}
export type DeliveryStatus = 'pending' | 'success' | 'failed' | 'retrying';
export interface WebhookDelivery {
    id: string;
    webhookId: string;
    eventType: WebhookEventType;
    eventId: string;
    payload: string;
    status: DeliveryStatus;
    attemptCount: number;
    nextRetryAt?: number;
    responseStatus?: number;
    responseBody?: string;
    errorMessage?: string;
    createdAt: number;
    completedAt?: number;
}
export interface WebhookEvent {
    id: string;
    type: WebhookEventType;
    timestamp: number;
    namespace: string;
    tenantId?: string;
    data: Record<string, unknown>;
}
export interface CreateWebhookOptions {
    namespace: string;
    tenantId?: string;
    url: string;
    events: WebhookEventType[];
    description?: string;
    maxRetries?: number;
    retryBackoffMs?: number;
}
export interface UpdateWebhookOptions {
    url?: string;
    events?: WebhookEventType[];
    isActive?: boolean;
    description?: string;
    maxRetries?: number;
    retryBackoffMs?: number;
}
export interface DeliveryQueryOptions {
    webhookId?: string;
    status?: DeliveryStatus;
    eventType?: WebhookEventType;
    limit?: number;
    offset?: number;
}
export interface DeliveryQueryResult {
    deliveries: WebhookDelivery[];
    total: number;
    hasMore: boolean;
}
/**
 * Webhook Service class
 */
export declare class WebhookService {
    private db;
    private ctx?;
    constructor(db: D1Database, ctx?: ExecutionContext);
    /**
     * Create a new webhook
     */
    create(options: CreateWebhookOptions): Promise<Webhook>;
    /**
     * Get a webhook by ID
     */
    get(id: string): Promise<Webhook | null>;
    /**
     * List webhooks for a namespace
     */
    list(namespace: string, options?: {
        activeOnly?: boolean;
        limit?: number;
        offset?: number;
    }): Promise<{
        webhooks: Webhook[];
        total: number;
        hasMore: boolean;
    }>;
    /**
     * Update a webhook
     */
    update(id: string, updates: UpdateWebhookOptions): Promise<Webhook | null>;
    /**
     * Delete a webhook
     */
    delete(id: string): Promise<boolean>;
    /**
     * Rotate the secret for a webhook
     */
    rotateSecret(id: string): Promise<string | null>;
    /**
     * Trigger webhooks for an event (async, non-blocking)
     */
    trigger(namespace: string, eventType: WebhookEventType, data: Record<string, unknown>, tenantId?: string): Promise<void>;
    /**
     * Deliver a webhook event
     */
    private deliver;
    /**
     * Test a webhook by sending a test event
     */
    test(id: string): Promise<{
        success: boolean;
        status?: number;
        error?: string;
    }>;
    /**
     * Get delivery history for a webhook
     */
    getDeliveries(options?: DeliveryQueryOptions): Promise<DeliveryQueryResult>;
    /**
     * Cleanup old deliveries
     */
    cleanupDeliveries(retentionDays?: number): Promise<number>;
    /**
     * Convert a database row to a Webhook object
     */
    private rowToWebhook;
    /**
     * Convert a database row to a WebhookDelivery object
     */
    private rowToDelivery;
}
/**
 * Create a webhook trigger helper for use in API handlers
 */
export declare function createWebhookTrigger(db: D1Database, ctx?: ExecutionContext): (namespace: string, event: WebhookEventType, data: Record<string, unknown>, tenantId?: string) => void;
//# sourceMappingURL=WebhookService.d.ts.map
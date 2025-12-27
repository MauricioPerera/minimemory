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

// Supported webhook events
export type WebhookEventType =
	| 'memory.remembered'
	| 'memory.forgotten'
	| 'memory.updated'
	| 'knowledge.ingested'
	| 'knowledge.deleted';

// All available events
export const WEBHOOK_EVENTS: WebhookEventType[] = [
	'memory.remembered',
	'memory.forgotten',
	'memory.updated',
	'knowledge.ingested',
	'knowledge.deleted',
];

// Webhook configuration
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

// Delivery status
export type DeliveryStatus = 'pending' | 'success' | 'failed' | 'retrying';

// Webhook delivery record
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

// Event payload structure
export interface WebhookEvent {
	id: string;
	type: WebhookEventType;
	timestamp: number;
	namespace: string;
	tenantId?: string;
	data: Record<string, unknown>;
}

// Options for creating a webhook
export interface CreateWebhookOptions {
	namespace: string;
	tenantId?: string;
	url: string;
	events: WebhookEventType[];
	description?: string;
	maxRetries?: number;
	retryBackoffMs?: number;
}

// Options for updating a webhook
export interface UpdateWebhookOptions {
	url?: string;
	events?: WebhookEventType[];
	isActive?: boolean;
	description?: string;
	maxRetries?: number;
	retryBackoffMs?: number;
}

// Query options for deliveries
export interface DeliveryQueryOptions {
	webhookId?: string;
	status?: DeliveryStatus;
	eventType?: WebhookEventType;
	limit?: number;
	offset?: number;
}

// Query result for deliveries
export interface DeliveryQueryResult {
	deliveries: WebhookDelivery[];
	total: number;
	hasMore: boolean;
}

/**
 * Generate a unique webhook ID
 */
function generateWebhookId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 10);
	return `wh_${timestamp}_${random}`;
}

/**
 * Generate a unique delivery ID
 */
function generateDeliveryId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 10);
	return `del_${timestamp}_${random}`;
}

/**
 * Generate a unique event ID
 */
function generateEventId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 10);
	return `evt_${timestamp}_${random}`;
}

/**
 * Generate a random secret for HMAC signing
 */
function generateSecret(): string {
	const array = new Uint8Array(32);
	crypto.getRandomValues(array);
	return Array.from(array)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

/**
 * Sign a payload using HMAC-SHA256
 */
async function signPayload(payload: string, secret: string): Promise<string> {
	const encoder = new TextEncoder();
	const keyData = encoder.encode(secret);
	const payloadData = encoder.encode(payload);

	const key = await crypto.subtle.importKey('raw', keyData, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);

	const signature = await crypto.subtle.sign('HMAC', key, payloadData);
	const signatureArray = new Uint8Array(signature);
	return Array.from(signatureArray)
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

/**
 * Webhook Service class
 */
export class WebhookService {
	private db: D1Database;
	private ctx?: ExecutionContext;

	constructor(db: D1Database, ctx?: ExecutionContext) {
		this.db = db;
		this.ctx = ctx;
	}

	/**
	 * Create a new webhook
	 */
	async create(options: CreateWebhookOptions): Promise<Webhook> {
		const id = generateWebhookId();
		const secret = generateSecret();
		const now = Date.now();

		const webhook: Webhook = {
			id,
			namespace: options.namespace,
			tenantId: options.tenantId,
			url: options.url,
			secret,
			events: options.events,
			isActive: true,
			description: options.description,
			maxRetries: options.maxRetries ?? 3,
			retryBackoffMs: options.retryBackoffMs ?? 1000,
			successCount: 0,
			failureCount: 0,
			createdAt: now,
			updatedAt: now,
		};

		await this.db
			.prepare(
				`INSERT INTO webhooks (
					id, namespace, tenant_id, url, secret, events,
					is_active, description, max_retries, retry_backoff_ms,
					success_count, failure_count, created_at, updated_at
				) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
			)
			.bind(
				webhook.id,
				webhook.namespace,
				webhook.tenantId || null,
				webhook.url,
				webhook.secret,
				JSON.stringify(webhook.events),
				webhook.isActive ? 1 : 0,
				webhook.description || null,
				webhook.maxRetries,
				webhook.retryBackoffMs,
				webhook.successCount,
				webhook.failureCount,
				webhook.createdAt,
				webhook.updatedAt
			)
			.run();

		return webhook;
	}

	/**
	 * Get a webhook by ID
	 */
	async get(id: string): Promise<Webhook | null> {
		const row = await this.db.prepare('SELECT * FROM webhooks WHERE id = ?').bind(id).first<Record<string, unknown>>();

		if (!row) return null;
		return this.rowToWebhook(row);
	}

	/**
	 * List webhooks for a namespace
	 */
	async list(
		namespace: string,
		options?: { activeOnly?: boolean; limit?: number; offset?: number }
	): Promise<{ webhooks: Webhook[]; total: number; hasMore: boolean }> {
		const conditions = ['namespace = ?'];
		const params: unknown[] = [namespace];

		if (options?.activeOnly) {
			conditions.push('is_active = 1');
		}

		const whereClause = conditions.join(' AND ');
		const limit = options?.limit ?? 100;
		const offset = options?.offset ?? 0;

		// Get total count
		const countResult = await this.db
			.prepare(`SELECT COUNT(*) as total FROM webhooks WHERE ${whereClause}`)
			.bind(...params)
			.first<{ total: number }>();
		const total = countResult?.total ?? 0;

		// Get webhooks
		const results = await this.db
			.prepare(
				`SELECT * FROM webhooks WHERE ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
			)
			.bind(...params, limit, offset)
			.all();

		const webhooks = (results.results || []).map((row) => this.rowToWebhook(row as Record<string, unknown>));

		return {
			webhooks,
			total,
			hasMore: offset + webhooks.length < total,
		};
	}

	/**
	 * Update a webhook
	 */
	async update(id: string, updates: UpdateWebhookOptions): Promise<Webhook | null> {
		const webhook = await this.get(id);
		if (!webhook) return null;

		const now = Date.now();
		const fields: string[] = ['updated_at = ?'];
		const params: unknown[] = [now];

		if (updates.url !== undefined) {
			fields.push('url = ?');
			params.push(updates.url);
		}

		if (updates.events !== undefined) {
			fields.push('events = ?');
			params.push(JSON.stringify(updates.events));
		}

		if (updates.isActive !== undefined) {
			fields.push('is_active = ?');
			params.push(updates.isActive ? 1 : 0);
		}

		if (updates.description !== undefined) {
			fields.push('description = ?');
			params.push(updates.description || null);
		}

		if (updates.maxRetries !== undefined) {
			fields.push('max_retries = ?');
			params.push(updates.maxRetries);
		}

		if (updates.retryBackoffMs !== undefined) {
			fields.push('retry_backoff_ms = ?');
			params.push(updates.retryBackoffMs);
		}

		params.push(id);

		await this.db.prepare(`UPDATE webhooks SET ${fields.join(', ')} WHERE id = ?`).bind(...params).run();

		return this.get(id);
	}

	/**
	 * Delete a webhook
	 */
	async delete(id: string): Promise<boolean> {
		const result = await this.db.prepare('DELETE FROM webhooks WHERE id = ?').bind(id).run();
		return (result.meta.changes || 0) > 0;
	}

	/**
	 * Rotate the secret for a webhook
	 */
	async rotateSecret(id: string): Promise<string | null> {
		const webhook = await this.get(id);
		if (!webhook) return null;

		const newSecret = generateSecret();
		const now = Date.now();

		await this.db.prepare('UPDATE webhooks SET secret = ?, updated_at = ? WHERE id = ?').bind(newSecret, now, id).run();

		return newSecret;
	}

	/**
	 * Trigger webhooks for an event (async, non-blocking)
	 */
	async trigger(namespace: string, eventType: WebhookEventType, data: Record<string, unknown>, tenantId?: string): Promise<void> {
		// Find all active webhooks that subscribe to this event
		const result = await this.db
			.prepare('SELECT * FROM webhooks WHERE namespace = ? AND is_active = 1')
			.bind(namespace)
			.all();

		const webhooks = (result.results || [])
			.map((row) => this.rowToWebhook(row as Record<string, unknown>))
			.filter((wh) => wh.events.includes(eventType));

		if (webhooks.length === 0) return;

		// Create event payload
		const event: WebhookEvent = {
			id: generateEventId(),
			type: eventType,
			timestamp: Date.now(),
			namespace,
			tenantId,
			data,
		};

		const payloadString = JSON.stringify(event);

		// Dispatch to each webhook
		for (const webhook of webhooks) {
			const deliveryId = generateDeliveryId();

			// Create delivery record
			await this.db
				.prepare(
					`INSERT INTO webhook_deliveries (
						id, webhook_id, event_type, event_id, payload, status, attempt_count, created_at
					) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
				)
				.bind(deliveryId, webhook.id, eventType, event.id, payloadString, 'pending', 0, Date.now())
				.run();

			// Dispatch async using waitUntil if available
			if (this.ctx) {
				this.ctx.waitUntil(this.deliver(webhook, deliveryId, payloadString));
			} else {
				// Fallback: fire and forget
				this.deliver(webhook, deliveryId, payloadString).catch(console.error);
			}
		}

		// Update last_triggered_at for all triggered webhooks
		const webhookIds = webhooks.map((wh) => wh.id);
		await this.db
			.prepare(
				`UPDATE webhooks SET last_triggered_at = ? WHERE id IN (${webhookIds.map(() => '?').join(', ')})`
			)
			.bind(Date.now(), ...webhookIds)
			.run();
	}

	/**
	 * Deliver a webhook event
	 */
	private async deliver(webhook: Webhook, deliveryId: string, payload: string): Promise<void> {
		let attemptCount = 0;
		let lastError: string | undefined;
		let responseStatus: number | undefined;
		let responseBody: string | undefined;

		while (attemptCount <= webhook.maxRetries) {
			attemptCount++;

			// Update delivery status
			await this.db
				.prepare('UPDATE webhook_deliveries SET status = ?, attempt_count = ? WHERE id = ?')
				.bind(attemptCount === 1 ? 'pending' : 'retrying', attemptCount, deliveryId)
				.run();

			try {
				// Sign the payload
				const signature = await signPayload(payload, webhook.secret);

				// Make the request
				const response = await fetch(webhook.url, {
					method: 'POST',
					headers: {
						'Content-Type': 'application/json',
						'X-Signature': `sha256=${signature}`,
						'X-Webhook-Id': webhook.id,
						'X-Event-Id': JSON.parse(payload).id,
						'X-Event-Type': JSON.parse(payload).type,
						'User-Agent': 'minimemory-webhooks/1.0',
					},
					body: payload,
				});

				responseStatus = response.status;
				responseBody = await response.text().catch(() => '');

				if (response.ok) {
					// Success!
					await this.db
						.prepare(
							`UPDATE webhook_deliveries
							SET status = 'success', response_status = ?, response_body = ?, completed_at = ?
							WHERE id = ?`
						)
						.bind(responseStatus, responseBody.substring(0, 1000), Date.now(), deliveryId)
						.run();

					// Update success count
					await this.db.prepare('UPDATE webhooks SET success_count = success_count + 1 WHERE id = ?').bind(webhook.id).run();

					return;
				}

				// Non-2xx response
				lastError = `HTTP ${response.status}: ${responseBody.substring(0, 200)}`;
			} catch (error) {
				lastError = error instanceof Error ? error.message : 'Unknown error';
			}

			// If we have more retries, wait before next attempt
			if (attemptCount <= webhook.maxRetries) {
				const backoff = webhook.retryBackoffMs * Math.pow(2, attemptCount - 1);
				const nextRetryAt = Date.now() + backoff;

				await this.db
					.prepare('UPDATE webhook_deliveries SET next_retry_at = ? WHERE id = ?')
					.bind(nextRetryAt, deliveryId)
					.run();

				await new Promise((resolve) => setTimeout(resolve, backoff));
			}
		}

		// All retries exhausted
		await this.db
			.prepare(
				`UPDATE webhook_deliveries
				SET status = 'failed', response_status = ?, error_message = ?, completed_at = ?
				WHERE id = ?`
			)
			.bind(responseStatus || null, lastError || 'Unknown error', Date.now(), deliveryId)
			.run();

		// Update failure count
		await this.db.prepare('UPDATE webhooks SET failure_count = failure_count + 1 WHERE id = ?').bind(webhook.id).run();
	}

	/**
	 * Test a webhook by sending a test event
	 */
	async test(id: string): Promise<{ success: boolean; status?: number; error?: string }> {
		const webhook = await this.get(id);
		if (!webhook) {
			return { success: false, error: 'Webhook not found' };
		}

		const testEvent: WebhookEvent = {
			id: generateEventId(),
			type: 'memory.remembered',
			timestamp: Date.now(),
			namespace: webhook.namespace,
			tenantId: webhook.tenantId,
			data: {
				test: true,
				message: 'This is a test webhook delivery',
			},
		};

		const payload = JSON.stringify(testEvent);

		try {
			const signature = await signPayload(payload, webhook.secret);

			const response = await fetch(webhook.url, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'X-Signature': `sha256=${signature}`,
					'X-Webhook-Id': webhook.id,
					'X-Event-Id': testEvent.id,
					'X-Event-Type': testEvent.type,
					'User-Agent': 'minimemory-webhooks/1.0 (test)',
				},
				body: payload,
			});

			if (response.ok) {
				return { success: true, status: response.status };
			}

			return {
				success: false,
				status: response.status,
				error: `HTTP ${response.status}`,
			};
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : 'Unknown error',
			};
		}
	}

	/**
	 * Get delivery history for a webhook
	 */
	async getDeliveries(options: DeliveryQueryOptions = {}): Promise<DeliveryQueryResult> {
		const conditions: string[] = [];
		const params: unknown[] = [];

		if (options.webhookId) {
			conditions.push('webhook_id = ?');
			params.push(options.webhookId);
		}

		if (options.status) {
			conditions.push('status = ?');
			params.push(options.status);
		}

		if (options.eventType) {
			conditions.push('event_type = ?');
			params.push(options.eventType);
		}

		const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
		const limit = options.limit ?? 100;
		const offset = options.offset ?? 0;

		// Get total count
		const countResult = await this.db
			.prepare(`SELECT COUNT(*) as total FROM webhook_deliveries ${whereClause}`)
			.bind(...params)
			.first<{ total: number }>();
		const total = countResult?.total ?? 0;

		// Get deliveries
		const results = await this.db
			.prepare(
				`SELECT * FROM webhook_deliveries ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
			)
			.bind(...params, limit, offset)
			.all();

		const deliveries = (results.results || []).map((row) => this.rowToDelivery(row as Record<string, unknown>));

		return {
			deliveries,
			total,
			hasMore: offset + deliveries.length < total,
		};
	}

	/**
	 * Cleanup old deliveries
	 */
	async cleanupDeliveries(retentionDays: number = 7): Promise<number> {
		const cutoffTime = Date.now() - retentionDays * 24 * 60 * 60 * 1000;

		const result = await this.db
			.prepare('DELETE FROM webhook_deliveries WHERE created_at < ? AND status IN (?, ?)')
			.bind(cutoffTime, 'success', 'failed')
			.run();

		return result.meta.changes || 0;
	}

	/**
	 * Convert a database row to a Webhook object
	 */
	private rowToWebhook(row: Record<string, unknown>): Webhook {
		return {
			id: row.id as string,
			namespace: row.namespace as string,
			tenantId: row.tenant_id as string | undefined,
			url: row.url as string,
			secret: row.secret as string,
			events: JSON.parse(row.events as string) as WebhookEventType[],
			isActive: row.is_active === 1,
			description: row.description as string | undefined,
			maxRetries: row.max_retries as number,
			retryBackoffMs: row.retry_backoff_ms as number,
			successCount: row.success_count as number,
			failureCount: row.failure_count as number,
			lastTriggeredAt: row.last_triggered_at as number | undefined,
			createdAt: row.created_at as number,
			updatedAt: row.updated_at as number,
		};
	}

	/**
	 * Convert a database row to a WebhookDelivery object
	 */
	private rowToDelivery(row: Record<string, unknown>): WebhookDelivery {
		return {
			id: row.id as string,
			webhookId: row.webhook_id as string,
			eventType: row.event_type as WebhookEventType,
			eventId: row.event_id as string,
			payload: row.payload as string,
			status: row.status as DeliveryStatus,
			attemptCount: row.attempt_count as number,
			nextRetryAt: row.next_retry_at as number | undefined,
			responseStatus: row.response_status as number | undefined,
			responseBody: row.response_body as string | undefined,
			errorMessage: row.error_message as string | undefined,
			createdAt: row.created_at as number,
			completedAt: row.completed_at as number | undefined,
		};
	}
}

/**
 * Create a webhook trigger helper for use in API handlers
 */
export function createWebhookTrigger(
	db: D1Database,
	ctx?: ExecutionContext
): (namespace: string, event: WebhookEventType, data: Record<string, unknown>, tenantId?: string) => void {
	const service = new WebhookService(db, ctx);

	return (namespace: string, event: WebhookEventType, data: Record<string, unknown>, tenantId?: string) => {
		// Fire and forget - don't await
		service.trigger(namespace, event, data, tenantId).catch((error) => {
			console.error('Webhook trigger failed:', error);
		});
	};
}

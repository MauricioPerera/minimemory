/**
 * Webhook API Routes
 *
 * Endpoints for managing webhooks and viewing delivery history
 */
import { Hono } from 'hono';
import { WebhookService, WEBHOOK_EVENTS, } from '../services/WebhookService.js';
const webhookRoutes = new Hono();
/**
 * Helper to get namespace from header
 */
function getNamespace(c) {
    return c.req.header('X-Namespace') || 'default';
}
/**
 * Helper to get tenant ID from header
 */
function getTenantId(c) {
    return c.req.header('X-Tenant-Id');
}
/**
 * Validate webhook events
 */
function validateEvents(events) {
    if (!Array.isArray(events))
        return null;
    if (events.length === 0)
        return null;
    for (const event of events) {
        if (typeof event !== 'string')
            return null;
        if (!WEBHOOK_EVENTS.includes(event))
            return null;
    }
    return events;
}
/**
 * Validate URL format
 */
function isValidUrl(url) {
    try {
        const parsed = new URL(url);
        return parsed.protocol === 'https:' || parsed.protocol === 'http:';
    }
    catch {
        return false;
    }
}
// ============ Webhook Management ============
/**
 * GET /webhooks/events
 * List all available webhook events
 */
webhookRoutes.get('/events', (c) => {
    return c.json({
        events: WEBHOOK_EVENTS.map((event) => ({
            type: event,
            description: getEventDescription(event),
        })),
    });
});
/**
 * GET /webhooks
 * List all webhooks for the namespace
 */
webhookRoutes.get('/', async (c) => {
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const activeOnly = c.req.query('active') === 'true';
    const limit = parseInt(c.req.query('limit') || '100', 10);
    const offset = parseInt(c.req.query('offset') || '0', 10);
    const result = await service.list(namespace, { activeOnly, limit, offset });
    // Don't expose secrets in list
    const webhooks = result.webhooks.map((wh) => ({
        ...wh,
        secret: undefined,
        secretPrefix: wh.secret.substring(0, 8) + '...',
    }));
    return c.json({
        webhooks,
        total: result.total,
        hasMore: result.hasMore,
    });
});
/**
 * POST /webhooks
 * Create a new webhook
 */
webhookRoutes.post('/', async (c) => {
    const namespace = getNamespace(c);
    const tenantId = getTenantId(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    try {
        const body = await c.req.json();
        const { url, events, description, maxRetries = 3, retryBackoffMs = 1000, } = body;
        // Validate URL
        if (!url || typeof url !== 'string') {
            return c.json({ error: 'url is required' }, 400);
        }
        if (!isValidUrl(url)) {
            return c.json({ error: 'url must be a valid HTTP/HTTPS URL' }, 400);
        }
        // Validate events
        const validatedEvents = validateEvents(events);
        if (!validatedEvents) {
            return c.json({
                error: 'events must be a non-empty array of valid event types',
                validEvents: WEBHOOK_EVENTS,
            }, 400);
        }
        // Validate retry settings
        if (maxRetries < 0 || maxRetries > 10) {
            return c.json({ error: 'maxRetries must be between 0 and 10' }, 400);
        }
        if (retryBackoffMs < 100 || retryBackoffMs > 60000) {
            return c.json({ error: 'retryBackoffMs must be between 100 and 60000' }, 400);
        }
        const service = new WebhookService(c.env.DB);
        const options = {
            namespace,
            tenantId,
            url,
            events: validatedEvents,
            description,
            maxRetries,
            retryBackoffMs,
        };
        const webhook = await service.create(options);
        return c.json({
            webhook: {
                ...webhook,
                // Return full secret only on creation
            },
            message: 'Webhook created. Save the secret - it will not be shown again.',
        }, 201);
    }
    catch (error) {
        console.error('Error creating webhook:', error);
        return c.json({ error: 'Failed to create webhook' }, 500);
    }
});
/**
 * GET /webhooks/:id
 * Get a specific webhook
 */
webhookRoutes.get('/:id', async (c) => {
    const id = c.req.param('id');
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const webhook = await service.get(id);
    if (!webhook) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    // Verify namespace access
    const namespace = getNamespace(c);
    if (webhook.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    return c.json({
        webhook: {
            ...webhook,
            secret: undefined,
            secretPrefix: webhook.secret.substring(0, 8) + '...',
        },
    });
});
/**
 * PUT /webhooks/:id
 * Update a webhook
 */
webhookRoutes.put('/:id', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const existing = await service.get(id);
    if (!existing) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    if (existing.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    try {
        const body = await c.req.json();
        const updates = {};
        // Validate and apply updates
        if (body.url !== undefined) {
            if (typeof body.url !== 'string' || !isValidUrl(body.url)) {
                return c.json({ error: 'url must be a valid HTTP/HTTPS URL' }, 400);
            }
            updates.url = body.url;
        }
        if (body.events !== undefined) {
            const validatedEvents = validateEvents(body.events);
            if (!validatedEvents) {
                return c.json({
                    error: 'events must be a non-empty array of valid event types',
                    validEvents: WEBHOOK_EVENTS,
                }, 400);
            }
            updates.events = validatedEvents;
        }
        if (body.isActive !== undefined) {
            updates.isActive = Boolean(body.isActive);
        }
        if (body.description !== undefined) {
            updates.description = body.description;
        }
        if (body.maxRetries !== undefined) {
            if (body.maxRetries < 0 || body.maxRetries > 10) {
                return c.json({ error: 'maxRetries must be between 0 and 10' }, 400);
            }
            updates.maxRetries = body.maxRetries;
        }
        if (body.retryBackoffMs !== undefined) {
            if (body.retryBackoffMs < 100 || body.retryBackoffMs > 60000) {
                return c.json({ error: 'retryBackoffMs must be between 100 and 60000' }, 400);
            }
            updates.retryBackoffMs = body.retryBackoffMs;
        }
        const webhook = await service.update(id, updates);
        return c.json({
            webhook: {
                ...webhook,
                secret: undefined,
                secretPrefix: webhook?.secret.substring(0, 8) + '...',
            },
        });
    }
    catch (error) {
        console.error('Error updating webhook:', error);
        return c.json({ error: 'Failed to update webhook' }, 500);
    }
});
/**
 * DELETE /webhooks/:id
 * Delete a webhook
 */
webhookRoutes.delete('/:id', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const existing = await service.get(id);
    if (!existing) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    if (existing.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    const deleted = await service.delete(id);
    if (!deleted) {
        return c.json({ error: 'Failed to delete webhook' }, 500);
    }
    return c.json({ success: true });
});
/**
 * POST /webhooks/:id/test
 * Test a webhook by sending a test event
 */
webhookRoutes.post('/:id/test', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const webhook = await service.get(id);
    if (!webhook) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    if (webhook.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    const result = await service.test(id);
    return c.json({
        success: result.success,
        status: result.status,
        error: result.error,
    });
});
/**
 * POST /webhooks/:id/rotate-secret
 * Generate a new secret for a webhook
 */
webhookRoutes.post('/:id/rotate-secret', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const webhook = await service.get(id);
    if (!webhook) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    if (webhook.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    const newSecret = await service.rotateSecret(id);
    if (!newSecret) {
        return c.json({ error: 'Failed to rotate secret' }, 500);
    }
    return c.json({
        secret: newSecret,
        message: 'Secret rotated. Update your webhook handler with the new secret.',
    });
});
/**
 * GET /webhooks/:id/deliveries
 * Get delivery history for a webhook
 */
webhookRoutes.get('/:id/deliveries', async (c) => {
    const id = c.req.param('id');
    const namespace = getNamespace(c);
    if (!c.env?.DB) {
        return c.json({ error: 'Database not available' }, 503);
    }
    const service = new WebhookService(c.env.DB);
    const webhook = await service.get(id);
    if (!webhook) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    if (webhook.namespace !== namespace) {
        return c.json({ error: 'Webhook not found' }, 404);
    }
    const status = c.req.query('status');
    const limit = parseInt(c.req.query('limit') || '100', 10);
    const offset = parseInt(c.req.query('offset') || '0', 10);
    const result = await service.getDeliveries({
        webhookId: id,
        status,
        limit,
        offset,
    });
    // Don't expose full payload in list
    const deliveries = result.deliveries.map((del) => ({
        ...del,
        payload: undefined,
        payloadPreview: JSON.parse(del.payload).type,
    }));
    return c.json({
        deliveries,
        total: result.total,
        hasMore: result.hasMore,
    });
});
/**
 * Get human-readable description for an event type
 */
function getEventDescription(event) {
    const descriptions = {
        'memory.remembered': 'Triggered when a new memory is created',
        'memory.forgotten': 'Triggered when a memory is deleted',
        'memory.updated': 'Triggered when a memory is updated',
        'knowledge.ingested': 'Triggered when a document is ingested into the knowledge bank',
        'knowledge.deleted': 'Triggered when a knowledge source is deleted',
    };
    return descriptions[event] || event;
}
export default webhookRoutes;
//# sourceMappingURL=webhooks.js.map
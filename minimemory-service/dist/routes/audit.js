/**
 * Audit Log API Routes
 *
 * Provides endpoints for querying audit logs, viewing history,
 * and audit statistics.
 */
import { Hono } from 'hono';
import { AuditService } from '../services/index.js';
const auditRoutes = new Hono();
/**
 * Helper to get AuditService
 */
function getAuditService(c) {
    if (!c.env?.DB)
        return null;
    return new AuditService(c.env.DB);
}
/**
 * GET /audit - Query audit logs
 *
 * Query params:
 * - action: filter by action type
 * - resourceType: filter by resource type
 * - resourceId: filter by specific resource
 * - userId: filter by user
 * - tenantId: filter by tenant
 * - namespace: filter by namespace
 * - startTime: start of time range (ms timestamp)
 * - endTime: end of time range (ms timestamp)
 * - success: filter by success status (true/false)
 * - requestId: filter by request ID
 * - limit: max results (default 100)
 * - offset: pagination offset
 */
auditRoutes.get('/', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const query = c.req.query();
        const options = {
            action: query.action,
            resourceType: query.resourceType,
            resourceId: query.resourceId,
            userId: query.userId,
            tenantId: query.tenantId || c.req.header('X-Tenant-Id'),
            namespace: query.namespace || c.req.header('X-Namespace'),
            startTime: query.startTime ? parseInt(query.startTime) : undefined,
            endTime: query.endTime ? parseInt(query.endTime) : undefined,
            success: query.success === 'true' ? true : query.success === 'false' ? false : undefined,
            requestId: query.requestId,
            limit: query.limit ? parseInt(query.limit) : 100,
            offset: query.offset ? parseInt(query.offset) : 0,
        };
        const result = await auditService.query(options);
        return c.json({
            success: true,
            ...result,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * GET /audit/:id - Get specific audit entry
 */
auditRoutes.get('/:id', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const id = c.req.param('id');
        const entry = await auditService.getById(id);
        if (!entry) {
            return c.json({ error: 'Audit entry not found' }, 404);
        }
        return c.json({
            success: true,
            entry,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * GET /audit/resource/:type/:id - Get history for a resource
 */
auditRoutes.get('/resource/:type/:id', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const resourceType = c.req.param('type');
        const resourceId = c.req.param('id');
        const limit = parseInt(c.req.query('limit') || '50');
        const entries = await auditService.getResourceHistory(resourceType, resourceId, limit);
        return c.json({
            success: true,
            resourceType,
            resourceId,
            entries,
            count: entries.length,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * GET /audit/user/:id - Get activity for a user
 */
auditRoutes.get('/user/:id', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const userId = c.req.param('id');
        const query = c.req.query();
        const entries = await auditService.getUserActivity(userId, {
            startTime: query.startTime ? parseInt(query.startTime) : undefined,
            endTime: query.endTime ? parseInt(query.endTime) : undefined,
            limit: query.limit ? parseInt(query.limit) : 100,
        });
        return c.json({
            success: true,
            userId,
            entries,
            count: entries.length,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * GET /audit/failures - Get failed operations
 */
auditRoutes.get('/failures', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const query = c.req.query();
        const entries = await auditService.getFailures({
            tenantId: query.tenantId || c.req.header('X-Tenant-Id'),
            namespace: query.namespace || c.req.header('X-Namespace'),
            limit: query.limit ? parseInt(query.limit) : 50,
        });
        return c.json({
            success: true,
            entries,
            count: entries.length,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * GET /audit/stats - Get audit statistics
 */
auditRoutes.get('/stats', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const query = c.req.query();
        const tenantId = query.tenantId || c.req.header('X-Tenant-Id');
        const stats = await auditService.getStats(tenantId, {
            startTime: query.startTime ? parseInt(query.startTime) : undefined,
            endTime: query.endTime ? parseInt(query.endTime) : undefined,
        });
        return c.json({
            success: true,
            tenantId,
            stats,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
/**
 * POST /audit/cleanup - Clean up old audit logs
 *
 * Body:
 * - retentionDays: number of days to retain (default: 90)
 */
auditRoutes.post('/cleanup', async (c) => {
    const auditService = getAuditService(c);
    if (!auditService) {
        return c.json({ error: 'Audit logging requires D1 database' }, 503);
    }
    try {
        const body = await c.req.json().catch(() => ({}));
        const retentionDays = body.retentionDays || 90;
        const deletedCount = await auditService.cleanup(retentionDays);
        return c.json({
            success: true,
            deletedCount,
            message: `Deleted ${deletedCount} audit entries older than ${retentionDays} days`,
        });
    }
    catch (error) {
        return c.json({
            error: error instanceof Error ? error.message : 'Unknown error',
        }, 500);
    }
});
export default auditRoutes;
//# sourceMappingURL=audit.js.map
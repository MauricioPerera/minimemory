/**
 * Audit Log API Routes
 *
 * Provides endpoints for querying audit logs, viewing history,
 * and audit statistics.
 */
import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
type Bindings = {
    DB?: D1Database;
};
declare const auditRoutes: Hono<{
    Bindings: Bindings;
}, import("hono/types").BlankSchema, "/">;
export default auditRoutes;
//# sourceMappingURL=audit.d.ts.map
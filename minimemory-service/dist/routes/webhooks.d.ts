/**
 * Webhook API Routes
 *
 * Endpoints for managing webhooks and viewing delivery history
 */
import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
type Bindings = {
    DB?: D1Database;
};
declare const webhookRoutes: Hono<{
    Bindings: Bindings;
}, import("hono/types").BlankSchema, "/">;
export default webhookRoutes;
//# sourceMappingURL=webhooks.d.ts.map
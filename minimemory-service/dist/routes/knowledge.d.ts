/**
 * Knowledge Bank API Routes
 *
 * Endpoints for RAG document ingestion and knowledge management
 */
import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
interface Ai {
    run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}
type Bindings = {
    DB?: D1Database;
    AI?: Ai;
};
declare const knowledgeRoutes: Hono<{
    Bindings: Bindings;
}, import("hono/types").BlankSchema, "/">;
export default knowledgeRoutes;
//# sourceMappingURL=knowledge.d.ts.map
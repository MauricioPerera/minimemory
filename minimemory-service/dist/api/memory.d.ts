/**
 * Memory API routes with D1 persistence and Workers AI embedding support
 */
import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
import type { MemoryManager } from '../memory/MemoryManager.js';
import type { ExecutionContext } from '@cloudflare/workers-types';
interface Ai {
    run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}
type Bindings = {
    DB?: D1Database;
    AI?: Ai;
};
type Variables = {
    executionCtx?: ExecutionContext;
};
export declare function createMemoryRoutes(getManager: (namespace: string, dimensions?: number) => MemoryManager): Hono<{
    Bindings: Bindings;
    Variables: Variables;
}, import("hono/types").BlankSchema, "/">;
export {};
//# sourceMappingURL=memory.d.ts.map
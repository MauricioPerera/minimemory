import { Hono } from 'hono';
interface Env {
    DB: D1Database;
    JWT_SECRET: string;
}
declare const tenants: Hono<{
    Bindings: Env;
}, import("hono/types").BlankSchema, "/">;
export default tenants;
//# sourceMappingURL=tenants.d.ts.map
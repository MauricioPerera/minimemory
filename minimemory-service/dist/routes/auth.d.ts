import { Hono } from 'hono';
interface Env {
    DB: D1Database;
    JWT_SECRET: string;
    JWT_REFRESH_SECRET: string;
}
declare const auth: Hono<{
    Bindings: Env;
}, import("hono/types").BlankSchema, "/">;
export default auth;
//# sourceMappingURL=auth.d.ts.map
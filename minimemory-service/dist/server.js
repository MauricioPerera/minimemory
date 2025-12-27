/**
 * Local development server
 */
import { serve } from '@hono/node-server';
import app from './index.js';
const port = process.env.PORT ? parseInt(process.env.PORT) : 3000;
console.log(`
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   🧠 minimemory-service                                       ║
║   Agentic Memory API                                          ║
║                                                               ║
║   Server running at: http://localhost:${port}                   ║
║                                                               ║
║   Endpoints:                                                  ║
║   • POST /api/v1/remember   - Store memory                    ║
║   • POST /api/v1/recall     - Search memories                 ║
║   • DELETE /api/v1/forget   - Delete memory                   ║
║   • GET /api/v1/stats       - Memory stats                    ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
`);
serve({
    fetch: app.fetch,
    port,
});
//# sourceMappingURL=server.js.map
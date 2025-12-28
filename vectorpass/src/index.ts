/**
 * VectorPass - RAG as a Service
 *
 * API Endpoints:
 * - POST /auth/register     - Create account
 * - POST /auth/login        - Get API key by email
 * - POST /auth/regenerate   - Regenerate API key
 *
 * - POST /v1/index          - Index document
 * - POST /v1/batch          - Batch index
 * - POST /v1/search         - Semantic search
 * - POST /v1/keyword        - Keyword search (BM25)
 * - DELETE /v1/vectors/:id  - Delete document
 * - GET /v1/stats           - Get usage stats
 *
 * - POST /webhooks/stripe   - Stripe subscription webhooks
 */

import { Env, User, TIER_LIMITS, IndexRequest, BatchIndexRequest, SearchRequest } from './types';
import { requireAuth, createUser, getUserByEmail, regenerateApiKey } from './auth';
import { checkSearchLimit, checkVectorLimit, recordSearch, getUsageStats, rateLimitExceeded, rateLimitHeaders } from './ratelimit';
import { VectorDB } from './vectordb';

// CORS headers
const CORS_HEADERS = {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, X-API-Key, Authorization',
};

/**
 * JSON response helper
 */
function json(data: any, status: number = 200, extraHeaders: Record<string, string> = {}): Response {
    return new Response(JSON.stringify(data), {
        status,
        headers: {
            'Content-Type': 'application/json',
            ...CORS_HEADERS,
            ...extraHeaders
        }
    });
}

/**
 * Error response helper
 */
function error(message: string, status: number = 400): Response {
    return json({ success: false, error: message }, status);
}

/**
 * Main worker handler
 */
export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);
        const path = url.pathname;

        // Handle CORS preflight
        if (request.method === 'OPTIONS') {
            return new Response(null, { headers: CORS_HEADERS });
        }

        try {
            // ============================================================
            // Public endpoints (no auth required)
            // ============================================================

            // GET / - API info
            if (path === '/' || path === '') {
                return json({
                    name: 'VectorPass',
                    version: '0.1.0',
                    description: 'RAG as a Service - Semantic search for AI agents',
                    docs: 'https://vectorpass.automators.work/docs',
                    endpoints: {
                        auth: {
                            'POST /auth/register': 'Create account {email, referralCode?}',
                            'POST /auth/login': 'Get API key {email}',
                        },
                        vectors: {
                            'POST /v1/index': 'Index document {id, text, metadata?}',
                            'POST /v1/batch': 'Batch index {items: [{id, text, metadata?}]}',
                            'POST /v1/search': 'Semantic search {query, k?}',
                            'POST /v1/keyword': 'Keyword search {query, k?}',
                            'DELETE /v1/vectors/:id': 'Delete document',
                            'GET /v1/stats': 'Usage statistics',
                        }
                    }
                });
            }

            // POST /auth/register - Create new account
            if (request.method === 'POST' && path === '/auth/register') {
                const body = await request.json() as { email: string; referralCode?: string };

                if (!body.email || !body.email.includes('@')) {
                    return error('Valid email required');
                }

                // Check if email exists
                const existing = await getUserByEmail(body.email, env);
                if (existing) {
                    return error('Email already registered', 409);
                }

                const user = await createUser(body.email, env, body.referralCode);

                return json({
                    success: true,
                    data: {
                        id: user.id,
                        email: user.email,
                        apiKey: user.apiKey,
                        tier: user.tier,
                        referralCode: user.referralCode,
                        limits: TIER_LIMITS[user.tier]
                    }
                }, 201);
            }

            // POST /auth/login - Get API key by email (simple auth for MVP)
            if (request.method === 'POST' && path === '/auth/login') {
                const body = await request.json() as { email: string };

                if (!body.email) {
                    return error('Email required');
                }

                const user = await getUserByEmail(body.email, env);
                if (!user) {
                    return error('User not found', 404);
                }

                // In production, implement proper email verification
                return json({
                    success: true,
                    data: {
                        apiKey: user.apiKey,
                        tier: user.tier,
                        limits: TIER_LIMITS[user.tier]
                    }
                });
            }

            // ============================================================
            // Protected endpoints (require API key)
            // ============================================================

            // Check authentication for /v1/* endpoints
            if (path.startsWith('/v1/') || path === '/auth/regenerate') {
                const authResult = await requireAuth(request, env);

                if (authResult instanceof Response) {
                    return authResult;
                }

                const { user } = authResult;

                // POST /auth/regenerate - Generate new API key
                if (path === '/auth/regenerate') {
                    const newKey = await regenerateApiKey(user.id, env);
                    if (!newKey) {
                        return error('Failed to regenerate key', 500);
                    }
                    return json({ success: true, data: { apiKey: newKey } });
                }

                // Get or create default database for user
                const db = await VectorDB.create(user, 'default', env);

                // POST /v1/index - Index single document
                if (request.method === 'POST' && path === '/v1/index') {
                    const body = await request.json() as IndexRequest;

                    if (!body.id || !body.text) {
                        return error('id and text required');
                    }

                    // Check vector limit
                    const vectorCheck = await checkVectorLimit(user, db.len(), 1);
                    if (!vectorCheck.allowed) {
                        return error(`Vector limit reached (${vectorCheck.max}). Upgrade to add more.`, 403);
                    }

                    await db.index(body.id, body.text, body.metadata);
                    await db.save();

                    return json({
                        success: true,
                        data: {
                            id: body.id,
                            vectorCount: db.len()
                        }
                    });
                }

                // POST /v1/batch - Batch index
                if (request.method === 'POST' && path === '/v1/batch') {
                    const body = await request.json() as BatchIndexRequest;

                    if (!body.items || !Array.isArray(body.items)) {
                        return error('items array required');
                    }

                    const limits = TIER_LIMITS[user.tier];
                    if (body.items.length > limits.batchSize) {
                        return error(`Batch size exceeds limit (${limits.batchSize})`);
                    }

                    // Check vector limit
                    const vectorCheck = await checkVectorLimit(user, db.len(), body.items.length);
                    if (!vectorCheck.allowed) {
                        return error(`Would exceed vector limit (${vectorCheck.max}). Can add ${vectorCheck.remaining} more.`, 403);
                    }

                    const indexed = await db.indexBatch(body.items);
                    await db.save();

                    return json({
                        success: true,
                        data: {
                            indexed,
                            vectorCount: db.len()
                        }
                    });
                }

                // POST /v1/search - Semantic search
                if (request.method === 'POST' && path === '/v1/search') {
                    const body = await request.json() as SearchRequest;

                    if (!body.query) {
                        return error('query required');
                    }

                    // Check rate limit
                    const rateCheck = await checkSearchLimit(user, env);
                    if (!rateCheck.allowed) {
                        return rateLimitExceeded(rateCheck.resetAt);
                    }

                    const k = Math.min(body.k || 10, 100);
                    const results = await db.search(body.query, k);

                    // Record the search
                    await recordSearch(user, env);

                    return json({
                        success: true,
                        data: {
                            results,
                            query: body.query,
                            k
                        }
                    }, 200, rateLimitHeaders(rateCheck.remaining - 1, rateCheck.resetAt));
                }

                // POST /v1/keyword - Keyword search
                if (request.method === 'POST' && path === '/v1/keyword') {
                    const body = await request.json() as SearchRequest;

                    if (!body.query) {
                        return error('query required');
                    }

                    // Check rate limit (counts as search)
                    const rateCheck = await checkSearchLimit(user, env);
                    if (!rateCheck.allowed) {
                        return rateLimitExceeded(rateCheck.resetAt);
                    }

                    const k = Math.min(body.k || 10, 100);
                    const results = db.keywordSearch(body.query, k);

                    await recordSearch(user, env);

                    return json({
                        success: true,
                        data: {
                            results,
                            query: body.query,
                            k
                        }
                    }, 200, rateLimitHeaders(rateCheck.remaining - 1, rateCheck.resetAt));
                }

                // DELETE /v1/vectors/:id
                if (request.method === 'DELETE' && path.startsWith('/v1/vectors/')) {
                    const id = path.split('/')[3];

                    if (!id) {
                        return error('Vector ID required');
                    }

                    const deleted = db.delete(id);
                    await db.save();

                    return json({
                        success: true,
                        data: { deleted, id }
                    });
                }

                // GET /v1/stats - Usage statistics
                if (request.method === 'GET' && path === '/v1/stats') {
                    const usage = await getUsageStats(user, env);
                    const limits = TIER_LIMITS[user.tier];

                    return json({
                        success: true,
                        data: {
                            tier: user.tier,
                            vectorCount: db.len(),
                            vectorLimit: limits.maxVectors,
                            vectorsRemaining: limits.maxVectors - db.len(),
                            ...usage,
                            referralCode: user.referralCode,
                            referralCount: user.referralCount
                        }
                    });
                }
            }

            // 404 for unknown routes
            return error('Not found', 404);

        } catch (err: any) {
            console.error('Error:', err);
            return error(err.message || 'Internal server error', 500);
        }
    }
};

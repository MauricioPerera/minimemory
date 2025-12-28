/**
 * VectorPass - RAG as a Service
 *
 * API Endpoints:
 * - POST /auth/register       - Create account
 * - POST /auth/login          - Get API key by email
 * - POST /auth/verify         - Verify email with code
 * - POST /auth/regenerate     - Regenerate API key
 *
 * - POST /v1/index            - Index document
 * - POST /v1/batch            - Batch index
 * - POST /v1/search           - Semantic search
 * - POST /v1/keyword          - Keyword search (BM25)
 * - DELETE /v1/vectors/:id    - Delete document
 * - GET /v1/stats             - Get usage stats
 *
 * - POST /webhooks/stripe     - Stripe subscription webhooks
 */

import { Env, User, TIER_LIMITS, IndexRequest, BatchIndexRequest, SearchRequest, KeywordSearchRequest, CreateDatabaseRequest } from './types';
import { requireAuth, createUser, getUserByEmail, regenerateApiKey, isAdmin, listAllUsers, getPlatformStats, getUserById, adminUpdateUser, adminDeleteUser, trimExcessVectors } from './auth';
import { checkSearchLimit, checkVectorLimit, recordSearch, getUsageStats, rateLimitExceeded, rateLimitHeaders } from './ratelimit';
import { VectorDB, initWasm } from './vectordb';
import { handleStripeWebhook, createCheckoutSession, checkGracePeriodAndTrim } from './stripe';
import { sendVerificationEmail, verifyEmailCode, isEmailVerified } from './email';
import { validateDatabaseName, listUserDatabases, getTotalVectorCount, checkDatabaseLimit, databaseExists, deleteDatabase, deleteAllUserDatabases, getDatabaseInfo } from './database';

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

// Initialize WASM on first request
let wasmInitialized = false;

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

        // Initialize WASM on first request
        if (!wasmInitialized) {
            await initWasm();
            wasmInitialized = true;
        }

        try {
            // ============================================================
            // Webhooks (no auth, verified by signature)
            // ============================================================

            // POST /webhooks/stripe - Stripe subscription webhooks
            if (request.method === 'POST' && path === '/webhooks/stripe') {
                return handleStripeWebhook(request, env);
            }

            // POST /billing/checkout - Create Stripe checkout session
            if (request.method === 'POST' && path === '/billing/checkout') {
                const authResult = await requireAuth(request, env);
                if (authResult instanceof Response) {
                    return authResult;
                }
                const { user } = authResult;

                const body = await request.json() as { tier: string; successUrl: string; cancelUrl: string };

                if (!body.tier || !body.successUrl || !body.cancelUrl) {
                    return error('tier, successUrl, and cancelUrl required');
                }

                if (!['starter', 'pro', 'business'].includes(body.tier)) {
                    return error('Invalid tier. Must be starter, pro, or business');
                }

                try {
                    const checkoutUrl = await createCheckoutSession(
                        user,
                        body.tier as 'starter' | 'pro' | 'business',
                        body.successUrl,
                        body.cancelUrl,
                        env
                    );

                    return json({
                        success: true,
                        data: { url: checkoutUrl }
                    });
                } catch (err: any) {
                    return error(err.message || 'Failed to create checkout session', 500);
                }
            }

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
                            'POST /auth/verify': 'Verify email {email, code}',
                            'POST /auth/login': 'Request login code {email}',
                        },
                        databases: {
                            'GET /v1/databases': 'List all databases',
                            'POST /v1/databases': 'Create database {name}',
                            'DELETE /v1/databases/:name': 'Delete database',
                        },
                        vectors: {
                            'POST /v1/index': 'Index document {id, text, metadata?, db?}',
                            'POST /v1/batch': 'Batch index {items, db?}',
                            'POST /v1/search': 'Semantic search {query, k?, db?}',
                            'POST /v1/keyword': 'Keyword search {query, k?, db?}',
                            'DELETE /v1/vectors/:id': 'Delete document (?db=name)',
                            'GET /v1/stats': 'Usage statistics (?db=name)',
                        }
                    },
                    pricing: {
                        free: { vectors: 1000, searches: '100/day', price: '$0' },
                        starter: { vectors: 50000, searches: '10K/day', price: '$9/mo' },
                        pro: { vectors: 500000, searches: '100K/day', price: '$29/mo' },
                        business: { vectors: 5000000, searches: '1M/day', price: '$79/mo' }
                    }
                });
            }

            // POST /auth/register - Create new account
            if (request.method === 'POST' && path === '/auth/register') {
                const body = await request.json() as { email: string; referralCode?: string };

                if (!body.email || !body.email.includes('@')) {
                    return error('Valid email required');
                }

                // Normalize email
                const email = body.email.toLowerCase().trim();

                // Check if email exists
                const existing = await getUserByEmail(email, env);
                if (existing) {
                    return error('Email already registered', 409);
                }

                // Create user (unverified)
                const user = await createUser(email, env, body.referralCode);

                // Send verification email
                await sendVerificationEmail(user.id, email, env);

                return json({
                    success: true,
                    message: 'Verification code sent to your email',
                    data: {
                        id: user.id,
                        email: user.email,
                        verified: false
                    }
                }, 201);
            }

            // POST /auth/verify - Verify email with code
            if (request.method === 'POST' && path === '/auth/verify') {
                const body = await request.json() as { email: string; code: string };

                if (!body.email || !body.code) {
                    return error('Email and code required');
                }

                const email = body.email.toLowerCase().trim();
                const user = await getUserByEmail(email, env);

                if (!user) {
                    return error('User not found', 404);
                }

                const verified = await verifyEmailCode(user.id, body.code, env);

                if (!verified) {
                    return error('Invalid or expired code', 401);
                }

                return json({
                    success: true,
                    data: {
                        apiKey: user.apiKey,
                        tier: user.tier,
                        referralCode: user.referralCode,
                        limits: TIER_LIMITS[user.tier]
                    }
                });
            }

            // POST /auth/login - Request login code (passwordless)
            if (request.method === 'POST' && path === '/auth/login') {
                const body = await request.json() as { email: string };

                if (!body.email) {
                    return error('Email required');
                }

                const email = body.email.toLowerCase().trim();
                const user = await getUserByEmail(email, env);

                if (!user) {
                    return error('User not found', 404);
                }

                // Send verification code for login
                await sendVerificationEmail(user.id, email, env);

                return json({
                    success: true,
                    message: 'Login code sent to your email'
                });
            }

            // POST /auth/resend - Resend verification code
            if (request.method === 'POST' && path === '/auth/resend') {
                const body = await request.json() as { email: string };

                if (!body.email) {
                    return error('Email required');
                }

                const email = body.email.toLowerCase().trim();
                const user = await getUserByEmail(email, env);

                if (!user) {
                    return error('User not found', 404);
                }

                await sendVerificationEmail(user.id, email, env);

                return json({
                    success: true,
                    message: 'Verification code sent'
                });
            }

            // ============================================================
            // Admin endpoints (require admin API key)
            // ============================================================

            if (path.startsWith('/admin/')) {
                const authResult = await requireAuth(request, env);
                if (authResult instanceof Response) {
                    return authResult;
                }
                const { user } = authResult;

                if (!isAdmin(user)) {
                    return error('Admin access required', 403);
                }

                // GET /admin/users - List all users
                if (request.method === 'GET' && path === '/admin/users') {
                    const users = await listAllUsers(env);

                    // Add verified status to each user
                    const usersWithStatus = await Promise.all(users.map(async (u) => ({
                        ...u,
                        verified: await isEmailVerified(u.id, env)
                    })));

                    return json({
                        success: true,
                        data: {
                            users: usersWithStatus,
                            total: users.length
                        }
                    });
                }

                // GET /admin/stats - Platform statistics
                if (request.method === 'GET' && path === '/admin/stats') {
                    const stats = await getPlatformStats(env);

                    // Calculate revenue (estimated from tier counts)
                    const monthlyRevenue =
                        stats.usersByTier.starter * 9 +
                        stats.usersByTier.pro * 29 +
                        stats.usersByTier.business * 79;

                    return json({
                        success: true,
                        data: {
                            ...stats,
                            estimatedMRR: monthlyRevenue,
                            paidUsers: stats.usersByTier.starter + stats.usersByTier.pro + stats.usersByTier.business
                        }
                    });
                }

                // GET /admin/users/:id - Get single user
                if (request.method === 'GET' && path.match(/^\/admin\/users\/[^/]+$/)) {
                    const userId = path.split('/')[3];
                    const targetUser = await getUserById(userId, env);

                    if (!targetUser) {
                        return error('User not found', 404);
                    }

                    // Get total vector count across all databases
                    const vectorCount = await getTotalVectorCount(userId, env);
                    const databases = await listUserDatabases(userId, env);

                    return json({
                        success: true,
                        data: {
                            ...targetUser,
                            apiKey: targetUser.apiKey.substring(0, 12) + '...',
                            verified: await isEmailVerified(userId, env),
                            vectorCount,
                            vectorLimit: TIER_LIMITS[targetUser.tier].maxVectors,
                            databaseCount: databases.length,
                            databaseLimit: TIER_LIMITS[targetUser.tier].maxDatabases,
                            databases
                        }
                    });
                }

                // PUT /admin/users/:id - Update user
                if (request.method === 'PUT' && path.match(/^\/admin\/users\/[^/]+$/)) {
                    const userId = path.split('/')[3];
                    const body = await request.json() as {
                        tier?: string;
                        email?: string;
                        referralDiscount?: number;
                    };

                    const updates: any = {};
                    if (body.tier && ['free', 'starter', 'pro', 'business'].includes(body.tier)) {
                        updates.tier = body.tier;
                    }
                    if (body.email) {
                        updates.email = body.email.toLowerCase().trim();
                    }
                    if (typeof body.referralDiscount === 'number') {
                        updates.referralDiscount = Math.max(0, Math.min(50, body.referralDiscount));
                    }

                    const updatedUser = await adminUpdateUser(userId, updates, env);
                    if (!updatedUser) {
                        return error('User not found', 404);
                    }

                    return json({
                        success: true,
                        data: {
                            ...updatedUser,
                            apiKey: updatedUser.apiKey.substring(0, 12) + '...'
                        }
                    });
                }

                // DELETE /admin/users/:id - Delete user
                if (request.method === 'DELETE' && path.match(/^\/admin\/users\/[^/]+$/)) {
                    const userId = path.split('/')[3];

                    // Prevent self-deletion
                    if (userId === user.id) {
                        return error('Cannot delete your own account', 400);
                    }

                    const deleted = await adminDeleteUser(userId, env);
                    if (!deleted) {
                        return error('User not found', 404);
                    }

                    return json({
                        success: true,
                        data: { deleted: true, userId }
                    });
                }

                // POST /admin/users/:id/trim - Force trim excess vectors
                if (request.method === 'POST' && path.match(/^\/admin\/users\/[^/]+\/trim$/)) {
                    const userId = path.split('/')[3];

                    const result = await trimExcessVectors(userId, env);

                    return json({
                        success: true,
                        data: result
                    });
                }

                // POST /admin/users/:id/clear-grace - Clear payment failure grace period
                if (request.method === 'POST' && path.match(/^\/admin\/users\/[^/]+\/clear-grace$/)) {
                    const userId = path.split('/')[3];

                    const updated = await adminUpdateUser(userId, {
                        paymentFailedAt: undefined,
                        previousTier: undefined
                    }, env);

                    if (!updated) {
                        return error('User not found', 404);
                    }

                    return json({
                        success: true,
                        data: { cleared: true }
                    });
                }

                return error('Admin endpoint not found', 404);
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

                // Check grace period and trim vectors if expired (runs in background)
                if (user.paymentFailedAt) {
                    await checkGracePeriodAndTrim(user.id, env);
                }

                // Check if email is verified (for write operations)
                const verified = await isEmailVerified(user.id, env);

                // POST /auth/regenerate - Generate new API key
                if (path === '/auth/regenerate') {
                    if (!verified) {
                        return error('Email verification required', 403);
                    }

                    const newKey = await regenerateApiKey(user.id, env);
                    if (!newKey) {
                        return error('Failed to regenerate key', 500);
                    }
                    return json({ success: true, data: { apiKey: newKey } });
                }

                // ============================================================
                // Database management endpoints
                // ============================================================

                // GET /v1/databases - List all databases
                if (request.method === 'GET' && path === '/v1/databases') {
                    const databases = await listUserDatabases(user.id, env);
                    const dbLimit = TIER_LIMITS[user.tier].maxDatabases;

                    return json({
                        success: true,
                        data: {
                            databases,
                            count: databases.length,
                            limit: dbLimit
                        }
                    });
                }

                // POST /v1/databases - Create new database
                if (request.method === 'POST' && path === '/v1/databases') {
                    if (!verified) {
                        return error('Email verification required', 403);
                    }

                    const body = await request.json() as CreateDatabaseRequest;

                    if (!body.name) {
                        return error('Database name required');
                    }

                    // Validate database name
                    const validation = validateDatabaseName(body.name);
                    if (!validation.valid) {
                        return error(validation.error || 'Invalid database name');
                    }

                    // Check if database already exists
                    if (await databaseExists(user.id, body.name, env)) {
                        return error('Database already exists', 409);
                    }

                    // Check database limit
                    const limitCheck = await checkDatabaseLimit(user, env);
                    if (!limitCheck.allowed) {
                        return error(`Database limit reached (${limitCheck.max}). Upgrade to create more.`, 403);
                    }

                    // Create empty database
                    const db = await VectorDB.create(user, body.name, env);
                    await db.save();

                    return json({
                        success: true,
                        data: {
                            name: body.name,
                            created: true,
                            databaseCount: limitCheck.current + 1,
                            databaseLimit: limitCheck.max
                        }
                    }, 201);
                }

                // DELETE /v1/databases/:name - Delete database
                if (request.method === 'DELETE' && path.match(/^\/v1\/databases\/[^/]+$/)) {
                    if (!verified) {
                        return error('Email verification required', 403);
                    }

                    const dbName = decodeURIComponent(path.split('/')[3]);

                    if (dbName === 'default') {
                        return error('Cannot delete the default database', 400);
                    }

                    const validation = validateDatabaseName(dbName);
                    if (!validation.valid) {
                        return error(validation.error || 'Invalid database name');
                    }

                    const deleted = await deleteDatabase(user.id, dbName, env);
                    if (!deleted) {
                        return error('Database not found', 404);
                    }

                    return json({
                        success: true,
                        data: { deleted: true, name: dbName }
                    });
                }

                // GET /v1/databases/:name - Get database info
                if (request.method === 'GET' && path.match(/^\/v1\/databases\/[^/]+$/)) {
                    const dbName = decodeURIComponent(path.split('/')[3]);

                    const validation = validateDatabaseName(dbName);
                    if (!validation.valid) {
                        return error(validation.error || 'Invalid database name');
                    }

                    const info = await getDatabaseInfo(user.id, dbName, env);
                    if (!info) {
                        return error('Database not found', 404);
                    }

                    return json({
                        success: true,
                        data: info
                    });
                }

                // ============================================================
                // Vector operations (with optional db parameter)
                // ============================================================

                // Helper to get database name from body or query params
                const getDbName = async (body: any): Promise<string> => {
                    const dbName = body?.db || url.searchParams.get('db') || 'default';
                    const validation = validateDatabaseName(dbName);
                    if (!validation.valid) {
                        throw new Error(validation.error || 'Invalid database name');
                    }
                    return dbName;
                };

                // Get total vector count across all databases (for limit checking)
                const totalVectorCount = await getTotalVectorCount(user.id, env);

                // POST /v1/index - Index single document
                if (request.method === 'POST' && path === '/v1/index') {
                    if (!verified) {
                        return error('Email verification required to index documents', 403);
                    }

                    const body = await request.json() as IndexRequest;

                    if (!body.id || !body.text) {
                        return error('id and text required');
                    }

                    // Get target database
                    const dbName = await getDbName(body);
                    const db = await VectorDB.create(user, dbName, env);

                    // Check vector limit (using total across all DBs)
                    const vectorCheck = await checkVectorLimit(user, totalVectorCount, 1);
                    if (!vectorCheck.allowed) {
                        return error(`Vector limit reached (${vectorCheck.max}). Upgrade to add more.`, 403);
                    }

                    await db.index(body.id, body.text, body.metadata);
                    await db.save();

                    return json({
                        success: true,
                        data: {
                            id: body.id,
                            db: dbName,
                            vectorCount: db.len(),
                            totalVectorCount: totalVectorCount + 1
                        }
                    });
                }

                // POST /v1/batch - Batch index
                if (request.method === 'POST' && path === '/v1/batch') {
                    if (!verified) {
                        return error('Email verification required to index documents', 403);
                    }

                    const body = await request.json() as BatchIndexRequest;

                    if (!body.items || !Array.isArray(body.items)) {
                        return error('items array required');
                    }

                    const limits = TIER_LIMITS[user.tier];
                    if (body.items.length > limits.batchSize) {
                        return error(`Batch size exceeds limit (${limits.batchSize})`);
                    }

                    // Get target database
                    const dbName = await getDbName(body);
                    const db = await VectorDB.create(user, dbName, env);

                    // Check vector limit (using total across all DBs)
                    const vectorCheck = await checkVectorLimit(user, totalVectorCount, body.items.length);
                    if (!vectorCheck.allowed) {
                        return error(`Would exceed vector limit (${vectorCheck.max}). Can add ${vectorCheck.remaining} more.`, 403);
                    }

                    const indexed = await db.indexBatch(body.items);
                    await db.save();

                    return json({
                        success: true,
                        data: {
                            indexed,
                            db: dbName,
                            vectorCount: db.len(),
                            totalVectorCount: totalVectorCount + indexed
                        }
                    });
                }

                // POST /v1/search - Semantic search (allowed without verification)
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

                    // Get target database
                    const dbName = await getDbName(body);
                    const db = await VectorDB.create(user, dbName, env);

                    const k = Math.min(body.k || 10, 100);
                    const results = await db.search(body.query, k);

                    // Record the search
                    await recordSearch(user, env);

                    return json({
                        success: true,
                        data: {
                            results,
                            query: body.query,
                            k,
                            db: dbName
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

                    // Get target database
                    const dbName = await getDbName(body);
                    const db = await VectorDB.create(user, dbName, env);

                    const k = Math.min(body.k || 10, 100);
                    const results = db.keywordSearch(body.query, k);

                    await recordSearch(user, env);

                    return json({
                        success: true,
                        data: {
                            results,
                            query: body.query,
                            k,
                            db: dbName
                        }
                    }, 200, rateLimitHeaders(rateCheck.remaining - 1, rateCheck.resetAt));
                }

                // DELETE /v1/vectors/:id
                if (request.method === 'DELETE' && path.startsWith('/v1/vectors/')) {
                    if (!verified) {
                        return error('Email verification required', 403);
                    }

                    const id = path.split('/')[3];

                    if (!id) {
                        return error('Vector ID required');
                    }

                    // Get target database from query params
                    const dbName = await getDbName({});
                    const db = await VectorDB.create(user, dbName, env);

                    const deleted = db.delete(id);
                    await db.save();

                    return json({
                        success: true,
                        data: { deleted, id, db: dbName }
                    });
                }

                // GET /v1/stats - Usage statistics
                if (request.method === 'GET' && path === '/v1/stats') {
                    const usage = await getUsageStats(user, env);
                    const limits = TIER_LIMITS[user.tier];
                    const databases = await listUserDatabases(user.id, env);

                    // If db param specified, get info for that specific database
                    const dbName = url.searchParams.get('db');
                    let dbInfo = null;
                    if (dbName) {
                        const validation = validateDatabaseName(dbName);
                        if (validation.valid) {
                            const db = await VectorDB.create(user, dbName, env);
                            dbInfo = db.info();
                        }
                    }

                    return json({
                        success: true,
                        data: {
                            tier: user.tier,
                            verified: await isEmailVerified(user.id, env),
                            totalVectorCount,
                            vectorLimit: limits.maxVectors,
                            vectorsRemaining: limits.maxVectors - totalVectorCount,
                            databaseCount: databases.length,
                            databaseLimit: limits.maxDatabases,
                            databases,
                            ...usage,
                            referralCode: user.referralCode,
                            referralCount: user.referralCount,
                            referralDiscount: user.referralDiscount || 0,
                            ...(dbInfo && { dbInfo })
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

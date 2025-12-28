/**
 * VectorPass Authentication & User Management
 */

import { Env, User, Tier, generateApiKey, generateReferralCode } from './types';

/**
 * Extracts API key from request headers
 */
export function extractApiKey(request: Request): string | null {
    // Check X-API-Key header first
    const apiKey = request.headers.get('X-API-Key');
    if (apiKey) return apiKey;

    // Check Authorization: Bearer header
    const auth = request.headers.get('Authorization');
    if (auth?.startsWith('Bearer ')) {
        return auth.slice(7);
    }

    return null;
}

/**
 * Validates API key and returns user data
 */
export async function validateApiKey(apiKey: string, env: Env): Promise<User | null> {
    if (!apiKey || !apiKey.startsWith('vp_')) {
        return null;
    }

    // Look up user by API key
    const userId = await env.USERS.get(`apikey:${apiKey}`);
    if (!userId) {
        return null;
    }

    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) {
        return null;
    }

    return JSON.parse(userData) as User;
}

/**
 * Creates a new user
 */
export async function createUser(
    email: string,
    env: Env,
    referredByCode?: string
): Promise<User> {
    const id = crypto.randomUUID();
    const apiKey = generateApiKey(false);
    const referralCode = generateReferralCode();

    let referredBy: string | undefined;

    // Handle referral
    if (referredByCode) {
        const referrerId = await env.USERS.get(`referral:${referredByCode}`);
        if (referrerId) {
            referredBy = referrerId;

            // Increment referrer's count
            const referrerData = await env.USERS.get(`user:${referrerId}`);
            if (referrerData) {
                const referrer = JSON.parse(referrerData) as User;
                referrer.referralCount++;
                await env.USERS.put(`user:${referrerId}`, JSON.stringify(referrer));
            }
        }
    }

    const user: User = {
        id,
        email,
        apiKey,
        tier: 'free',
        createdAt: new Date().toISOString(),
        referralCode,
        referredBy,
        referralCount: 0
    };

    // Store user data
    await env.USERS.put(`user:${id}`, JSON.stringify(user));

    // Create API key lookup
    await env.USERS.put(`apikey:${apiKey}`, id);

    // Create email lookup (for login)
    await env.USERS.put(`email:${email}`, id);

    // Create referral code lookup
    await env.USERS.put(`referral:${referralCode}`, id);

    return user;
}

/**
 * Gets user by email
 */
export async function getUserByEmail(email: string, env: Env): Promise<User | null> {
    const userId = await env.USERS.get(`email:${email}`);
    if (!userId) return null;

    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return null;

    return JSON.parse(userData) as User;
}

/**
 * Updates user tier (called after Stripe webhook)
 */
export async function updateUserTier(
    userId: string,
    tier: Tier,
    stripeCustomerId?: string,
    stripeSubscriptionId?: string,
    env?: Env
): Promise<User | null> {
    if (!env) return null;

    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return null;

    const user = JSON.parse(userData) as User;
    user.tier = tier;

    if (stripeCustomerId) user.stripeCustomerId = stripeCustomerId;
    if (stripeSubscriptionId) user.stripeSubscriptionId = stripeSubscriptionId;

    await env.USERS.put(`user:${userId}`, JSON.stringify(user));
    return user;
}

/**
 * Regenerates API key for user
 */
export async function regenerateApiKey(userId: string, env: Env): Promise<string | null> {
    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return null;

    const user = JSON.parse(userData) as User;
    const oldApiKey = user.apiKey;
    const newApiKey = generateApiKey(false);

    // Update user
    user.apiKey = newApiKey;
    await env.USERS.put(`user:${userId}`, JSON.stringify(user));

    // Remove old API key lookup
    await env.USERS.delete(`apikey:${oldApiKey}`);

    // Create new API key lookup
    await env.USERS.put(`apikey:${newApiKey}`, userId);

    return newApiKey;
}

/**
 * Middleware that requires authentication
 */
export async function requireAuth(
    request: Request,
    env: Env
): Promise<{ user: User } | Response> {
    const apiKey = extractApiKey(request);

    if (!apiKey) {
        return new Response(JSON.stringify({
            success: false,
            error: 'API key required. Use X-API-Key header or Bearer token.'
        }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
        });
    }

    const user = await validateApiKey(apiKey, env);

    if (!user) {
        return new Response(JSON.stringify({
            success: false,
            error: 'Invalid API key'
        }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
        });
    }

    return { user };
}

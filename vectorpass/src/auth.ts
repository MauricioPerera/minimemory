/**
 * VectorPass Authentication & User Management
 */

import { Env, User, Tier, generateApiKey, generateReferralCode, REFERRAL_CONFIG, ADMIN_EMAILS } from './types';

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
        referralCount: 0,
        referralDiscount: 0
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
 * Grant referral discount to the referrer when their referral subscribes
 * Called when a referred user upgrades to a paid plan
 */
export async function grantReferralReward(
    referredUserId: string,
    env: Env
): Promise<void> {
    // Get the referred user
    const referredData = await env.USERS.get(`user:${referredUserId}`);
    if (!referredData) return;

    const referredUser = JSON.parse(referredData) as User;

    // Check if they were referred by someone
    if (!referredUser.referredBy) return;

    // Get the referrer
    const referrerData = await env.USERS.get(`user:${referredUser.referredBy}`);
    if (!referrerData) return;

    const referrer = JSON.parse(referrerData) as User;

    // Calculate new discount (cap at max)
    const newDiscount = Math.min(
        (referrer.referralDiscount || 0) + REFERRAL_CONFIG.discountPerReferral,
        REFERRAL_CONFIG.maxDiscount
    );

    // Only update if discount would increase
    if (newDiscount > (referrer.referralDiscount || 0)) {
        referrer.referralDiscount = newDiscount;
        referrer.referralCount = (referrer.referralCount || 0) + 1;
        await env.USERS.put(`user:${referrer.id}`, JSON.stringify(referrer));
        console.log(`Granted ${REFERRAL_CONFIG.discountPerReferral}% referral discount to ${referrer.email}. Total: ${newDiscount}%`);
    }
}

/**
 * Revoke referral discount when a referred user cancels their subscription
 * Called when a referred user downgrades to free
 */
export async function revokeReferralReward(
    referredUserId: string,
    env: Env
): Promise<void> {
    // Get the referred user
    const referredData = await env.USERS.get(`user:${referredUserId}`);
    if (!referredData) return;

    const referredUser = JSON.parse(referredData) as User;

    // Check if they were referred by someone
    if (!referredUser.referredBy) return;

    // Get the referrer
    const referrerData = await env.USERS.get(`user:${referredUser.referredBy}`);
    if (!referrerData) return;

    const referrer = JSON.parse(referrerData) as User;

    // Only decrement if there's a discount to revoke
    if ((referrer.referralDiscount || 0) > 0) {
        referrer.referralDiscount = Math.max(
            0,
            (referrer.referralDiscount || 0) - REFERRAL_CONFIG.discountPerReferral
        );
        referrer.referralCount = Math.max(0, (referrer.referralCount || 1) - 1);
        await env.USERS.put(`user:${referrer.id}`, JSON.stringify(referrer));
        console.log(`Revoked ${REFERRAL_CONFIG.discountPerReferral}% referral discount from ${referrer.email}. Remaining: ${referrer.referralDiscount}%`);
    }
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

/**
 * Check if user is an admin
 */
export function isAdmin(user: User): boolean {
    return ADMIN_EMAILS.includes(user.email.toLowerCase());
}

/**
 * List all users (admin only)
 */
export async function listAllUsers(env: Env): Promise<User[]> {
    const users: User[] = [];

    // List all keys with user: prefix
    let cursor: string | undefined;

    do {
        const result = await env.USERS.list({ prefix: 'user:', cursor });

        for (const key of result.keys) {
            const userData = await env.USERS.get(key.name);
            if (userData) {
                const user = JSON.parse(userData) as User;
                // Remove sensitive data
                users.push({
                    ...user,
                    apiKey: user.apiKey.substring(0, 12) + '...'  // Mask API key
                });
            }
        }

        cursor = result.list_complete ? undefined : result.cursor;
    } while (cursor);

    return users;
}

/**
 * Get platform-wide statistics (admin only)
 */
export async function getPlatformStats(env: Env): Promise<{
    totalUsers: number;
    usersByTier: Record<string, number>;
    totalVectors: number;
    recentSignups: number;
    verifiedUsers: number;
}> {
    const users = await listAllUsers(env);

    const usersByTier: Record<string, number> = {
        free: 0,
        starter: 0,
        pro: 0,
        business: 0
    };

    let totalVectors = 0;
    let recentSignups = 0;
    let verifiedUsers = 0;

    const oneDayAgo = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();

    for (const user of users) {
        usersByTier[user.tier] = (usersByTier[user.tier] || 0) + 1;

        if (user.createdAt > oneDayAgo) {
            recentSignups++;
        }

        // Check if verified
        const verified = await env.USERS.get(`verified:${user.id}`);
        if (verified === 'true') {
            verifiedUsers++;
        }

        // Get vector count from user's database
        const dbData = await env.VECTORS.get(`db:${user.id}:default`);
        if (dbData) {
            try {
                const db = JSON.parse(dbData);
                totalVectors += db.ids?.length || 0;
            } catch {}
        }
    }

    return {
        totalUsers: users.length,
        usersByTier,
        totalVectors,
        recentSignups,
        verifiedUsers
    };
}

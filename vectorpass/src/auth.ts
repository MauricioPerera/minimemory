/**
 * VectorPass Authentication & User Management
 */

import { Env, User, Tier, generateApiKey, generateReferralCode, REFERRAL_CONFIG, ADMIN_EMAILS, TIER_LIMITS } from './types';

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
 * Get user by ID (admin only)
 */
export async function getUserById(userId: string, env: Env): Promise<User | null> {
    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return null;
    return JSON.parse(userData) as User;
}

/**
 * Update user data (admin only)
 */
export async function adminUpdateUser(
    userId: string,
    updates: Partial<Pick<User, 'tier' | 'email' | 'referralDiscount' | 'paymentFailedAt' | 'previousTier'>>,
    env: Env
): Promise<User | null> {
    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return null;

    const user = JSON.parse(userData) as User;
    const oldEmail = user.email;

    // Apply updates
    if (updates.tier !== undefined) user.tier = updates.tier;
    if (updates.email !== undefined) user.email = updates.email;
    if (updates.referralDiscount !== undefined) user.referralDiscount = updates.referralDiscount;
    if (updates.paymentFailedAt !== undefined) user.paymentFailedAt = updates.paymentFailedAt;
    if (updates.previousTier !== undefined) user.previousTier = updates.previousTier;

    // Save updated user
    await env.USERS.put(`user:${userId}`, JSON.stringify(user));

    // Update email lookup if email changed
    if (updates.email && updates.email !== oldEmail) {
        await env.USERS.delete(`email:${oldEmail}`);
        await env.USERS.put(`email:${updates.email}`, userId);
    }

    return user;
}

/**
 * Delete user and all associated data (admin only)
 */
export async function adminDeleteUser(userId: string, env: Env): Promise<boolean> {
    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return false;

    const user = JSON.parse(userData) as User;

    // Delete user data
    await env.USERS.delete(`user:${userId}`);

    // Delete API key lookup
    await env.USERS.delete(`apikey:${user.apiKey}`);

    // Delete email lookup
    await env.USERS.delete(`email:${user.email}`);

    // Delete referral code lookup
    if (user.referralCode) {
        await env.USERS.delete(`referral:${user.referralCode}`);
    }

    // Delete Stripe customer mapping
    if (user.stripeCustomerId) {
        await env.USERS.delete(`stripe:${user.stripeCustomerId}`);
    }

    // Delete verification status
    await env.USERS.delete(`verified:${userId}`);

    // Delete vector database
    await env.VECTORS.delete(`db:${userId}:default`);

    // Delete rate limit data
    await env.RATE_LIMITS.delete(`search:${userId}`);

    console.log(`Admin deleted user ${user.email} (${userId})`);
    return true;
}

/**
 * Trim excess vectors for a user who exceeded their tier limit
 * Called after 15-day grace period expires
 */
export async function trimExcessVectors(userId: string, env: Env): Promise<{ deleted: number; remaining: number }> {
    const userData = await env.USERS.get(`user:${userId}`);
    if (!userData) return { deleted: 0, remaining: 0 };

    const user = JSON.parse(userData) as User;
    const limits = TIER_LIMITS[user.tier];
    const maxVectors = limits.maxVectors;

    // Get user's vector database
    const dbData = await env.VECTORS.get(`db:${userId}:default`);
    if (!dbData) return { deleted: 0, remaining: 0 };

    try {
        const db = JSON.parse(dbData);
        const currentCount = db.ids?.length || 0;

        if (currentCount <= maxVectors) {
            return { deleted: 0, remaining: currentCount };
        }

        // Need to trim vectors - delete oldest ones first
        const toDelete = currentCount - maxVectors;
        const idsToKeep = db.ids.slice(-maxVectors);  // Keep newest

        // Rebuild database with only kept vectors
        const keptIndices = new Set(idsToKeep.map((id: string) => db.ids.indexOf(id)));

        db.ids = idsToKeep;
        if (db.vectors) {
            db.vectors = db.vectors.filter((_: any, i: number) => keptIndices.has(i));
        }
        if (db.metadata) {
            const newMetadata: Record<string, any> = {};
            for (const id of idsToKeep) {
                if (db.metadata[id]) {
                    newMetadata[id] = db.metadata[id];
                }
            }
            db.metadata = newMetadata;
        }

        await env.VECTORS.put(`db:${userId}:default`, JSON.stringify(db));

        console.log(`Trimmed ${toDelete} excess vectors for user ${user.email}. Remaining: ${maxVectors}`);
        return { deleted: toDelete, remaining: maxVectors };
    } catch (err) {
        console.error('Error trimming vectors:', err);
        return { deleted: 0, remaining: 0 };
    }
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

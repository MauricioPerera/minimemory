/**
 * VectorPass Rate Limiting
 */

import { Env, User, RateLimitData, TIER_LIMITS } from './types';

/**
 * Gets the start of the current day (UTC) as timestamp
 */
function getDayStart(): number {
    const now = new Date();
    return new Date(Date.UTC(
        now.getUTCFullYear(),
        now.getUTCMonth(),
        now.getUTCDate()
    )).getTime();
}

/**
 * Gets rate limit data for a user
 */
async function getRateLimitData(
    userId: string,
    type: 'search' | 'index',
    env: Env
): Promise<RateLimitData> {
    const key = `${type}:${userId}`;
    const data = await env.RATE_LIMITS.get(key);

    if (!data) {
        return {
            count: 0,
            resetAt: getDayStart() + 86400000  // End of day
        };
    }

    const parsed = JSON.parse(data) as RateLimitData;

    // Reset if past reset time
    if (Date.now() >= parsed.resetAt) {
        return {
            count: 0,
            resetAt: getDayStart() + 86400000
        };
    }

    return parsed;
}

/**
 * Increments rate limit counter
 */
async function incrementRateLimit(
    userId: string,
    type: 'search' | 'index',
    amount: number,
    env: Env
): Promise<void> {
    const key = `${type}:${userId}`;
    const data = await getRateLimitData(userId, type, env);

    data.count += amount;

    // TTL until end of day
    const ttl = Math.ceil((data.resetAt - Date.now()) / 1000);

    await env.RATE_LIMITS.put(key, JSON.stringify(data), {
        expirationTtl: Math.max(ttl, 60)  // At least 60 seconds
    });
}

/**
 * Checks search rate limit
 */
export async function checkSearchLimit(user: User, env: Env): Promise<{
    allowed: boolean;
    remaining: number;
    resetAt: number;
}> {
    const limits = TIER_LIMITS[user.tier];
    const data = await getRateLimitData(user.id, 'search', env);

    const remaining = Math.max(0, limits.searchesPerDay - data.count);

    return {
        allowed: data.count < limits.searchesPerDay,
        remaining,
        resetAt: data.resetAt
    };
}

/**
 * Checks vector count limit
 */
export async function checkVectorLimit(
    user: User,
    currentCount: number,
    toAdd: number
): Promise<{
    allowed: boolean;
    remaining: number;
    max: number;
}> {
    const limits = TIER_LIMITS[user.tier];
    const newTotal = currentCount + toAdd;

    return {
        allowed: newTotal <= limits.maxVectors,
        remaining: Math.max(0, limits.maxVectors - currentCount),
        max: limits.maxVectors
    };
}

/**
 * Records a search operation
 */
export async function recordSearch(user: User, env: Env): Promise<void> {
    await incrementRateLimit(user.id, 'search', 1, env);
}

/**
 * Gets usage stats for user
 */
export async function getUsageStats(user: User, env: Env): Promise<{
    searchesToday: number;
    searchesLimit: number;
    searchesRemaining: number;
    resetAt: string;
}> {
    const limits = TIER_LIMITS[user.tier];
    const searchData = await getRateLimitData(user.id, 'search', env);

    return {
        searchesToday: searchData.count,
        searchesLimit: limits.searchesPerDay,
        searchesRemaining: Math.max(0, limits.searchesPerDay - searchData.count),
        resetAt: new Date(searchData.resetAt).toISOString()
    };
}

/**
 * Rate limit response headers
 */
export function rateLimitHeaders(remaining: number, resetAt: number): Record<string, string> {
    return {
        'X-RateLimit-Remaining': remaining.toString(),
        'X-RateLimit-Reset': Math.ceil(resetAt / 1000).toString(),
        'X-RateLimit-Reset-Date': new Date(resetAt).toISOString()
    };
}

/**
 * Rate limit exceeded response
 */
export function rateLimitExceeded(resetAt: number): Response {
    return new Response(JSON.stringify({
        success: false,
        error: 'Rate limit exceeded',
        resetAt: new Date(resetAt).toISOString()
    }), {
        status: 429,
        headers: {
            'Content-Type': 'application/json',
            'Retry-After': Math.ceil((resetAt - Date.now()) / 1000).toString(),
            ...rateLimitHeaders(0, resetAt)
        }
    });
}

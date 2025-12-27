/**
 * Rate Limiting Middleware
 */
/**
 * In-memory rate limiter
 * For production, use Redis or Cloudflare Rate Limiting
 * Note: Workers-compatible - no setInterval in global scope
 */
export class RateLimiter {
    entries = new Map();
    lastCleanup = 0;
    cleanupInterval = null;
    constructor() {
        // Don't use setInterval in constructor for Workers compatibility
        // Cleanup is done lazily on each check() call
    }
    /**
     * Check and increment rate limit
     */
    check(key, limit, windowSeconds) {
        const now = Date.now();
        const windowMs = windowSeconds * 1000;
        const resetAt = now + windowMs;
        // Lazy cleanup every 60 seconds
        if (now - this.lastCleanup > 60000) {
            this.cleanup();
            this.lastCleanup = now;
        }
        let entry = this.entries.get(key);
        // If no entry or expired, create new
        if (!entry || entry.resetAt <= now) {
            entry = { count: 1, resetAt };
            this.entries.set(key, entry);
            return {
                limit,
                remaining: limit - 1,
                reset: Math.floor(resetAt / 1000),
                retryAfter: 0,
            };
        }
        // Increment count
        entry.count++;
        const remaining = Math.max(0, limit - entry.count);
        const retryAfter = entry.count > limit
            ? Math.ceil((entry.resetAt - now) / 1000)
            : 0;
        return {
            limit,
            remaining,
            reset: Math.floor(entry.resetAt / 1000),
            retryAfter,
        };
    }
    /**
     * Clean up expired entries
     */
    cleanup() {
        const now = Date.now();
        for (const [key, entry] of this.entries) {
            if (entry.resetAt <= now) {
                this.entries.delete(key);
            }
        }
    }
    /**
     * Reset a specific key
     */
    reset(key) {
        this.entries.delete(key);
    }
    /**
     * Get current stats
     */
    stats() {
        return { activeKeys: this.entries.size };
    }
    /**
     * Start periodic cleanup (for Node.js environments)
     * Not needed for Workers - they are stateless
     */
    startCleanup() {
        if (!this.cleanupInterval && typeof setInterval !== 'undefined') {
            this.cleanupInterval = setInterval(() => this.cleanup(), 60000);
        }
    }
    /**
     * Destroy the rate limiter
     */
    destroy() {
        if (this.cleanupInterval) {
            clearInterval(this.cleanupInterval);
            this.cleanupInterval = null;
        }
        this.entries.clear();
    }
}
// Default rate limiter instance
export const defaultRateLimiter = new RateLimiter();
/**
 * Create rate limiting middleware
 */
export function createRateLimitMiddleware(config) {
    const defaultLimit = config?.defaultLimit || 100;
    const defaultWindow = config?.defaultWindow || 60;
    const keyGenerator = config?.keyGenerator || defaultKeyGenerator;
    const skip = config?.skip || (() => false);
    const onLimit = config?.onLimit || defaultOnLimit;
    return async (c, next) => {
        // Check if we should skip
        if (skip(c)) {
            await next();
            return;
        }
        // Get rate limit config from auth or use defaults
        const auth = c.get('auth');
        const limit = auth?.rateLimit?.limit || defaultLimit;
        const window = auth?.rateLimit?.window || defaultWindow;
        // Generate key
        const key = keyGenerator(c);
        // Check rate limit
        const info = defaultRateLimiter.check(key, limit, window);
        // Set rate limit headers
        c.header('X-RateLimit-Limit', String(info.limit));
        c.header('X-RateLimit-Remaining', String(info.remaining));
        c.header('X-RateLimit-Reset', String(info.reset));
        // If over limit, return error
        if (info.retryAfter > 0) {
            c.header('Retry-After', String(info.retryAfter));
            return onLimit(c, info);
        }
        await next();
    };
}
/**
 * Default key generator - uses API key or IP
 */
function defaultKeyGenerator(c) {
    const auth = c.get('auth');
    if (auth?.userId) {
        return `user:${auth.userId}`;
    }
    // Fallback to IP (for Cloudflare, use CF-Connecting-IP)
    const ip = c.req.header('CF-Connecting-IP')
        || c.req.header('X-Forwarded-For')?.split(',')[0]
        || 'unknown';
    return `ip:${ip}`;
}
/**
 * Default rate limit exceeded handler
 */
function defaultOnLimit(c, info) {
    return c.json({
        error: 'Too Many Requests',
        message: `Rate limit exceeded. Try again in ${info.retryAfter} seconds.`,
        retryAfter: info.retryAfter,
    }, 429);
}
/**
 * Create a stricter rate limit for specific routes
 */
export function strictRateLimit(limit, windowSeconds) {
    return async (c, next) => {
        const auth = c.get('auth');
        const key = auth?.userId
            ? `strict:${auth.userId}:${c.req.path}`
            : `strict:anonymous:${c.req.path}`;
        const info = defaultRateLimiter.check(key, limit, windowSeconds);
        c.header('X-RateLimit-Limit', String(info.limit));
        c.header('X-RateLimit-Remaining', String(info.remaining));
        c.header('X-RateLimit-Reset', String(info.reset));
        if (info.retryAfter > 0) {
            c.header('Retry-After', String(info.retryAfter));
            return c.json({
                error: 'Too Many Requests',
                message: `Rate limit exceeded. Try again in ${info.retryAfter} seconds.`,
                retryAfter: info.retryAfter,
            }, 429);
        }
        await next();
    };
}
//# sourceMappingURL=rateLimit.js.map
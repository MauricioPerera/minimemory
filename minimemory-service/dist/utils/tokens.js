// JWT Token utilities using jose
import { SignJWT, jwtVerify } from 'jose';
// Token expiration times
export const ACCESS_TOKEN_EXPIRY = '15m'; // 15 minutes
export const REFRESH_TOKEN_EXPIRY = '7d'; // 7 days
/**
 * Generate a cryptographically secure random ID
 */
export function generateId() {
    const array = new Uint8Array(16);
    crypto.getRandomValues(array);
    return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
}
/**
 * Create an access token
 */
export async function createAccessToken(payload, secret) {
    const secretKey = new TextEncoder().encode(secret);
    return new SignJWT(payload)
        .setProtectedHeader({ alg: 'HS256' })
        .setIssuedAt()
        .setExpirationTime(ACCESS_TOKEN_EXPIRY)
        .sign(secretKey);
}
/**
 * Create a refresh token
 */
export async function createRefreshToken(userId, sessionId, secret) {
    const secretKey = new TextEncoder().encode(secret);
    return new SignJWT({ sub: userId, jti: sessionId })
        .setProtectedHeader({ alg: 'HS256' })
        .setIssuedAt()
        .setExpirationTime(REFRESH_TOKEN_EXPIRY)
        .sign(secretKey);
}
/**
 * Verify and decode an access token
 */
export async function verifyAccessToken(token, secret) {
    try {
        const secretKey = new TextEncoder().encode(secret);
        const { payload } = await jwtVerify(token, secretKey);
        return payload;
    }
    catch {
        return null;
    }
}
/**
 * Verify and decode a refresh token
 */
export async function verifyRefreshToken(token, secret) {
    try {
        const secretKey = new TextEncoder().encode(secret);
        const { payload } = await jwtVerify(token, secretKey);
        return payload;
    }
    catch {
        return null;
    }
}
/**
 * Hash a refresh token for storage
 */
export async function hashRefreshToken(token) {
    // Use a simple hash for refresh tokens since they're already cryptographically secure
    const encoder = new TextEncoder();
    const data = encoder.encode(token);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}
/**
 * Calculate refresh token expiry timestamp
 */
export function getRefreshTokenExpiry() {
    // 7 days from now in milliseconds
    return Date.now() + 7 * 24 * 60 * 60 * 1000;
}
//# sourceMappingURL=tokens.js.map
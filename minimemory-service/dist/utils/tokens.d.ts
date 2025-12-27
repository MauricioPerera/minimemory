import { JWTPayload } from 'jose';
export declare const ACCESS_TOKEN_EXPIRY = "15m";
export declare const REFRESH_TOKEN_EXPIRY = "7d";
export interface TenantInfo {
    id: string;
    name: string;
    role: 'owner' | 'admin' | 'member' | 'viewer';
}
export interface AccessTokenPayload extends JWTPayload {
    sub: string;
    email: string;
    name: string;
    tenants: TenantInfo[];
}
export interface RefreshTokenPayload extends JWTPayload {
    sub: string;
    jti: string;
}
/**
 * Generate a cryptographically secure random ID
 */
export declare function generateId(): string;
/**
 * Create an access token
 */
export declare function createAccessToken(payload: Omit<AccessTokenPayload, 'iat' | 'exp'>, secret: string): Promise<string>;
/**
 * Create a refresh token
 */
export declare function createRefreshToken(userId: string, sessionId: string, secret: string): Promise<string>;
/**
 * Verify and decode an access token
 */
export declare function verifyAccessToken(token: string, secret: string): Promise<AccessTokenPayload | null>;
/**
 * Verify and decode a refresh token
 */
export declare function verifyRefreshToken(token: string, secret: string): Promise<RefreshTokenPayload | null>;
/**
 * Hash a refresh token for storage
 */
export declare function hashRefreshToken(token: string): Promise<string>;
/**
 * Calculate refresh token expiry timestamp
 */
export declare function getRefreshTokenExpiry(): number;
//# sourceMappingURL=tokens.d.ts.map
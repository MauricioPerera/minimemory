/**
 * Hash a password using PBKDF2
 * Returns a string in format: salt:hash
 */
export declare function hashPassword(password: string): Promise<string>;
/**
 * Verify a password against a hash
 */
export declare function verifyPassword(password: string, storedHash: string): Promise<boolean>;
/**
 * Validate password strength
 * Returns error message or null if valid
 */
export declare function validatePassword(password: string): string | null;
/**
 * Validate email format
 */
export declare function validateEmail(email: string): boolean;
//# sourceMappingURL=password.d.ts.map
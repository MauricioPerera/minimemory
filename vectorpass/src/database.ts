/**
 * VectorPass - Database Management
 *
 * Helper functions for managing multiple vector databases per user
 */

import { Env, User, TIER_LIMITS } from './types';

/**
 * Database info returned from list operations
 */
export interface DatabaseInfo {
    name: string;
    vectorCount: number;
    createdAt?: string;
    updatedAt?: string;
}

/**
 * Validate database name format
 * - Alphanumeric, underscores, hyphens only
 * - 1-50 characters
 * - Cannot be empty
 */
export function validateDatabaseName(name: string): { valid: boolean; error?: string } {
    if (!name || typeof name !== 'string') {
        return { valid: false, error: 'Database name is required' };
    }

    if (name.length > 50) {
        return { valid: false, error: 'Database name must be 50 characters or less' };
    }

    if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
        return { valid: false, error: 'Database name can only contain letters, numbers, underscores, and hyphens' };
    }

    return { valid: true };
}

/**
 * List all databases for a user
 */
export async function listUserDatabases(userId: string, env: Env): Promise<DatabaseInfo[]> {
    const databases: DatabaseInfo[] = [];
    let cursor: string | undefined;

    do {
        const result = await env.VECTORS.list({
            prefix: `db:${userId}:`,
            cursor
        });

        for (const key of result.keys) {
            // Extract database name from key: db:{userId}:{dbName}
            const parts = key.name.split(':');
            if (parts.length >= 3) {
                const dbName = parts.slice(2).join(':'); // Handle names with colons (unlikely but safe)

                // Get vector count
                const dbData = await env.VECTORS.get(key.name);
                let vectorCount = 0;

                if (dbData) {
                    try {
                        const db = JSON.parse(dbData);
                        vectorCount = db.ids?.length || 0;
                    } catch {}
                }

                databases.push({
                    name: dbName,
                    vectorCount
                });
            }
        }

        cursor = result.list_complete ? undefined : result.cursor;
    } while (cursor);

    // Sort alphabetically, but keep 'default' first
    return databases.sort((a, b) => {
        if (a.name === 'default') return -1;
        if (b.name === 'default') return 1;
        return a.name.localeCompare(b.name);
    });
}

/**
 * Get total vector count across all user's databases
 */
export async function getTotalVectorCount(userId: string, env: Env): Promise<number> {
    let total = 0;
    let cursor: string | undefined;

    do {
        const result = await env.VECTORS.list({
            prefix: `db:${userId}:`,
            cursor
        });

        for (const key of result.keys) {
            const dbData = await env.VECTORS.get(key.name);
            if (dbData) {
                try {
                    const db = JSON.parse(dbData);
                    total += db.ids?.length || 0;
                } catch {}
            }
        }

        cursor = result.list_complete ? undefined : result.cursor;
    } while (cursor);

    return total;
}

/**
 * Get count of databases for a user
 */
export async function getDatabaseCount(userId: string, env: Env): Promise<number> {
    let count = 0;
    let cursor: string | undefined;

    do {
        const result = await env.VECTORS.list({
            prefix: `db:${userId}:`,
            cursor
        });

        count += result.keys.length;
        cursor = result.list_complete ? undefined : result.cursor;
    } while (cursor);

    return count;
}

/**
 * Check if user can create more databases based on tier limits
 */
export async function checkDatabaseLimit(user: User, env: Env): Promise<{ allowed: boolean; current: number; max: number }> {
    const current = await getDatabaseCount(user.id, env);
    const max = TIER_LIMITS[user.tier].maxDatabases;

    return {
        allowed: current < max,
        current,
        max
    };
}

/**
 * Check if a database exists for a user
 */
export async function databaseExists(userId: string, dbName: string, env: Env): Promise<boolean> {
    const data = await env.VECTORS.get(`db:${userId}:${dbName}`);
    return data !== null;
}

/**
 * Delete a database and all its vectors
 */
export async function deleteDatabase(userId: string, dbName: string, env: Env): Promise<boolean> {
    if (dbName === 'default') {
        return false; // Cannot delete default database
    }

    const exists = await databaseExists(userId, dbName, env);
    if (!exists) {
        return false;
    }

    await env.VECTORS.delete(`db:${userId}:${dbName}`);
    return true;
}

/**
 * Delete ALL databases for a user (used when deleting user account)
 */
export async function deleteAllUserDatabases(userId: string, env: Env): Promise<number> {
    let deleted = 0;
    let cursor: string | undefined;

    do {
        const result = await env.VECTORS.list({
            prefix: `db:${userId}:`,
            cursor
        });

        for (const key of result.keys) {
            await env.VECTORS.delete(key.name);
            deleted++;
        }

        cursor = result.list_complete ? undefined : result.cursor;
    } while (cursor);

    return deleted;
}

/**
 * Get info for a specific database
 */
export async function getDatabaseInfo(userId: string, dbName: string, env: Env): Promise<DatabaseInfo | null> {
    const dbData = await env.VECTORS.get(`db:${userId}:${dbName}`);

    if (!dbData) {
        return null;
    }

    try {
        const db = JSON.parse(dbData);
        return {
            name: dbName,
            vectorCount: db.ids?.length || 0
        };
    } catch {
        return null;
    }
}

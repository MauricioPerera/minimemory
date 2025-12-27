import { describe, it, expect } from 'vitest';
import { hashPassword, verifyPassword, validatePassword, validateEmail } from '../../src/utils/password.js';

describe('Password Utilities', () => {
  describe('hashPassword', () => {
    it('should hash a password', async () => {
      const hash = await hashPassword('TestPassword123');
      expect(hash).toBeDefined();
      expect(hash.length).toBeGreaterThan(0);
    });

    it('should produce different hashes for same password', async () => {
      const hash1 = await hashPassword('TestPassword123');
      const hash2 = await hashPassword('TestPassword123');
      expect(hash1).not.toBe(hash2);
    });

    it('should produce hash in salt:hash format', async () => {
      const hash = await hashPassword('TestPassword123');
      expect(hash).toContain(':');
      const parts = hash.split(':');
      expect(parts.length).toBe(2);
    });
  });

  describe('verifyPassword', () => {
    it('should verify correct password', async () => {
      const hash = await hashPassword('TestPassword123');
      const isValid = await verifyPassword('TestPassword123', hash);
      expect(isValid).toBe(true);
    });

    it('should reject incorrect password', async () => {
      const hash = await hashPassword('TestPassword123');
      const isValid = await verifyPassword('WrongPassword', hash);
      expect(isValid).toBe(false);
    });

    it('should reject malformed hash', async () => {
      const isValid = await verifyPassword('TestPassword123', 'invalid-hash');
      expect(isValid).toBe(false);
    });

    it('should reject empty hash', async () => {
      const isValid = await verifyPassword('TestPassword123', '');
      expect(isValid).toBe(false);
    });
  });

  describe('validatePassword', () => {
    it('should accept valid password', () => {
      const error = validatePassword('ValidPass123');
      expect(error).toBeNull();
    });

    it('should reject password shorter than 8 characters', () => {
      const error = validatePassword('Short1A');
      expect(error).not.toBeNull();
      expect(error).toContain('8 characters');
    });

    it('should reject password longer than 128 characters', () => {
      const longPassword = 'A1' + 'a'.repeat(127);
      const error = validatePassword(longPassword);
      expect(error).not.toBeNull();
      expect(error).toContain('128 characters');
    });

    it('should reject password without lowercase letter', () => {
      const error = validatePassword('UPPERCASE123');
      expect(error).not.toBeNull();
      expect(error).toContain('lowercase');
    });

    it('should reject password without uppercase letter', () => {
      const error = validatePassword('lowercase123');
      expect(error).not.toBeNull();
      expect(error).toContain('uppercase');
    });

    it('should reject password without number', () => {
      const error = validatePassword('NoNumbersHere');
      expect(error).not.toBeNull();
      expect(error).toContain('number');
    });
  });

  describe('validateEmail', () => {
    it('should accept valid email', () => {
      expect(validateEmail('test@example.com')).toBe(true);
    });

    it('should accept email with subdomain', () => {
      expect(validateEmail('user@mail.example.com')).toBe(true);
    });

    it('should accept email with plus sign', () => {
      expect(validateEmail('user+tag@example.com')).toBe(true);
    });

    it('should reject email without @', () => {
      expect(validateEmail('invalid-email')).toBe(false);
    });

    it('should reject email without domain', () => {
      expect(validateEmail('user@')).toBe(false);
    });

    it('should reject email without local part', () => {
      expect(validateEmail('@example.com')).toBe(false);
    });

    it('should reject email with spaces', () => {
      expect(validateEmail('user @example.com')).toBe(false);
    });

    it('should reject very long email', () => {
      const longEmail = 'a'.repeat(250) + '@example.com';
      expect(validateEmail(longEmail)).toBe(false);
    });
  });
});

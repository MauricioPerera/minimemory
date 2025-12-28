/**
 * PKCE (Proof Key for Code Exchange) utilities
 * https://datatracker.ietf.org/doc/html/rfc7636
 */

/**
 * Verify PKCE code_verifier against code_challenge
 */
export async function verifyPKCE(
  codeVerifier: string,
  codeChallenge: string,
  codeChallengeMethod: string
): Promise<boolean> {
  if (codeChallengeMethod === 'plain') {
    return codeVerifier === codeChallenge;
  }

  if (codeChallengeMethod === 'S256') {
    const encoder = new TextEncoder();
    const data = encoder.encode(codeVerifier);
    const digest = await crypto.subtle.digest('SHA-256', data);
    const base64 = base64UrlEncode(new Uint8Array(digest));
    return base64 === codeChallenge;
  }

  return false;
}

/**
 * Base64 URL encode (no padding)
 */
export function base64UrlEncode(buffer: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < buffer.length; i++) {
    binary += String.fromCharCode(buffer[i]);
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

/**
 * Generate a random string for tokens/codes
 */
export function generateRandomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  const array = new Uint8Array(length);
  crypto.getRandomValues(array);
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars[array[i] % chars.length];
  }
  return result;
}

/**
 * Generate a 6-digit verification code
 */
export function generateVerificationCode(): string {
  const array = new Uint8Array(3);
  crypto.getRandomValues(array);
  const num = (array[0] << 16) | (array[1] << 8) | array[2];
  return String(num % 1000000).padStart(6, '0');
}

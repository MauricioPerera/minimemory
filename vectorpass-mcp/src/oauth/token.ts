/**
 * OAuth Token Endpoint
 * Exchanges authorization code for access token
 */

import { Env, OAuthCode, OAuthToken, TokenResponse, OAuthError } from '../types';
import { TokenRequest } from './types';
import { verifyPKCE, generateRandomString } from './pkce';

const ACCESS_TOKEN_TTL = 86400; // 24 hours
const REFRESH_TOKEN_TTL = 2592000; // 30 days

/**
 * Handle POST /token
 */
export async function handleToken(
  request: Request,
  env: Env
): Promise<Response> {
  // Parse form data or JSON
  let params: TokenRequest;

  const contentType = request.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    params = await request.json();
  } else {
    const formData = await request.formData();
    params = {
      grant_type: formData.get('grant_type') as string,
      code: formData.get('code') as string,
      redirect_uri: formData.get('redirect_uri') as string,
      code_verifier: formData.get('code_verifier') as string,
      refresh_token: formData.get('refresh_token') as string,
      client_id: formData.get('client_id') as string,
    };
  }

  // Handle different grant types
  switch (params.grant_type) {
    case 'authorization_code':
      return handleAuthorizationCode(params, env);
    case 'refresh_token':
      return handleRefreshToken(params, env);
    default:
      return errorResponse({
        error: 'unsupported_grant_type',
        error_description: 'Only authorization_code and refresh_token grants are supported',
      });
  }
}

/**
 * Exchange authorization code for tokens
 */
async function handleAuthorizationCode(
  params: TokenRequest,
  env: Env
): Promise<Response> {
  const { code, redirect_uri, code_verifier } = params;

  // Validate required params
  if (!code) {
    return errorResponse({
      error: 'invalid_request',
      error_description: 'Missing code parameter',
    });
  }

  if (!code_verifier) {
    return errorResponse({
      error: 'invalid_request',
      error_description: 'Missing code_verifier parameter (PKCE required)',
    });
  }

  // Get authorization code data
  const codeData = await env.OAUTH_CODES.get(`code:${code}`);
  if (!codeData) {
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'Invalid or expired authorization code',
    });
  }

  const authCode: OAuthCode = JSON.parse(codeData);

  // Check expiration
  if (Date.now() > authCode.expiresAt) {
    await env.OAUTH_CODES.delete(`code:${code}`);
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'Authorization code has expired',
    });
  }

  // Validate redirect_uri
  if (redirect_uri && redirect_uri !== authCode.redirectUri) {
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'redirect_uri mismatch',
    });
  }

  // Verify PKCE
  const pkceValid = await verifyPKCE(
    code_verifier,
    authCode.codeChallenge,
    authCode.codeChallengeMethod
  );

  if (!pkceValid) {
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'Invalid code_verifier',
    });
  }

  // Delete the authorization code (one-time use)
  await env.OAUTH_CODES.delete(`code:${code}`);

  // Get user data for token
  const userData = await env.USERS.get(`user:${authCode.userId}`);
  if (!userData) {
    return errorResponse({
      error: 'server_error',
      error_description: 'User not found',
    });
  }

  const user = JSON.parse(userData);

  // Generate tokens
  const accessToken = `vp_access_${generateRandomString(40)}`;
  const refreshToken = `vp_refresh_${generateRandomString(40)}`;

  // Store access token
  const tokenData: OAuthToken = {
    userId: authCode.userId,
    apiKey: authCode.apiKey,
    email: user.email,
    tier: user.tier,
    expiresAt: Date.now() + ACCESS_TOKEN_TTL * 1000,
  };

  await env.OAUTH_TOKENS.put(
    `access:${accessToken}`,
    JSON.stringify(tokenData),
    { expirationTtl: ACCESS_TOKEN_TTL }
  );

  // Store refresh token
  await env.OAUTH_TOKENS.put(
    `refresh:${refreshToken}`,
    JSON.stringify({
      userId: authCode.userId,
      expiresAt: Date.now() + REFRESH_TOKEN_TTL * 1000,
    }),
    { expirationTtl: REFRESH_TOKEN_TTL }
  );

  const response: TokenResponse = {
    access_token: accessToken,
    token_type: 'Bearer',
    expires_in: ACCESS_TOKEN_TTL,
    refresh_token: refreshToken,
  };

  return new Response(JSON.stringify(response), {
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-store',
      'Pragma': 'no-cache',
    },
  });
}

/**
 * Refresh access token using refresh token
 */
async function handleRefreshToken(
  params: TokenRequest,
  env: Env
): Promise<Response> {
  const { refresh_token } = params;

  if (!refresh_token) {
    return errorResponse({
      error: 'invalid_request',
      error_description: 'Missing refresh_token parameter',
    });
  }

  // Get refresh token data
  const refreshData = await env.OAUTH_TOKENS.get(`refresh:${refresh_token}`);
  if (!refreshData) {
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'Invalid or expired refresh token',
    });
  }

  const refresh = JSON.parse(refreshData);

  // Check expiration
  if (Date.now() > refresh.expiresAt) {
    await env.OAUTH_TOKENS.delete(`refresh:${refresh_token}`);
    return errorResponse({
      error: 'invalid_grant',
      error_description: 'Refresh token has expired',
    });
  }

  // Get current user data
  const userData = await env.USERS.get(`user:${refresh.userId}`);
  if (!userData) {
    return errorResponse({
      error: 'server_error',
      error_description: 'User not found',
    });
  }

  const user = JSON.parse(userData);

  // Generate new access token
  const accessToken = `vp_access_${generateRandomString(40)}`;

  const tokenData: OAuthToken = {
    userId: user.id,
    apiKey: user.apiKey,
    email: user.email,
    tier: user.tier,
    expiresAt: Date.now() + ACCESS_TOKEN_TTL * 1000,
  };

  await env.OAUTH_TOKENS.put(
    `access:${accessToken}`,
    JSON.stringify(tokenData),
    { expirationTtl: ACCESS_TOKEN_TTL }
  );

  const response: TokenResponse = {
    access_token: accessToken,
    token_type: 'Bearer',
    expires_in: ACCESS_TOKEN_TTL,
  };

  return new Response(JSON.stringify(response), {
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-store',
      'Pragma': 'no-cache',
    },
  });
}

/**
 * Return OAuth error response
 */
function errorResponse(error: OAuthError, status: number = 400): Response {
  return new Response(JSON.stringify(error), {
    status,
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-store',
    },
  });
}

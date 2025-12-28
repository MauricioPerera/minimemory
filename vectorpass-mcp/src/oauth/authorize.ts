/**
 * OAuth Authorization Endpoint
 * Handles the login flow with email verification
 */

import { Env, OAuthSession, User, VerificationData } from '../types';
import { AuthorizeRequest } from './types';
import { generateRandomString, generateVerificationCode } from './pkce';
import { renderLoginPage } from '../pages/login';
import { renderVerifyPage } from '../pages/verify';

const SESSION_TTL = 900; // 15 minutes
const CODE_TTL = 600; // 10 minutes
const VERIFICATION_TTL = 900; // 15 minutes

/**
 * Handle GET /authorize - Show login page or process OAuth params
 */
export async function handleAuthorizeGet(
  request: Request,
  env: Env
): Promise<Response> {
  const url = new URL(request.url);

  // Check for restart parameter
  const restart = url.searchParams.get('restart');
  if (restart) {
    // Clear the session and start fresh
    await env.OAUTH_SESSIONS.delete(`session:${restart}`);
  }

  // Parse OAuth parameters
  const params: AuthorizeRequest = {
    response_type: url.searchParams.get('response_type') || '',
    client_id: url.searchParams.get('client_id') || '',
    redirect_uri: url.searchParams.get('redirect_uri') || '',
    state: url.searchParams.get('state') || '',
    code_challenge: url.searchParams.get('code_challenge') || '',
    code_challenge_method: url.searchParams.get('code_challenge_method') || 'S256',
    scope: url.searchParams.get('scope') || '',
  };

  // Validate required parameters
  const validationError = validateAuthorizeParams(params, env);
  if (validationError) {
    return new Response(renderLoginPage('', validationError), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Create a new session
  const sessionId = generateRandomString(32);
  const session: OAuthSession = {
    clientId: params.client_id,
    redirectUri: params.redirect_uri,
    state: params.state,
    codeChallenge: params.code_challenge,
    codeChallengeMethod: params.code_challenge_method,
    verified: false,
    createdAt: Date.now(),
  };

  await env.OAUTH_SESSIONS.put(
    `session:${sessionId}`,
    JSON.stringify(session),
    { expirationTtl: SESSION_TTL }
  );

  return new Response(renderLoginPage(sessionId), {
    headers: { 'Content-Type': 'text/html' },
  });
}

/**
 * Handle POST /authorize - Process login steps
 */
export async function handleAuthorizePost(
  request: Request,
  env: Env
): Promise<Response> {
  const formData = await request.formData();
  const sessionId = formData.get('session_id') as string;
  const step = formData.get('step') as string;

  if (!sessionId) {
    return new Response(renderLoginPage('', 'Invalid session'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Get session
  const sessionData = await env.OAUTH_SESSIONS.get(`session:${sessionId}`);
  if (!sessionData) {
    return new Response(renderLoginPage('', 'Session expired. Please start again.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  const session: OAuthSession = JSON.parse(sessionData);

  switch (step) {
    case 'email':
      return handleEmailStep(formData, session, sessionId, env);
    case 'verify':
      return handleVerifyStep(formData, session, sessionId, env);
    case 'resend':
      return handleResendStep(session, sessionId, env);
    default:
      return new Response(renderLoginPage(sessionId, 'Invalid step'), {
        status: 400,
        headers: { 'Content-Type': 'text/html' },
      });
  }
}

/**
 * Step 1: User submits email
 */
async function handleEmailStep(
  formData: FormData,
  session: OAuthSession,
  sessionId: string,
  env: Env
): Promise<Response> {
  const email = (formData.get('email') as string || '').toLowerCase().trim();

  if (!email || !isValidEmail(email)) {
    return new Response(renderLoginPage(sessionId, 'Please enter a valid email address'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Look up user in VectorPass
  const userIdData = await env.USERS.get(`email:${email}`);
  if (!userIdData) {
    return new Response(renderLoginPage(sessionId, 'No account found with this email. Please register at vectorpass.com first.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Get user data
  const userData = await env.USERS.get(`user:${userIdData}`);
  if (!userData) {
    return new Response(renderLoginPage(sessionId, 'Account error. Please contact support.'), {
      status: 500,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  const user: User = JSON.parse(userData);

  // Generate and store verification code
  const code = generateVerificationCode();
  const verification: VerificationData = {
    code,
    attempts: 0,
    createdAt: Date.now(),
  };

  await env.OAUTH_SESSIONS.put(
    `verify:${sessionId}`,
    JSON.stringify(verification),
    { expirationTtl: VERIFICATION_TTL }
  );

  // Update session with email and userId
  session.email = email;
  session.userId = user.id;
  await env.OAUTH_SESSIONS.put(
    `session:${sessionId}`,
    JSON.stringify(session),
    { expirationTtl: SESSION_TTL }
  );

  // Send verification email
  await sendVerificationEmail(email, code, env);

  return new Response(renderVerifyPage(sessionId, email), {
    headers: { 'Content-Type': 'text/html' },
  });
}

/**
 * Step 2: User submits verification code
 */
async function handleVerifyStep(
  formData: FormData,
  session: OAuthSession,
  sessionId: string,
  env: Env
): Promise<Response> {
  const code = (formData.get('code') as string || '').trim();

  if (!session.email || !session.userId) {
    return new Response(renderLoginPage(sessionId, 'Session invalid. Please start again.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Get verification data
  const verifyData = await env.OAUTH_SESSIONS.get(`verify:${sessionId}`);
  if (!verifyData) {
    return new Response(renderVerifyPage(sessionId, session.email, 'Code expired. Please request a new one.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  const verification: VerificationData = JSON.parse(verifyData);

  // Check attempts
  if (verification.attempts >= 5) {
    return new Response(renderVerifyPage(sessionId, session.email, 'Too many attempts. Please request a new code.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Verify code
  if (code !== verification.code) {
    verification.attempts++;
    await env.OAUTH_SESSIONS.put(
      `verify:${sessionId}`,
      JSON.stringify(verification),
      { expirationTtl: VERIFICATION_TTL }
    );
    return new Response(renderVerifyPage(sessionId, session.email, 'Invalid code. Please try again.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Get user's API key
  const userData = await env.USERS.get(`user:${session.userId}`);
  if (!userData) {
    return new Response(renderVerifyPage(sessionId, session.email, 'User not found. Please contact support.'), {
      status: 500,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  const user: User = JSON.parse(userData);

  // Generate authorization code
  const authCode = `vp_code_${generateRandomString(32)}`;
  await env.OAUTH_CODES.put(
    `code:${authCode}`,
    JSON.stringify({
      userId: user.id,
      apiKey: user.apiKey,
      clientId: session.clientId,
      redirectUri: session.redirectUri,
      codeChallenge: session.codeChallenge,
      codeChallengeMethod: session.codeChallengeMethod,
      expiresAt: Date.now() + CODE_TTL * 1000,
    }),
    { expirationTtl: CODE_TTL }
  );

  // Clean up session
  await env.OAUTH_SESSIONS.delete(`session:${sessionId}`);
  await env.OAUTH_SESSIONS.delete(`verify:${sessionId}`);

  // Redirect back to client with code
  const redirectUrl = new URL(session.redirectUri);
  redirectUrl.searchParams.set('code', authCode);
  redirectUrl.searchParams.set('state', session.state);

  return Response.redirect(redirectUrl.toString(), 302);
}

/**
 * Resend verification code
 */
async function handleResendStep(
  session: OAuthSession,
  sessionId: string,
  env: Env
): Promise<Response> {
  if (!session.email) {
    return new Response(renderLoginPage(sessionId, 'Please enter your email first.'), {
      status: 400,
      headers: { 'Content-Type': 'text/html' },
    });
  }

  // Generate new code
  const code = generateVerificationCode();
  const verification: VerificationData = {
    code,
    attempts: 0,
    createdAt: Date.now(),
  };

  await env.OAUTH_SESSIONS.put(
    `verify:${sessionId}`,
    JSON.stringify(verification),
    { expirationTtl: VERIFICATION_TTL }
  );

  // Send email
  await sendVerificationEmail(session.email, code, env);

  return new Response(renderVerifyPage(sessionId, session.email, 'New code sent!'), {
    headers: { 'Content-Type': 'text/html' },
  });
}

/**
 * Validate OAuth authorize parameters
 */
function validateAuthorizeParams(params: AuthorizeRequest, env: Env): string | null {
  if (params.response_type !== 'code') {
    return 'Invalid response_type. Must be "code".';
  }

  if (!params.client_id) {
    return 'Missing client_id parameter.';
  }

  if (!params.redirect_uri) {
    return 'Missing redirect_uri parameter.';
  }

  // Validate redirect_uri against whitelist
  const allowedUris = env.ALLOWED_REDIRECT_URIS.split(',').map(u => u.trim());
  const isAllowed = allowedUris.some(uri => {
    // Allow exact match or localhost with any port
    if (params.redirect_uri === uri) return true;
    if (params.redirect_uri.startsWith('http://localhost:')) return true;
    if (params.redirect_uri.startsWith('http://127.0.0.1:')) return true;
    return false;
  });

  if (!isAllowed) {
    return 'Invalid redirect_uri.';
  }

  if (!params.code_challenge) {
    return 'Missing code_challenge. PKCE is required.';
  }

  if (params.code_challenge_method !== 'S256' && params.code_challenge_method !== 'plain') {
    return 'Invalid code_challenge_method. Must be "S256" or "plain".';
  }

  return null;
}

function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

/**
 * Send verification email using Resend
 */
async function sendVerificationEmail(email: string, code: string, env: Env): Promise<void> {
  if (!env.EMAIL_API_KEY) {
    console.log(`[DEV] Verification code for ${email}: ${code}`);
    return;
  }

  const html = `
    <div style="font-family: Arial, sans-serif; max-width: 500px; margin: 0 auto; padding: 20px;">
      <div style="text-align: center; margin-bottom: 30px;">
        <h1 style="color: #4f46e5; margin: 0;">VectorPass</h1>
        <p style="color: #6b7280; margin-top: 5px;">MCP Authorization</p>
      </div>
      <div style="background: #f9fafb; border-radius: 8px; padding: 30px; text-align: center;">
        <p style="color: #374151; margin-bottom: 20px;">Your verification code is:</p>
        <div style="font-size: 36px; font-weight: bold; letter-spacing: 8px; color: #111827; font-family: monospace;">
          ${code}
        </div>
        <p style="color: #6b7280; margin-top: 20px; font-size: 14px;">
          This code expires in 15 minutes.
        </p>
      </div>
      <p style="color: #9ca3af; font-size: 12px; text-align: center; margin-top: 30px;">
        If you didn't request this code, you can safely ignore this email.
      </p>
    </div>
  `;

  try {
    const response = await fetch('https://api.resend.com/emails', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${env.EMAIL_API_KEY}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        from: 'VectorPass <noreply@automators.work>',
        to: email,
        subject: 'VectorPass MCP - Verification Code',
        html,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      console.error(`Email send failed: ${response.status} - ${error}`);
    }
  } catch (error) {
    console.error(`Email send error: ${error}`);
  }
}

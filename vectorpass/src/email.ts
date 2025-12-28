/**
 * VectorPass - Email Verification System
 *
 * Handles email verification with 6-digit codes
 * Uses Cloudflare Email Workers or external service (Resend, SendGrid, etc.)
 */

import { Env } from './types';

// Verification code settings
const CODE_LENGTH = 6;
const CODE_EXPIRY_MINUTES = 15;
const MAX_ATTEMPTS = 5;

/**
 * Generate a random 6-digit verification code
 */
function generateCode(): string {
    const array = new Uint32Array(1);
    crypto.getRandomValues(array);
    return String(array[0] % 1000000).padStart(CODE_LENGTH, '0');
}

/**
 * Store verification code in KV
 */
async function storeVerificationCode(
    userId: string,
    code: string,
    env: Env
): Promise<void> {
    const data = {
        code,
        attempts: 0,
        createdAt: Date.now()
    };

    // Store with expiry
    await env.USERS.put(
        `verify:${userId}`,
        JSON.stringify(data),
        { expirationTtl: CODE_EXPIRY_MINUTES * 60 }
    );
}

/**
 * Send verification email
 * Configure EMAIL_SERVICE in wrangler.toml to use different providers
 */
export async function sendVerificationEmail(
    userId: string,
    email: string,
    env: Env
): Promise<boolean> {
    const code = generateCode();
    await storeVerificationCode(userId, code, env);

    // Get email service configuration
    const emailService = (env as any).EMAIL_SERVICE || 'log';
    const emailApiKey = (env as any).EMAIL_API_KEY;
    const fromEmail = (env as any).FROM_EMAIL || 'noreply@vectorpass.automators.work';

    const subject = 'VectorPass - Verification Code';
    const body = `
Your VectorPass verification code is:

${code}

This code expires in ${CODE_EXPIRY_MINUTES} minutes.

If you didn't request this code, you can safely ignore this email.

---
VectorPass - RAG as a Service
https://vectorpass.automators.work
    `.trim();

    const html = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>VectorPass Verification</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="text-align: center; margin-bottom: 30px;">
        <h1 style="color: #2563eb; margin: 0;">VectorPass</h1>
        <p style="color: #666; margin: 5px 0 0 0;">RAG as a Service</p>
    </div>

    <div style="background: #f8fafc; border-radius: 8px; padding: 30px; text-align: center;">
        <p style="margin: 0 0 20px 0; font-size: 16px;">Your verification code is:</p>
        <div style="font-size: 32px; font-weight: bold; letter-spacing: 8px; color: #2563eb; background: white; padding: 20px; border-radius: 8px; border: 2px dashed #2563eb;">
            ${code}
        </div>
        <p style="margin: 20px 0 0 0; font-size: 14px; color: #666;">
            This code expires in ${CODE_EXPIRY_MINUTES} minutes.
        </p>
    </div>

    <p style="font-size: 14px; color: #666; margin-top: 30px; text-align: center;">
        If you didn't request this code, you can safely ignore this email.
    </p>

    <div style="border-top: 1px solid #e2e8f0; margin-top: 30px; padding-top: 20px; text-align: center; font-size: 12px; color: #999;">
        <p style="margin: 0;">
            <a href="https://vectorpass.automators.work" style="color: #2563eb; text-decoration: none;">vectorpass.automators.work</a>
        </p>
    </div>
</body>
</html>
    `.trim();

    try {
        switch (emailService) {
            case 'resend':
                return await sendWithResend(email, fromEmail, subject, body, html, emailApiKey);

            case 'sendgrid':
                return await sendWithSendGrid(email, fromEmail, subject, body, html, emailApiKey);

            case 'mailgun':
                return await sendWithMailgun(email, fromEmail, subject, body, html, emailApiKey, env);

            case 'log':
            default:
                // Development: log code to console
                console.log(`[DEV] Verification code for ${email}: ${code}`);
                return true;
        }
    } catch (error) {
        console.error('Failed to send email:', error);
        return false;
    }
}

/**
 * Send email via Resend API
 */
async function sendWithResend(
    to: string,
    from: string,
    subject: string,
    text: string,
    html: string,
    apiKey: string
): Promise<boolean> {
    const response = await fetch('https://api.resend.com/emails', {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${apiKey}`,
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            from,
            to,
            subject,
            text,
            html
        })
    });

    return response.ok;
}

/**
 * Send email via SendGrid API
 */
async function sendWithSendGrid(
    to: string,
    from: string,
    subject: string,
    text: string,
    html: string,
    apiKey: string
): Promise<boolean> {
    const response = await fetch('https://api.sendgrid.com/v3/mail/send', {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${apiKey}`,
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            personalizations: [{ to: [{ email: to }] }],
            from: { email: from },
            subject,
            content: [
                { type: 'text/plain', value: text },
                { type: 'text/html', value: html }
            ]
        })
    });

    return response.ok;
}

/**
 * Send email via Mailgun API
 */
async function sendWithMailgun(
    to: string,
    from: string,
    subject: string,
    text: string,
    html: string,
    apiKey: string,
    env: Env
): Promise<boolean> {
    const domain = (env as any).MAILGUN_DOMAIN || 'vectorpass.automators.work';

    const formData = new FormData();
    formData.append('from', from);
    formData.append('to', to);
    formData.append('subject', subject);
    formData.append('text', text);
    formData.append('html', html);

    const response = await fetch(`https://api.mailgun.net/v3/${domain}/messages`, {
        method: 'POST',
        headers: {
            'Authorization': `Basic ${btoa(`api:${apiKey}`)}`
        },
        body: formData
    });

    return response.ok;
}

/**
 * Verify email code
 */
export async function verifyEmailCode(
    userId: string,
    code: string,
    env: Env
): Promise<boolean> {
    const key = `verify:${userId}`;
    const stored = await env.USERS.get(key);

    if (!stored) {
        return false;
    }

    const data = JSON.parse(stored);

    // Check max attempts
    if (data.attempts >= MAX_ATTEMPTS) {
        await env.USERS.delete(key);
        return false;
    }

    // Check if expired (double-check in case KV TTL hasn't kicked in)
    const elapsed = (Date.now() - data.createdAt) / 1000 / 60;
    if (elapsed > CODE_EXPIRY_MINUTES) {
        await env.USERS.delete(key);
        return false;
    }

    // Check code
    if (data.code !== code) {
        // Increment attempts
        data.attempts++;
        await env.USERS.put(key, JSON.stringify(data), {
            expirationTtl: Math.ceil((CODE_EXPIRY_MINUTES * 60) - (elapsed * 60))
        });
        return false;
    }

    // Success - mark as verified and delete code
    await env.USERS.delete(key);
    await env.USERS.put(`verified:${userId}`, 'true');

    return true;
}

/**
 * Check if email is verified
 */
export async function isEmailVerified(userId: string, env: Env): Promise<boolean> {
    const verified = await env.USERS.get(`verified:${userId}`);
    return verified === 'true';
}

/**
 * Revoke email verification (for testing or security)
 */
export async function revokeVerification(userId: string, env: Env): Promise<void> {
    await env.USERS.delete(`verified:${userId}`);
}

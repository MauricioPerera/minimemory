/**
 * VectorPass - Stripe Integration
 *
 * Handles subscription webhooks and payment processing
 */

import { Env, User, Tier } from './types';
import { updateUserTier } from './auth';

// Stripe price IDs (created via scripts/setup-stripe.js)
export const STRIPE_PRICES = {
    starter: 'price_1SjCW8DqXeTbPD4KRmH9mmuh',   // $9/mo
    pro: 'price_1SjCW8DqXeTbPD4Ky42FvJje',       // $29/mo
    business: 'price_1SjCW9DqXeTbPD4K7sf81w8R'   // $79/mo
};

// Map Stripe price ID to tier
const PRICE_TO_TIER: Record<string, Tier> = {
    'price_1SjCW8DqXeTbPD4KRmH9mmuh': 'starter',
    'price_1SjCW8DqXeTbPD4Ky42FvJje': 'pro',
    'price_1SjCW9DqXeTbPD4K7sf81w8R': 'business'
};

/**
 * Verify Stripe webhook signature
 */
async function verifyWebhookSignature(
    payload: string,
    signature: string,
    webhookSecret: string
): Promise<boolean> {
    // Stripe uses HMAC-SHA256
    const encoder = new TextEncoder();
    const key = await crypto.subtle.importKey(
        'raw',
        encoder.encode(webhookSecret),
        { name: 'HMAC', hash: 'SHA-256' },
        false,
        ['sign']
    );

    // Parse signature header: t=timestamp,v1=signature
    const parts = signature.split(',');
    const timestamp = parts.find(p => p.startsWith('t='))?.slice(2);
    const v1Sig = parts.find(p => p.startsWith('v1='))?.slice(3);

    if (!timestamp || !v1Sig) {
        return false;
    }

    // Check timestamp is within 5 minutes
    const now = Math.floor(Date.now() / 1000);
    if (Math.abs(now - parseInt(timestamp)) > 300) {
        return false;
    }

    // Compute expected signature
    const signedPayload = `${timestamp}.${payload}`;
    const signatureBytes = await crypto.subtle.sign(
        'HMAC',
        key,
        encoder.encode(signedPayload)
    );

    const expectedSig = Array.from(new Uint8Array(signatureBytes))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

    return expectedSig === v1Sig;
}

/**
 * Handle Stripe webhook events
 */
export async function handleStripeWebhook(
    request: Request,
    env: Env
): Promise<Response> {
    const signature = request.headers.get('Stripe-Signature');
    const webhookSecret = (env as any).STRIPE_WEBHOOK_SECRET;

    if (!signature || !webhookSecret) {
        return new Response(JSON.stringify({ error: 'Missing signature or secret' }), {
            status: 400,
            headers: { 'Content-Type': 'application/json' }
        });
    }

    const payload = await request.text();

    // Verify signature
    const isValid = await verifyWebhookSignature(payload, signature, webhookSecret);
    if (!isValid) {
        return new Response(JSON.stringify({ error: 'Invalid signature' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
        });
    }

    const event = JSON.parse(payload);

    try {
        switch (event.type) {
            case 'checkout.session.completed':
                await handleCheckoutCompleted(event.data.object, env);
                break;

            case 'customer.subscription.created':
            case 'customer.subscription.updated':
                await handleSubscriptionUpdate(event.data.object, env);
                break;

            case 'customer.subscription.deleted':
                await handleSubscriptionCanceled(event.data.object, env);
                break;

            case 'invoice.payment_failed':
                await handlePaymentFailed(event.data.object, env);
                break;

            default:
                console.log(`Unhandled event type: ${event.type}`);
        }

        return new Response(JSON.stringify({ received: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' }
        });

    } catch (error: any) {
        console.error('Webhook error:', error);
        return new Response(JSON.stringify({ error: error.message }), {
            status: 500,
            headers: { 'Content-Type': 'application/json' }
        });
    }
}

/**
 * Handle successful checkout
 */
async function handleCheckoutCompleted(session: any, env: Env): Promise<void> {
    const { customer, subscription, client_reference_id } = session;

    if (!client_reference_id) {
        console.warn('No client_reference_id in checkout session');
        return;
    }

    // client_reference_id should be the user ID
    const userId = client_reference_id;

    // Get subscription details to determine tier
    // In production, you'd call Stripe API to get subscription items
    // For now, we'll handle it in subscription.updated event
    console.log(`Checkout completed for user ${userId}, customer ${customer}`);
}

/**
 * Handle subscription created/updated
 */
async function handleSubscriptionUpdate(subscription: any, env: Env): Promise<void> {
    const { customer, status, items } = subscription;

    if (status !== 'active' && status !== 'trialing') {
        return;
    }

    // Get price ID from subscription items
    const priceId = items?.data?.[0]?.price?.id;
    if (!priceId) {
        console.warn('No price ID in subscription');
        return;
    }

    const tier = PRICE_TO_TIER[priceId] || 'free';

    // Find user by Stripe customer ID
    const userId = await env.USERS.get(`stripe:${customer}`);
    if (!userId) {
        console.warn(`No user found for Stripe customer ${customer}`);
        return;
    }

    // Update user tier
    await updateUserTier(userId, tier, customer, subscription.id, env);
    console.log(`Updated user ${userId} to tier ${tier}`);
}

/**
 * Handle subscription canceled
 */
async function handleSubscriptionCanceled(subscription: any, env: Env): Promise<void> {
    const { customer } = subscription;

    // Find user by Stripe customer ID
    const userId = await env.USERS.get(`stripe:${customer}`);
    if (!userId) {
        return;
    }

    // Downgrade to free tier
    await updateUserTier(userId, 'free', customer, undefined, env);
    console.log(`Downgraded user ${userId} to free tier`);
}

/**
 * Handle failed payment
 */
async function handlePaymentFailed(invoice: any, env: Env): Promise<void> {
    const { customer, subscription } = invoice;

    // Find user by Stripe customer ID
    const userId = await env.USERS.get(`stripe:${customer}`);
    if (!userId) {
        return;
    }

    // Log the failure - in production, send email notification
    console.log(`Payment failed for user ${userId}, subscription ${subscription}`);

    // Optionally downgrade after multiple failures
    // For now, Stripe handles dunning automatically
}

/**
 * Create Stripe Checkout session URL
 * Call this from your frontend to start subscription flow
 */
export function getCheckoutUrl(
    userId: string,
    tier: 'starter' | 'pro' | 'business',
    successUrl: string,
    cancelUrl: string
): string {
    const priceId = STRIPE_PRICES[tier];

    // In production, you'd create this via Stripe API
    // This is a placeholder showing the expected flow
    const params = new URLSearchParams({
        client_reference_id: userId,
        success_url: successUrl,
        cancel_url: cancelUrl,
        mode: 'subscription',
        'line_items[0][price]': priceId,
        'line_items[0][quantity]': '1'
    });

    return `https://checkout.stripe.com/pay?${params.toString()}`;
}

/**
 * Link Stripe customer to user (call after first purchase)
 */
export async function linkStripeCustomer(
    userId: string,
    stripeCustomerId: string,
    env: Env
): Promise<void> {
    await env.USERS.put(`stripe:${stripeCustomerId}`, userId);
}

/**
 * Create a billing portal session URL
 */
export function getBillingPortalUrl(customerId: string, returnUrl: string): string {
    // In production, create via Stripe API
    const params = new URLSearchParams({
        customer: customerId,
        return_url: returnUrl
    });

    return `https://billing.stripe.com/session?${params.toString()}`;
}

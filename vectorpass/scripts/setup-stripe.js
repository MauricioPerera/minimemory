/**
 * Setup Stripe Products and Prices for VectorPass
 *
 * Usage: STRIPE_SECRET_KEY=sk_live_xxx node scripts/setup-stripe.js
 */

const Stripe = require('stripe');

const STRIPE_SECRET_KEY = process.env.STRIPE_SECRET_KEY;

if (!STRIPE_SECRET_KEY) {
    console.error('Error: STRIPE_SECRET_KEY environment variable is required');
    console.error('Usage: STRIPE_SECRET_KEY=sk_live_xxx node scripts/setup-stripe.js');
    process.exit(1);
}

const stripe = new Stripe(STRIPE_SECRET_KEY, {
    apiVersion: '2023-10-16'
});

const PRODUCTS = [
    {
        name: 'VectorPass Starter',
        description: 'Perfect for small projects and prototypes',
        features: [
            '50,000 vectors',
            '10,000 searches/day',
            'Batch size: 100',
            'Email support'
        ],
        price: 900,  // $9.00 in cents
        interval: 'month'
    },
    {
        name: 'VectorPass Pro',
        description: 'For growing applications with higher demands',
        features: [
            '500,000 vectors',
            '100,000 searches/day',
            'Batch size: 500',
            'Priority support'
        ],
        price: 2900,  // $29.00 in cents
        interval: 'month'
    },
    {
        name: 'VectorPass Business',
        description: 'Enterprise-grade for large scale applications',
        features: [
            '5,000,000 vectors',
            '1,000,000 searches/day',
            'Batch size: 1,000',
            'Dedicated support',
            'SLA guarantee'
        ],
        price: 7900,  // $79.00 in cents
        interval: 'month'
    }
];

async function createProducts() {
    console.log('Creating VectorPass products in Stripe...\n');

    const results = [];

    for (const productData of PRODUCTS) {
        try {
            // Create product
            const product = await stripe.products.create({
                name: productData.name,
                description: productData.description,
                metadata: {
                    features: JSON.stringify(productData.features)
                }
            });

            console.log(`✓ Created product: ${product.name} (${product.id})`);

            // Create price
            const price = await stripe.prices.create({
                product: product.id,
                unit_amount: productData.price,
                currency: 'usd',
                recurring: {
                    interval: productData.interval
                },
                metadata: {
                    tier: productData.name.toLowerCase().replace('vectorpass ', '')
                }
            });

            console.log(`  └─ Price: $${productData.price / 100}/mo (${price.id})\n`);

            results.push({
                tier: productData.name.toLowerCase().replace('vectorpass ', ''),
                productId: product.id,
                priceId: price.id,
                amount: productData.price / 100
            });

        } catch (error) {
            console.error(`✗ Error creating ${productData.name}:`, error.message);
        }
    }

    // Output configuration
    console.log('\n' + '='.repeat(60));
    console.log('CONFIGURATION FOR src/stripe.ts:');
    console.log('='.repeat(60) + '\n');

    console.log('export const STRIPE_PRICES = {');
    for (const r of results) {
        console.log(`    ${r.tier}: '${r.priceId}',   // $${r.amount}/mo`);
    }
    console.log('};');

    console.log('\nconst PRICE_TO_TIER: Record<string, Tier> = {');
    for (const r of results) {
        console.log(`    '${r.priceId}': '${r.tier}',`);
    }
    console.log('};');

    console.log('\n' + '='.repeat(60));
    console.log('Copy the above configuration to src/stripe.ts');
    console.log('='.repeat(60));

    return results;
}

// Run
createProducts()
    .then(results => {
        console.log('\nDone! Created', results.length, 'products.');
        process.exit(0);
    })
    .catch(err => {
        console.error('Failed:', err);
        process.exit(1);
    });

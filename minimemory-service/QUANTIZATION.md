# Vector Quantization Guide

Reduce memory usage by 75-97% while maintaining search quality.

## Overview

Vector quantization compresses high-dimensional embeddings to reduce storage.

| Type | Memory Reduction | Accuracy | Best For |
|------|-----------------|----------|----------|
| none | 0% | 100% | Small datasets |
| int8 | 75% | 99% | Large datasets |
| binary | 97% | 95% | Massive datasets |

## Scalar Quantization (int8)

Converts float32 to int8:
- Original: 4 bytes per dimension
- Quantized: 1 byte per dimension
- Values scaled from [-1,1] to [-127,127]

## Binary Quantization

Converts each dimension to 1 bit:
- Quantized: 1/8 byte per dimension
- Positive = 1, Negative = 0
- Uses Hamming distance for fast search
- Supports rescoring for better accuracy

## Memory Savings

For 768-dimensional vectors:

| Vectors | float32 | int8 | binary |
|---------|---------|------|--------|
| 10K | 29.3 MB | 7.3 MB | 0.9 MB |
| 100K | 293 MB | 73 MB | 9 MB |
| 1M | 2.9 GB | 732 MB | 91 MB |

## Cost Impact (Cloudflare D1)

D1 pricing: 0.75/GB-month, 5GB free

| Scale | none | int8 | binary |
|-------|------|------|--------|
| 100K | 0.22 | 0.05 | 0.01 |
| 1M | 2.20 | 0.55 | 0.07 |
| 10M | 22.00 | 5.50 | 0.68 |

## Usage

VectorDB configuration:

    const db = new VectorDB({
      dimensions: 768,
      quantization: int8,
      rescoreOversample: 4
    });

## Recommendations

- none: < 10K vectors or 100% accuracy required
- int8: 10K-1M vectors (best balance)
- binary: > 1M vectors or speed critical

## API Response

GET /api/v1/stats includes:

    quantization: int8
    memoryEstimate:
      float32MB: 2.93
      quantizedMB: 0.73
      savingsPercent: 75

## Serialization

- Format version: 3.0.0
- Quantized vectors stored as base64
- Full precision maintained for import/export

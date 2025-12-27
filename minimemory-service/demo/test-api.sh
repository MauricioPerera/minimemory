#!/bin/bash
# minimemory-service API Demo
# Run the server first: npm run dev

API="http://localhost:3000/api/v1"
KEY="X-API-Key: mm_dev_key_12345"

echo "======================================"
echo "  minimemory-service API Demo"
echo "======================================"
echo ""

# 1. Check health
echo "1. Health Check"
curl -s http://localhost:3000/health | jq .
echo ""

# 2. Get stats (empty)
echo "2. Initial Stats (empty)"
curl -s -H "$KEY" "$API/stats" | jq .
echo ""

# 3. Remember - Store memories
echo "3. Storing Memories..."

# Episodic memory - user interaction
curl -s -X POST "$API/remember" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User asked about pricing for enterprise plan",
    "embedding": [0.9, 0.1, 0.2, 0.1, 0.0],
    "type": "episodic",
    "importance": 0.8,
    "metadata": {
      "event": "pricing_inquiry",
      "context": "sales_chat",
      "userId": "user-123"
    }
  }' | jq .

# Semantic memory - user fact
curl -s -X POST "$API/remember" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User works at Acme Corp, a Fortune 500 company",
    "embedding": [0.85, 0.15, 0.25, 0.1, 0.0],
    "type": "semantic",
    "importance": 0.9,
    "metadata": {
      "category": "user_info",
      "confidence": 0.95,
      "userId": "user-123"
    }
  }' | jq .

# More episodic memories
curl -s -X POST "$API/remember" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User requested demo of the product",
    "embedding": [0.8, 0.2, 0.3, 0.15, 0.0],
    "type": "episodic",
    "importance": 0.7,
    "metadata": {
      "event": "demo_request",
      "userId": "user-123"
    }
  }' | jq .

curl -s -X POST "$API/remember" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User prefers email communication over phone calls",
    "embedding": [0.1, 0.9, 0.1, 0.8, 0.7],
    "type": "semantic",
    "importance": 0.6,
    "metadata": {
      "category": "preferences",
      "userId": "user-123"
    }
  }' | jq .

curl -s -X POST "$API/remember" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User mentioned budget is around 50k annually",
    "embedding": [0.15, 0.85, 0.15, 0.75, 0.65],
    "type": "semantic",
    "importance": 0.85,
    "metadata": {
      "category": "budget",
      "userId": "user-123"
    }
  }' | jq .

echo ""
echo "4. Stats After Adding Memories"
curl -s -H "$KEY" "$API/stats" | jq .
echo ""

# 4. Recall - Vector search
echo "5. Recall - Vector Search (pricing related)"
curl -s -X POST "$API/recall" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "embedding": [0.88, 0.12, 0.22, 0.12, 0.0],
    "mode": "vector",
    "limit": 3
  }' | jq .
echo ""

# 5. Recall - Keyword search
echo "6. Recall - Keyword Search (user preferences)"
curl -s -X POST "$API/recall" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "keywords": "user prefers email",
    "mode": "keyword",
    "limit": 3
  }' | jq .
echo ""

# 6. Recall - Hybrid search
echo "7. Recall - Hybrid Search (combining vector + keywords)"
curl -s -X POST "$API/recall" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "embedding": [0.5, 0.5, 0.2, 0.4, 0.3],
    "keywords": "user budget pricing",
    "mode": "hybrid",
    "alpha": 0.6,
    "limit": 5
  }' | jq .
echo ""

# 7. Get specific memory
echo "8. Get Stats"
curl -s -H "$KEY" "$API/stats" | jq .
echo ""

# 8. Export
echo "9. Export All Memories"
curl -s -X POST "$API/export" -H "$KEY" | jq '.data.memories | length'
echo " memories exported"
echo ""

# 9. Filter by type
echo "10. Recall - Filter by Type (semantic only)"
curl -s -X POST "$API/recall" \
  -H "$KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "keywords": "user",
    "mode": "keyword",
    "type": "semantic",
    "limit": 10
  }' | jq '.results | map({content, type, importance})'
echo ""

# 10. Clean up
echo "11. Cleanup - Delete all"
curl -s -X DELETE "$API/clear" -H "$KEY" | jq .
echo ""

echo "======================================"
echo "  Demo Complete!"
echo "======================================"

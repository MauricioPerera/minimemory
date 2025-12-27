# Audit Logging Guide

Complete traceability for all memory operations in minimemory-service.

## Overview

The audit system automatically logs all memory operations, providing:

- **Full Traceability**: Every create, read, update, delete, and search operation
- **Actor Tracking**: Who performed the action (user, API key, tenant)
- **Change History**: Before/after states for modifications
- **Performance Metrics**: Duration of each operation
- **Error Logging**: Failed operations with error details
- **Compliance Ready**: 90-day retention with configurable cleanup

## Automatic Logging

All memory operations are logged automatically when D1 is configured:

| Operation | Action Logged | Details Captured |
|-----------|---------------|------------------|
| `/remember` | `create` | type, importance, contentLength, embeddingGenerated |
| `/recall` | `search` | mode, resultCount, limit, embeddingGenerated |
| `/memory/:id` GET | `read` | source (d1/memory) |
| `/memory/:id` PATCH | `update` | updatedFields |
| `/forget/:id` | `delete` | deleted (boolean) |
| `/forget` POST | `delete` | filter, count |
| `/export` | `export` | count, source |
| `/import` | `import` | count |
| `/clear` | `clear` | namespace |

## Audit Entry Structure

```typescript
interface AuditEntry {
  id: string;              // "aud_xxx" unique identifier
  timestamp: number;       // Unix timestamp in milliseconds
  action: AuditAction;     // create, read, update, delete, search, etc.
  resourceType: string;    // memory, namespace, user, tenant
  resourceId?: string;     // ID of affected resource

  // Actor information
  userId?: string;         // From X-User-Id header or JWT
  tenantId?: string;       // From X-Tenant-Id header
  namespace?: string;      // From X-Namespace header
  apiKeyPrefix?: string;   // First 8 chars of API key (masked)

  // Request context
  ipAddress?: string;      // From CF-Connecting-IP or X-Forwarded-For
  userAgent?: string;      // Browser/client info
  requestId?: string;      // For correlating related operations

  // Operation details
  details?: object;        // Action-specific information
  success: boolean;        // Operation result
  errorMessage?: string;   // Error details if failed
  durationMs?: number;     // Operation duration
}
```

## API Endpoints

### Query Audit Logs

```bash
GET /api/v1/audit
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| action | string | Filter by action type |
| resourceType | string | Filter by resource type |
| resourceId | string | Filter by specific resource |
| userId | string | Filter by user |
| tenantId | string | Filter by tenant |
| namespace | string | Filter by namespace |
| startTime | number | Start of time range (ms) |
| endTime | number | End of time range (ms) |
| success | boolean | Filter by success status |
| requestId | string | Filter by request ID |
| limit | number | Max results (default: 100) |
| offset | number | Pagination offset |

**Example:**

```bash
# Get all create operations in the last hour
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit?action=create&startTime=$(date -d '1 hour ago' +%s)000"

# Get operations for a specific memory
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit?resourceType=memory&resourceId=mem-123"
```

**Response:**

```json
{
  "success": true,
  "entries": [
    {
      "id": "aud_m1abc_xyz123",
      "timestamp": 1703443200000,
      "action": "create",
      "resourceType": "memory",
      "resourceId": "mem-123",
      "userId": "user-456",
      "tenantId": "tenant-789",
      "namespace": "default",
      "details": {
        "type": "semantic",
        "importance": 0.8,
        "embeddingGenerated": true,
        "contentLength": 150
      },
      "success": true,
      "durationMs": 45
    }
  ],
  "total": 100,
  "hasMore": true
}
```

### Get Entry by ID

```bash
GET /api/v1/audit/:id
```

**Example:**

```bash
curl -H "X-API-Key: mm_dev_key_12345" \
  http://localhost:8787/api/v1/audit/aud_m1abc_xyz123
```

### Get Resource History

View all operations on a specific resource:

```bash
GET /api/v1/audit/resource/:type/:id
```

**Example:**

```bash
# View history of memory mem-123
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit/resource/memory/mem-123?limit=20"
```

**Response:**

```json
{
  "success": true,
  "resourceType": "memory",
  "resourceId": "mem-123",
  "entries": [
    {
      "id": "aud_xxx",
      "timestamp": 1703443500000,
      "action": "update",
      "details": { "updatedFields": ["importance"] }
    },
    {
      "id": "aud_yyy",
      "timestamp": 1703443200000,
      "action": "create",
      "details": { "type": "semantic", "importance": 0.5 }
    }
  ],
  "count": 2
}
```

### Get User Activity

View all operations by a specific user:

```bash
GET /api/v1/audit/user/:id
```

**Example:**

```bash
# View activity for user-456 in the last 24 hours
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit/user/user-456?startTime=$(date -d '24 hours ago' +%s)000"
```

### Get Failed Operations

View operations that failed:

```bash
GET /api/v1/audit/failures
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| tenantId | string | Filter by tenant |
| namespace | string | Filter by namespace |
| limit | number | Max results (default: 50) |

**Example:**

```bash
curl -H "X-API-Key: mm_dev_key_12345" \
  http://localhost:8787/api/v1/audit/failures
```

**Response:**

```json
{
  "success": true,
  "entries": [
    {
      "id": "aud_xxx",
      "timestamp": 1703443200000,
      "action": "create",
      "resourceType": "memory",
      "success": false,
      "errorMessage": "content is required and must be a string",
      "durationMs": 5
    }
  ],
  "count": 1
}
```

### Get Audit Statistics

Aggregate statistics for audit data:

```bash
GET /api/v1/audit/stats
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| tenantId | string | Filter by tenant |
| startTime | number | Start of time range (ms) |
| endTime | number | End of time range (ms) |

**Example:**

```bash
# Stats for the last 7 days
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit/stats?startTime=$(date -d '7 days ago' +%s)000"
```

**Response:**

```json
{
  "success": true,
  "tenantId": "tenant-123",
  "stats": {
    "totalOperations": 15000,
    "byAction": {
      "create": 5000,
      "read": 3000,
      "search": 6000,
      "update": 500,
      "delete": 300,
      "export": 100,
      "import": 100
    },
    "byResource": {
      "memory": 14500,
      "namespace": 500
    },
    "successRate": 99.5,
    "avgDurationMs": 42
  }
}
```

### Clean Up Old Logs

Remove audit logs older than a specified retention period:

```bash
POST /api/v1/audit/cleanup
```

**Request Body:**

```json
{
  "retentionDays": 90
}
```

**Response:**

```json
{
  "success": true,
  "deletedCount": 50000,
  "message": "Deleted 50000 audit entries older than 90 days"
}
```

## Programmatic Usage

### Using the AuditService Directly

```typescript
import { AuditService } from './services/AuditService.js';

const auditService = new AuditService(env.DB);

// Log an operation
await auditService.log({
  action: 'create',
  resourceType: 'memory',
  resourceId: 'mem-123',
  userId: 'user-456',
  tenantId: 'tenant-789',
  namespace: 'default',
  details: { type: 'semantic', importance: 0.8 },
  success: true,
  durationMs: 45
});

// Query logs
const result = await auditService.query({
  action: 'create',
  startTime: Date.now() - 86400000, // Last 24 hours
  limit: 100
});

// Get resource history
const history = await auditService.getResourceHistory('memory', 'mem-123');

// Get user activity
const activity = await auditService.getUserActivity('user-456');

// Get statistics
const stats = await auditService.getStats('tenant-789', {
  startTime: Date.now() - 604800000 // Last 7 days
});
```

### Using the Audit Logger Helper

For request-scoped logging with automatic context:

```typescript
import { createAuditLogger } from './services/AuditService.js';

// Create logger with request context
const logger = createAuditLogger(env.DB, {
  userId: 'user-456',
  tenantId: 'tenant-789',
  namespace: 'default',
  apiKey: 'mm_xxx...',
  ipAddress: '192.168.1.1',
  userAgent: 'Mozilla/5.0...',
  requestId: 'req-abc123'
});

// Log memory operations
await logger.logMemory('create', 'mem-123', { type: 'semantic' });
await logger.logMemory('search', undefined, { resultCount: 10 });
await logger.logMemory('update', 'mem-123', { updatedFields: ['importance'] });
await logger.logMemory('delete', 'mem-123', { deleted: true });

// Log namespace operations
await logger.logNamespace('create', 'my-namespace', { dimensions: 768 });
await logger.logNamespace('clear', 'my-namespace', {});

// Log bulk operations
await logger.logBulk('export', { count: 100 });
await logger.logBulk('import', { count: 50 });

// Log auth operations
await logger.logAuth('login', 'user-456', { method: 'password' });
await logger.logAuth('logout', 'user-456', {});

// Log failed operations
await logger.logMemory('create', undefined, { error: 'Invalid content' }, {
  success: false,
  errorMessage: 'content is required'
});
```

## Database Schema

The audit log uses this D1 schema:

```sql
CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  timestamp INTEGER NOT NULL,

  -- Operation
  action TEXT NOT NULL,
  resource_type TEXT NOT NULL,
  resource_id TEXT,

  -- Actor
  user_id TEXT,
  tenant_id TEXT,
  namespace TEXT,
  api_key_prefix TEXT,

  -- Request
  ip_address TEXT,
  user_agent TEXT,
  request_id TEXT,

  -- Details
  details TEXT,        -- JSON
  success INTEGER NOT NULL DEFAULT 1,
  error_message TEXT,
  duration_ms INTEGER
);

-- Indexes for efficient queries
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_action ON audit_log(action, timestamp DESC);
CREATE INDEX idx_audit_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_user ON audit_log(user_id, timestamp DESC);
CREATE INDEX idx_audit_tenant ON audit_log(tenant_id, timestamp DESC);
```

## Best Practices

### 1. Use Request IDs for Correlation

Pass `X-Request-Id` header to correlate related operations:

```bash
REQUEST_ID=$(uuidgen)

# Create memory
curl -H "X-API-Key: mm_dev_key_12345" \
     -H "X-Request-Id: $REQUEST_ID" \
     -X POST http://localhost:8787/api/v1/remember \
     -d '{"content": "..."}'

# Later, find all operations for this request
curl -H "X-API-Key: mm_dev_key_12345" \
     "http://localhost:8787/api/v1/audit?requestId=$REQUEST_ID"
```

### 2. Schedule Regular Cleanup

Set up a cron job to clean old audit logs:

```bash
# Clean logs older than 90 days (run weekly)
0 0 * * 0 curl -X POST http://localhost:8787/api/v1/audit/cleanup \
  -H "X-API-Key: mm_dev_key_12345" \
  -d '{"retentionDays": 90}'
```

### 3. Monitor Failed Operations

Set up alerting on failed operations:

```bash
# Check for failures in the last hour
FAILURES=$(curl -s -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit/failures?limit=1" | jq '.count')

if [ "$FAILURES" -gt 0 ]; then
  echo "Alert: $FAILURES failed operations in the last hour"
fi
```

### 4. Compliance Reporting

Generate compliance reports:

```bash
# Get monthly stats for compliance
curl -H "X-API-Key: mm_dev_key_12345" \
  "http://localhost:8787/api/v1/audit/stats?startTime=$(date -d '1 month ago' +%s)000" \
  | jq '{
    period: "last_30_days",
    total_operations: .stats.totalOperations,
    success_rate: .stats.successRate,
    operations_by_type: .stats.byAction
  }'
```

## Security Considerations

1. **API Key Masking**: Only the first 8 characters of API keys are stored
2. **No Content Logging**: Memory content is never stored in audit logs
3. **Tenant Isolation**: Audit queries respect tenant boundaries
4. **Retention Policy**: Configure cleanup to meet data retention requirements

## Cost Estimation

Audit log storage in D1:

| Scale | Monthly Entries | Storage | Cost |
|-------|-----------------|---------|------|
| 10K ops/day | 300K | ~30MB | ~$0.01 |
| 100K ops/day | 3M | ~300MB | ~$0.10 |
| 1M ops/day | 30M | ~3GB | ~$1.00 |

*With 90-day retention*

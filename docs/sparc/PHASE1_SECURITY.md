# SPARC Specification: Phase 1 - Security Fixes

## S - Specification

### Overview
Implement production-grade security controls for the LLM-Simulator to prevent unauthorized access, protect against common web vulnerabilities, and enable safe deployment in enterprise environments.

### Objectives
1. Implement API key authentication with configurable validation
2. Add role-based authorization for admin endpoints
3. Restrict CORS to explicitly allowed origins
4. Implement token bucket rate limiting
5. Add security headers to all responses

### Requirements

#### 1.1 API Key Authentication
- **MUST** validate API keys in `Authorization: Bearer <key>` header format
- **MUST** support configurable API key list via environment variable or config file
- **MUST** return 401 Unauthorized for missing/invalid keys
- **SHOULD** support key rotation without restart
- **MAY** support multiple key tiers (admin, user, readonly)

#### 1.2 Admin Endpoint Authorization
- **MUST** require special admin API key for `/admin/*` endpoints
- **MUST** return 403 Forbidden for non-admin keys accessing admin routes
- **SHOULD** log all admin endpoint access attempts
- **SHOULD** support IP allowlist for admin access

#### 1.3 CORS Restriction
- **MUST** replace wildcard `*` with explicit origin allowlist
- **MUST** make allowed origins configurable via config/env
- **MUST** validate `Origin` header against allowlist
- **SHOULD** support regex patterns for origin matching
- **MUST** set `Access-Control-Allow-Credentials: false` by default

#### 1.4 Rate Limiting
- **MUST** implement token bucket algorithm per API key
- **MUST** return 429 Too Many Requests with `Retry-After` header
- **MUST** make limits configurable (requests/minute, burst size)
- **SHOULD** support different limits per endpoint tier
- **SHOULD** expose rate limit headers (`X-RateLimit-*`)

#### 1.5 Security Headers
- **MUST** add `X-Content-Type-Options: nosniff`
- **MUST** add `X-Frame-Options: DENY`
- **MUST** add `X-XSS-Protection: 1; mode=block`
- **SHOULD** add `Content-Security-Policy` header
- **SHOULD** add `Strict-Transport-Security` when behind TLS

### Success Criteria
- All endpoints require valid API key (except health checks)
- Admin endpoints accessible only with admin keys
- CORS blocks requests from non-allowed origins
- Rate limiting enforces configured limits
- All security headers present in responses
- Zero authentication bypasses in security audit

---

## P - Pseudocode

### 1.1 API Key Validation Middleware

```
FUNCTION api_key_middleware(request, config):
    // Skip auth for health endpoints
    IF request.path IN ["/health", "/healthz", "/ready", "/readyz", "/metrics"]:
        RETURN next(request)

    // Extract API key from header
    auth_header = request.headers.get("Authorization")
    IF auth_header IS NULL:
        RETURN error_response(401, "Missing Authorization header")

    IF NOT auth_header.starts_with("Bearer "):
        RETURN error_response(401, "Invalid Authorization format")

    api_key = auth_header.strip_prefix("Bearer ")

    // Validate key against configured keys
    key_info = config.api_keys.validate(api_key)
    IF key_info IS NULL:
        log.warn("Invalid API key attempt", key_prefix=api_key[0:8])
        RETURN error_response(401, "Invalid API key")

    // Attach key info to request context
    request.extensions.insert(ApiKeyInfo(key_info))

    RETURN next(request)
```

### 1.2 Admin Authorization Middleware

```
FUNCTION admin_auth_middleware(request):
    IF NOT request.path.starts_with("/admin"):
        RETURN next(request)

    key_info = request.extensions.get(ApiKeyInfo)
    IF key_info IS NULL:
        RETURN error_response(401, "Authentication required")

    IF key_info.role != "admin":
        log.warn("Non-admin access attempt to admin endpoint",
                 key_id=key_info.id, path=request.path)
        RETURN error_response(403, "Admin access required")

    // Optional: Check IP allowlist
    IF config.admin_ip_allowlist IS NOT EMPTY:
        client_ip = request.client_ip()
        IF client_ip NOT IN config.admin_ip_allowlist:
            log.warn("Admin access from non-allowed IP", ip=client_ip)
            RETURN error_response(403, "IP not allowed for admin access")

    RETURN next(request)
```

### 1.3 CORS Configuration

```
STRUCT CorsConfig:
    enabled: bool
    allowed_origins: Vec<String>
    allowed_methods: Vec<Method>
    allowed_headers: Vec<String>
    allow_credentials: bool
    max_age: Duration

FUNCTION build_cors_layer(config: CorsConfig) -> CorsLayer:
    IF NOT config.enabled:
        RETURN CorsLayer::very_permissive()  // For development only

    layer = CorsLayer::new()

    // Build origin validator
    IF config.allowed_origins.contains("*"):
        log.warn("Wildcard CORS origin detected - not recommended for production")
        layer = layer.allow_origin(Any)
    ELSE:
        origins = config.allowed_origins
            .map(|o| o.parse::<HeaderValue>())
            .collect()
        layer = layer.allow_origin(origins)

    layer = layer
        .allow_methods(config.allowed_methods)
        .allow_headers(config.allowed_headers)
        .allow_credentials(config.allow_credentials)
        .max_age(config.max_age)

    RETURN layer
```

### 1.4 Token Bucket Rate Limiter

```
STRUCT TokenBucket:
    capacity: u32           // Max tokens
    tokens: AtomicU32       // Current tokens
    refill_rate: u32        // Tokens per second
    last_refill: AtomicU64  // Last refill timestamp

STRUCT RateLimiter:
    buckets: DashMap<String, TokenBucket>  // Key -> Bucket
    config: RateLimitConfig

FUNCTION rate_limit_middleware(request, limiter):
    key_info = request.extensions.get(ApiKeyInfo)
    IF key_info IS NULL:
        RETURN next(request)  // Will be caught by auth middleware

    bucket_key = key_info.id
    limit_config = limiter.config.get_limit_for_tier(key_info.tier)

    bucket = limiter.buckets.entry(bucket_key)
        .or_insert(TokenBucket::new(limit_config))

    // Refill tokens based on elapsed time
    bucket.refill()

    // Try to consume a token
    IF NOT bucket.try_consume(1):
        retry_after = bucket.time_until_token()
        RETURN error_response(429, "Rate limit exceeded")
            .header("Retry-After", retry_after.as_secs())
            .header("X-RateLimit-Limit", limit_config.requests_per_minute)
            .header("X-RateLimit-Remaining", bucket.tokens())
            .header("X-RateLimit-Reset", bucket.next_reset_time())

    // Add rate limit headers to response
    response = next(request)
    response.headers.insert("X-RateLimit-Limit", limit_config.requests_per_minute)
    response.headers.insert("X-RateLimit-Remaining", bucket.tokens())

    RETURN response
```

### 1.5 Security Headers Middleware

```
FUNCTION security_headers_middleware(request, next):
    response = next(request)

    headers = response.headers_mut()

    // Prevent MIME sniffing
    headers.insert("X-Content-Type-Options", "nosniff")

    // Prevent clickjacking
    headers.insert("X-Frame-Options", "DENY")

    // XSS protection (legacy browsers)
    headers.insert("X-XSS-Protection", "1; mode=block")

    // Referrer policy
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin")

    // Content Security Policy (API-focused)
    headers.insert("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'")

    // Permissions policy
    headers.insert("Permissions-Policy", "geolocation=(), microphone=(), camera=()")

    RETURN response
```

---

## A - Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Request Flow                                   │
└─────────────────────────────────────────────────────────────────────┘

    Client Request
          │
          ▼
┌─────────────────────┐
│  Security Headers   │  ← Adds headers to all responses
│     Middleware      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   CORS Middleware   │  ← Validates Origin header
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   API Key Auth      │  ← Validates Bearer token
│     Middleware      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Rate Limiting      │  ← Token bucket per API key
│     Middleware      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Admin Auth         │  ← Checks admin role for /admin/*
│     Middleware      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   Route Handlers    │
└─────────────────────┘
```

### File Structure

```
src/
├── security/
│   ├── mod.rs              # Module exports
│   ├── api_key.rs          # API key validation & storage
│   ├── rate_limit.rs       # Token bucket implementation
│   ├── cors.rs             # CORS configuration builder
│   └── headers.rs          # Security headers middleware
├── server/
│   ├── middleware.rs       # Updated middleware stack
│   └── mod.rs              # Updated server builder
└── config/
    └── security.rs         # Security configuration structs
```

### Configuration Schema

```yaml
# config/default.yaml additions
security:
  # API Key Authentication
  api_keys:
    enabled: true
    keys:
      - id: "key-001"
        key: "${API_KEY_001}"  # From environment
        role: "admin"
        rate_limit_tier: "premium"
      - id: "key-002"
        key: "${API_KEY_002}"
        role: "user"
        rate_limit_tier: "standard"

  # Admin Authorization
  admin:
    require_admin_key: true
    ip_allowlist: []  # Empty = allow all IPs with admin key

  # CORS Settings
  cors:
    enabled: true
    allowed_origins:
      - "https://app.example.com"
      - "https://staging.example.com"
    allowed_methods: ["GET", "POST", "OPTIONS"]
    allowed_headers: ["Content-Type", "Authorization", "X-Request-ID"]
    allow_credentials: false
    max_age_seconds: 3600

  # Rate Limiting
  rate_limiting:
    enabled: true
    default_tier: "standard"
    tiers:
      standard:
        requests_per_minute: 60
        burst_size: 10
      premium:
        requests_per_minute: 600
        burst_size: 100
      admin:
        requests_per_minute: 1000
        burst_size: 200

  # Security Headers
  headers:
    enabled: true
    hsts_enabled: false  # Enable when behind TLS
    hsts_max_age: 31536000
```

### Data Flow

```
1. Request arrives
2. Security headers middleware wraps response builder
3. CORS middleware checks Origin header
   - If not in allowlist → 403 Forbidden
4. API key middleware extracts Bearer token
   - If missing/invalid → 401 Unauthorized
   - If valid → attach ApiKeyInfo to request
5. Rate limit middleware checks bucket
   - If empty → 429 Too Many Requests
   - If available → consume token, continue
6. Admin auth middleware checks role (for /admin/*)
   - If not admin → 403 Forbidden
7. Request reaches handler
8. Response flows back through middleware
   - Rate limit headers added
   - Security headers added
```

---

## R - Refinement

### Edge Cases

1. **Key Rotation During Request**
   - Use versioned keys with grace period
   - Accept old key for 5 minutes after rotation

2. **Rate Limit Bucket Overflow**
   - Cap refill at bucket capacity
   - Use atomic operations for thread safety

3. **CORS Preflight Caching**
   - Set appropriate `Access-Control-Max-Age`
   - Cache preflight responses client-side

4. **Clock Skew in Rate Limiting**
   - Use monotonic clock for timing
   - Don't rely on wall clock for token refill

5. **Memory Growth from Rate Limit Buckets**
   - Implement bucket expiration (TTL)
   - Clean up inactive buckets periodically

### Error Handling

```rust
// Security-specific error types
pub enum SecurityError {
    MissingApiKey,
    InvalidApiKey { key_prefix: String },
    ExpiredApiKey { key_id: String },
    InsufficientPermissions { required: String, actual: String },
    RateLimitExceeded { retry_after: Duration },
    CorsOriginNotAllowed { origin: String },
    IpNotAllowed { ip: IpAddr },
}

impl IntoResponse for SecurityError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            Self::MissingApiKey => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "Missing API key in Authorization header"
            ),
            Self::InvalidApiKey { .. } => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "Invalid API key"
            ),
            Self::InsufficientPermissions { required, .. } => (
                StatusCode::FORBIDDEN,
                "permission_error",
                format!("Requires {} permission", required)
            ),
            Self::RateLimitExceeded { retry_after } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_error",
                "Rate limit exceeded"
            ),
            // ... other variants
        };

        // Return OpenAI-compatible error response
        (status, Json(ErrorResponse::new(error_type, &message))).into_response()
    }
}
```

### Performance Considerations

1. **API Key Lookup**: O(1) with HashMap, consider bloom filter for negative lookups
2. **Rate Limit Buckets**: Use `DashMap` for concurrent access without global lock
3. **CORS Origin Matching**: Pre-compile regex patterns, cache results
4. **Header Injection**: Use static `HeaderName` constants to avoid parsing

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    // Unit tests for each component
    #[test]
    fn test_api_key_validation() { ... }

    #[test]
    fn test_token_bucket_refill() { ... }

    #[test]
    fn test_cors_origin_matching() { ... }

    // Integration tests
    #[tokio::test]
    async fn test_unauthenticated_request_rejected() { ... }

    #[tokio::test]
    async fn test_admin_endpoint_requires_admin_key() { ... }

    #[tokio::test]
    async fn test_rate_limit_enforced() { ... }

    #[tokio::test]
    async fn test_cors_preflight_handling() { ... }
}
```

---

## C - Completion

### Definition of Done

- [ ] API key validation middleware implemented and tested
- [ ] Admin authorization middleware implemented and tested
- [ ] CORS restriction with configurable origins
- [ ] Token bucket rate limiter with per-key tracking
- [ ] Security headers added to all responses
- [ ] Configuration schema documented and validated
- [ ] All existing tests still pass
- [ ] New security tests achieve >90% coverage of security module
- [ ] Security audit passes with no critical findings
- [ ] Documentation updated with security configuration guide

### Verification Checklist

```bash
# 1. Test missing API key
curl -i http://localhost:8080/v1/chat/completions
# Expected: 401 Unauthorized

# 2. Test invalid API key
curl -i -H "Authorization: Bearer invalid-key" http://localhost:8080/v1/chat/completions
# Expected: 401 Unauthorized

# 3. Test valid API key
curl -i -H "Authorization: Bearer $VALID_KEY" http://localhost:8080/v1/chat/completions -d '{...}'
# Expected: 200 OK with response

# 4. Test admin endpoint with user key
curl -i -H "Authorization: Bearer $USER_KEY" http://localhost:8080/admin/stats
# Expected: 403 Forbidden

# 5. Test admin endpoint with admin key
curl -i -H "Authorization: Bearer $ADMIN_KEY" http://localhost:8080/admin/stats
# Expected: 200 OK

# 6. Test CORS from disallowed origin
curl -i -H "Origin: https://evil.com" http://localhost:8080/v1/models
# Expected: No CORS headers / blocked

# 7. Test rate limiting
for i in {1..100}; do curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $KEY" http://localhost:8080/v1/models; done
# Expected: 429 responses after limit exceeded

# 8. Verify security headers
curl -i http://localhost:8080/health
# Expected: X-Content-Type-Options, X-Frame-Options, etc.
```

### Rollback Plan

1. Keep feature flag for each security component
2. Configuration allows disabling individual features
3. Maintain backward compatibility mode for development
4. Document emergency disable procedure

### Monitoring

- Metric: `llm_simulator_auth_failures_total{reason="..."}`
- Metric: `llm_simulator_rate_limit_hits_total{tier="..."}`
- Metric: `llm_simulator_cors_blocked_total{origin="..."}`
- Alert: Auth failure rate > 10/min from single IP
- Alert: Rate limit exhaustion for any key


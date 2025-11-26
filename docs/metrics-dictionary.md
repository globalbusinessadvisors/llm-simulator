# Metrics Dictionary

**Version:** 1.0
**Last Updated:** 2025-11-26

This document provides a comprehensive reference for all metrics exposed by LLM-Simulator.

---

## Metric Categories

- [Request Metrics](#request-metrics)
- [Token Metrics](#token-metrics)
- [Latency Metrics](#latency-metrics)
- [Error Metrics](#error-metrics)
- [Cost Metrics](#cost-metrics)
- [System Metrics](#system-metrics)
- [Cache Metrics](#cache-metrics)

---

## Request Metrics

### `llm.requests.total`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Total number of LLM requests processed

**Labels:**
- `provider` (string): LLM provider (openai, anthropic, cohere, google)
- `model` (string): Model name (gpt-4, claude-3, etc.)
- `operation` (string): Operation type (completion, chat, embedding, streaming)
- `status` (string): Request status (success, error)

**Example Query:**
```promql
# Total requests per second
sum(rate(llm_requests_total[5m]))

# Success rate
sum(rate(llm_requests_total{status="success"}[5m])) / sum(rate(llm_requests_total[5m]))

# Requests by provider
sum(rate(llm_requests_total[5m])) by (provider)
```

**Alert Thresholds:**
- Critical: < 0.95 success rate
- Warning: < 0.98 success rate

---

### `llm.requests.duration`

**Type:** Histogram
**Unit:** milliseconds
**Description:** Distribution of request processing time

**Buckets:** [10, 50, 100, 250, 500, 1000, 2500, 5000, 10000] ms

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name
- `operation` (string): Operation type

**Example Query:**
```promql
# P95 latency
histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le))

# P99 latency by provider
histogram_quantile(0.99, sum(rate(llm_requests_duration_bucket[5m])) by (le, provider))

# Average latency
sum(rate(llm_requests_duration_sum[5m])) / sum(rate(llm_requests_duration_count[5m]))
```

**SLO Targets:**
- P95 < 5000ms
- P99 < 10000ms

---

### `llm.requests.errors.total`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Total number of failed requests

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name
- `error_type` (string): Error category (rate_limit, timeout, auth, invalid_request, unknown)
- `error_code` (string): HTTP error code (429, 500, etc.)

**Example Query:**
```promql
# Error rate
sum(rate(llm_requests_errors_total[5m])) / sum(rate(llm_requests_total[5m]))

# Errors by type
sum(rate(llm_requests_errors_total[5m])) by (error_type)

# Top error sources
topk(5, sum(rate(llm_requests_errors_total[5m])) by (provider, error_type))
```

**Alert Thresholds:**
- Critical: > 5% error rate
- Warning: > 2% error rate

---

## Token Metrics

### `llm.tokens.total`

**Type:** Counter
**Unit:** tokens
**Description:** Total number of tokens processed

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name
- `token_type` (string): Token category (prompt, completion, total)

**Example Query:**
```promql
# Total tokens per second
sum(rate(llm_tokens_total[5m]))

# Tokens by type
sum(rate(llm_tokens_total[5m])) by (token_type)

# Tokens per provider
sum(rate(llm_tokens_total[5m])) by (provider)

# Prompt-to-completion ratio
sum(rate(llm_tokens_total{token_type="completion"}[5m])) / sum(rate(llm_tokens_total{token_type="prompt"}[5m]))
```

**Business Metrics:**
- Track for capacity planning
- Monitor for cost optimization
- Analyze for prompt efficiency

---

### `llm.tokens.per_request`

**Type:** Histogram
**Unit:** tokens
**Description:** Distribution of tokens per request

**Buckets:** [100, 500, 1000, 2000, 4000, 8000, 16000, 32000] tokens

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# P95 tokens per request
histogram_quantile(0.95, sum(rate(llm_tokens_per_request_bucket[5m])) by (le))

# Average tokens per request
sum(rate(llm_tokens_per_request_sum[5m])) / sum(rate(llm_tokens_per_request_count[5m]))

# Median tokens
histogram_quantile(0.5, sum(rate(llm_tokens_per_request_bucket[5m])) by (le))
```

**Analysis:**
- Monitor for prompt optimization opportunities
- Track model context window usage
- Identify outliers

---

## Latency Metrics

### `llm.latency.ttft`

**Type:** Histogram
**Unit:** milliseconds
**Description:** Time to First Token in streaming responses

**Buckets:** [50, 100, 250, 500, 1000, 2000, 5000] ms

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# P95 TTFT
histogram_quantile(0.95, sum(rate(llm_latency_ttft_bucket[5m])) by (le))

# TTFT by provider
histogram_quantile(0.95, sum(rate(llm_latency_ttft_bucket[5m])) by (le, provider))

# Average TTFT
sum(rate(llm_latency_ttft_sum[5m])) / sum(rate(llm_latency_ttft_count[5m]))
```

**SLO Target:** P95 < 2000ms

**Impact:** User experience for streaming responses

---

### `llm.latency.inter_token`

**Type:** Histogram
**Unit:** milliseconds
**Description:** Time between consecutive tokens in streaming

**Buckets:** [5, 10, 25, 50, 100, 250, 500] ms

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# P95 inter-token latency
histogram_quantile(0.95, sum(rate(llm_latency_inter_token_bucket[5m])) by (le))

# Tokens per second (derived)
1000 / histogram_quantile(0.5, sum(rate(llm_latency_inter_token_bucket[5m])) by (le))
```

**Analysis:**
- Smoothness of streaming experience
- Provider throughput comparison
- Network stability indicator

---

### `llm.latency.e2e`

**Type:** Histogram
**Unit:** milliseconds
**Description:** End-to-end request processing time

**Buckets:** [100, 500, 1000, 2500, 5000, 10000, 30000] ms

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name
- `operation` (string): Operation type

**Example Query:**
```promql
# P99 E2E latency
histogram_quantile(0.99, sum(rate(llm_latency_e2e_bucket[5m])) by (le))

# E2E by operation
histogram_quantile(0.95, sum(rate(llm_latency_e2e_bucket[5m])) by (le, operation))
```

**SLO Target:** P99 < 10000ms

---

### `llm.latency.provider_api`

**Type:** Histogram
**Unit:** milliseconds
**Description:** External provider API response time

**Buckets:** [100, 250, 500, 1000, 2000, 5000, 10000] ms

**Labels:**
- `provider` (string): LLM provider
- `endpoint` (string): API endpoint

**Example Query:**
```promql
# Provider API latency comparison
histogram_quantile(0.95, sum(rate(llm_latency_provider_api_bucket[5m])) by (le, provider))

# Overhead calculation (E2E - Provider API)
histogram_quantile(0.95, sum(rate(llm_latency_e2e_bucket[5m])) by (le)) - histogram_quantile(0.95, sum(rate(llm_latency_provider_api_bucket[5m])) by (le))
```

**Analysis:**
- Provider performance comparison
- Application overhead measurement
- Network latency tracking

---

## Error Metrics

### `llm.errors.by_type`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Errors categorized by type

**Labels:**
- `provider` (string): LLM provider
- `error_type` (string): Error category
- `error_code` (string): Error code
- `model` (string): Model name

**Example Query:**
```promql
# Error distribution
sum(rate(llm_errors_by_type[5m])) by (error_type)

# Top error combinations
topk(10, sum(rate(llm_errors_by_type[5m])) by (provider, error_type))
```

---

### `llm.errors.rate_limit`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Rate limit errors from providers

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# Rate limit errors per second
sum(rate(llm_errors_rate_limit[5m])) by (provider)

# Rate limit error percentage
sum(rate(llm_errors_rate_limit[5m])) / sum(rate(llm_requests_total[5m]))
```

**Alert Threshold:** > 0.1 errors/sec

**Action:** Implement backoff, increase rate limits, or scale

---

### `llm.errors.timeout`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Request timeout errors

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# Timeout rate
sum(rate(llm_errors_timeout[5m])) / sum(rate(llm_requests_total[5m]))

# Timeouts by provider
sum(rate(llm_errors_timeout[5m])) by (provider)
```

**Alert Threshold:** > 1% timeout rate

---

## Cost Metrics

### `llm.cost.total`

**Type:** Counter
**Unit:** USD
**Description:** Cumulative estimated cost

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name
- `cost_component` (string): Cost type (prompt, completion, total)

**Example Query:**
```promql
# Hourly cost rate
sum(rate(llm_cost_total[1h])) * 3600

# Daily cost
sum(increase(llm_cost_total[24h]))

# Cost by provider
sum(rate(llm_cost_total[1h])) by (provider) * 3600
```

**Budget Monitoring:**
- Set daily/monthly budgets
- Alert on unexpected spikes
- Track cost trends

---

### `llm.cost.per_request`

**Type:** Histogram
**Unit:** USD
**Description:** Cost distribution per request

**Buckets:** [0.001, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0] USD

**Labels:**
- `provider` (string): LLM provider
- `model` (string): Model name

**Example Query:**
```promql
# P95 cost per request
histogram_quantile(0.95, sum(rate(llm_cost_per_request_bucket[5m])) by (le))

# Average cost per request
sum(rate(llm_cost_per_request_sum[5m])) / sum(rate(llm_cost_per_request_count[5m]))

# Cost efficiency by model
sum(rate(llm_cost_per_request_sum[5m])) by (model) / sum(rate(llm_cost_per_request_count[5m])) by (model)
```

**Optimization:**
- Identify expensive requests
- Compare model cost-effectiveness
- Optimize prompt length

---

### `llm.cost.hourly_rate`

**Type:** Gauge
**Unit:** USD/hour
**Description:** Current cost rate per hour

**Labels:**
- `provider` (string): LLM provider

**Example Query:**
```promql
# Current hourly rate
sum(llm_cost_hourly_rate)

# Hourly rate by provider
sum(llm_cost_hourly_rate) by (provider)

# Projected monthly cost
sum(llm_cost_hourly_rate) * 24 * 30
```

**Alert Threshold:** > $100/hour (configurable)

---

## System Metrics

### `llm.connections.active`

**Type:** Gauge
**Unit:** 1 (count)
**Description:** Number of active provider connections

**Labels:**
- `provider` (string): LLM provider

**Example Query:**
```promql
# Active connections
sum(llm_connections_active) by (provider)

# Connection utilization
sum(llm_connections_active) / sum(llm_connections_max)
```

**Capacity Planning:**
- Monitor for connection pool saturation
- Track connection efficiency

---

### `llm.queue.depth`

**Type:** Gauge
**Unit:** 1 (count)
**Description:** Number of requests in processing queue

**Labels:**
- `priority` (string): Queue priority level (high, normal, low)

**Example Query:**
```promql
# Total queue depth
sum(llm_queue_depth)

# Queue depth by priority
sum(llm_queue_depth) by (priority)

# Queue wait time (estimated)
llm_queue_depth / sum(rate(llm_requests_total[5m]))
```

**Alert Threshold:** > 100 requests

**Action:** Scale workers or implement request shedding

---

### `llm.system.memory_usage`

**Type:** Histogram
**Unit:** bytes
**Description:** Application memory usage distribution

**Labels:**
- `component` (string): Application component

**Example Query:**
```promql
# Memory usage by component
sum(llm_system_memory_usage) by (component)

# Memory growth rate
rate(llm_system_memory_usage[5m])
```

**Alert Threshold:** > 2GB

---

## Cache Metrics

### `llm.cache.hits`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Number of cache hits

**Labels:**
- `cache_type` (string): Cache category (prompt, response, embedding)

**Example Query:**
```promql
# Cache hit rate
sum(rate(llm_cache_hits[5m])) / (sum(rate(llm_cache_hits[5m])) + sum(rate(llm_cache_misses[5m])))

# Hits by cache type
sum(rate(llm_cache_hits[5m])) by (cache_type)
```

**Target:** > 60% hit rate

**Cost Impact:**
- Higher hit rate = lower costs
- Reduced provider API calls

---

### `llm.cache.misses`

**Type:** Counter
**Unit:** 1 (count)
**Description:** Number of cache misses

**Labels:**
- `cache_type` (string): Cache category

**Example Query:**
```promql
# Miss rate
sum(rate(llm_cache_misses[5m])) / (sum(rate(llm_cache_hits[5m])) + sum(rate(llm_cache_misses[5m])))

# Misses by type
sum(rate(llm_cache_misses[5m])) by (cache_type)
```

**Analysis:**
- High miss rate may indicate:
  - Insufficient cache size
  - Low request similarity
  - Need for cache optimization

---

## Recording Rules

Pre-computed aggregations for performance:

```yaml
groups:
  - name: llm_recording_rules
    interval: 15s
    rules:
      # Request rate
      - record: llm:requests:rate5m
        expr: sum(rate(llm_requests_total[5m]))

      # Success rate
      - record: llm:requests:success_rate
        expr: sum(rate(llm_requests_total{status="success"}[5m])) / sum(rate(llm_requests_total[5m]))

      # P95 latency
      - record: llm:latency:p95
        expr: histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le))

      # P99 latency
      - record: llm:latency:p99
        expr: histogram_quantile(0.99, sum(rate(llm_requests_duration_bucket[5m])) by (le))

      # Hourly cost
      - record: llm:cost:hourly
        expr: sum(rate(llm_cost_total[1h])) * 3600

      # Cache hit rate
      - record: llm:cache:hit_rate
        expr: sum(rate(llm_cache_hits[5m])) / (sum(rate(llm_cache_hits[5m])) + sum(rate(llm_cache_misses[5m])))

      # Token rate
      - record: llm:tokens:rate5m
        expr: sum(rate(llm_tokens_total[5m]))
```

---

## Metric Naming Convention

All metrics follow this pattern:

```
{namespace}.{subsystem}.{metric}[.{unit}]
```

- **namespace**: `llm` (constant)
- **subsystem**: Component area (requests, tokens, latency, errors, cost, etc.)
- **metric**: Specific measurement (total, duration, rate, etc.)
- **unit**: Optional unit suffix (total, bytes, seconds)

**Examples:**
- `llm.requests.total`
- `llm.latency.ttft`
- `llm.cost.per_request`

---

## Label Cardinality Guidelines

**Low Cardinality** (< 10 values): provider, operation, status, priority
**Medium Cardinality** (10-100 values): model, error_type, cache_type
**High Cardinality** (> 100 values): AVOID in metrics (use trace attributes instead)

**Never use as labels:**
- User IDs
- Request IDs
- Trace IDs
- Timestamps
- Free-form text

---

## Retention Policy

| Metric Resolution | Retention Period | Use Case |
|------------------|------------------|----------|
| Raw (15s) | 7 days | Real-time monitoring |
| 5m aggregates | 30 days | Short-term analysis |
| 1h aggregates | 90 days | Medium-term trends |
| Daily aggregates | 1 year | Long-term planning |

---

**Version History:**

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-26 | Initial release |

**Maintained by:** Platform Engineering
**Next Review:** 2026-02-26

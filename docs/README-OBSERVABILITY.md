# LLM-Simulator Observability Architecture

> **Enterprise-grade observability and monitoring for LLM applications**

[![Production Ready](https://img.shields.io/badge/status-production--ready-green.svg)]()
[![OpenTelemetry](https://img.shields.io/badge/OpenTelemetry-enabled-blue.svg)]()
[![SLO Compliant](https://img.shields.io/badge/SLO-99.9%25-success.svg)]()

---

## Overview

This observability architecture provides comprehensive monitoring, tracing, and alerting for LLM-Simulator with a focus on:

- **Full-stack visibility** into LLM request flows
- **Production-grade reliability** with 99.9% availability SLO
- **Cost optimization** through detailed usage tracking
- **Performance insights** for latency and throughput
- **Operational excellence** with automated alerting and runbooks

---

## Quick Navigation

### Core Documentation

| Document | Description | Audience |
|----------|-------------|----------|
| [Observability Architecture](./observability-architecture.md) | Complete architecture, metrics, tracing, logging | All |
| [Implementation Guide](./observability-implementation-guide.md) | Step-by-step setup and instrumentation | Engineers |
| [Metrics Dictionary](./metrics-dictionary.md) | Complete metrics reference | Engineers, SREs |
| [SLO Definitions](./slo-definitions.md) | Service Level Objectives and error budgets | Product, SRE |

### Configuration Files

| File | Purpose |
|------|---------|
| [prometheus.yml](../config/prometheus.yml) | Prometheus scraping and alerting config |
| [otel-collector.yaml](../config/otel-collector.yaml) | OpenTelemetry Collector pipeline |
| [recording-rules.yaml](../config/recording-rules.yaml) | Pre-computed metrics aggregations |
| [docker-compose.observability.yml](../docker-compose.observability.yml) | Local development stack |

---

## Architecture At a Glance

```
┌─────────────────────────────────────────────────────────┐
│                  LLM-Simulator App                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │   API    │→ │Simulator │→ │ Provider │              │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘              │
│       └─────────────┴─────────────┘                     │
│                     │                                    │
│         ┌───────────┴───────────┐                       │
│         │ OpenTelemetry SDK     │                       │
│         │ • Traces  • Metrics   │                       │
│         │ • Logs    • Context   │                       │
│         └───────────┬───────────┘                       │
└─────────────────────┼─────────────────────────────────┘
                      │
            ┌─────────┴─────────┐
            │ OTLP Collector    │
            │ • Process         │
            │ • Sample          │
            │ • Export          │
            └─────────┬─────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
         ▼            ▼            ▼
    ┌────────┐  ┌─────────┐  ┌────────┐
    │ Jaeger │  │Prometheus│ │  Loki  │
    │ Traces │  │ Metrics  │ │  Logs  │
    └───┬────┘  └────┬────┘  └───┬────┘
        └────────────┼───────────┘
                     │
              ┌──────┴──────┐
              │   Grafana   │
              │  Dashboards │
              └─────────────┘
```

---

## Key Features

### 1. Distributed Tracing

- **Full request lifecycle** tracking from API to LLM provider
- **Context propagation** across service boundaries
- **Trace sampling** with intelligent tail-based decisions
- **Correlation** between traces, metrics, and logs

**Example Trace:**
```
Root Span: POST /v1/completions (2.3s)
├─ Validation (12ms)
├─ Rate Limit Check (8ms)
├─ LLM Completion (2.1s)
│  ├─ Provider Selection (3ms)
│  ├─ Prompt Processing (15ms)
│  ├─ OpenAI API Call (2.0s)
│  └─ Response Processing (25ms)
└─ Cache Update (45ms)
```

### 2. LLM-Specific Metrics

Comprehensive metrics for LLM operations:

- **Request Metrics**: Rate, duration, errors
- **Token Metrics**: Usage, distribution, efficiency
- **Latency Metrics**: TTFT, ITL, E2E
- **Cost Metrics**: Per-request, hourly, cumulative
- **Quality Metrics**: Completion rate, cache hits

**Total Metrics**: 19 core metrics, ~1,500 time series

### 3. Structured Logging

JSON-formatted logs with:

- **Trace correlation** via trace_id/span_id
- **Standardized schema** for consistent parsing
- **Multiple levels**: debug, info, warn, error, fatal
- **LLM context**: provider, model, tokens, cost

**Example Log Entry:**
```json
{
  "timestamp": "2025-11-26T10:30:45.123Z",
  "level": "info",
  "message": "LLM response received",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "service": {
    "name": "llm-simulator",
    "version": "1.0.0",
    "environment": "production"
  },
  "llm": {
    "provider": "openai",
    "model": "gpt-4",
    "tokens": {
      "prompt": 150,
      "completion": 850,
      "total": 1000
    },
    "duration_ms": 2340,
    "cost_usd": 0.045
  }
}
```

### 4. Intelligent Alerting

Multi-tier alerting with SLO-based thresholds:

| Severity | Examples | Response |
|----------|----------|----------|
| **P0 Critical** | Service down, >5% errors | Page immediately |
| **P1 High** | SLO violation, rate limits | Page on-call |
| **P2 Medium** | Elevated latency | Team ticket |
| **P3 Low** | Minor degradation | Next review |

### 5. SLO Monitoring

Production SLOs:

- **Availability**: 99.9% (43 min downtime/month)
- **Latency**: P95 < 5s, P99 < 10s
- **Error Rate**: < 1%
- **TTFT**: P95 < 2s

**Error Budget Policy**:
- 100% remaining → Normal operations
- 25% remaining → Deployment freeze
- 0% remaining → All hands reliability focus

---

## Getting Started

### Prerequisites

- Node.js 18+
- Docker & Docker Compose
- 8GB RAM (for local stack)

### Quick Start (5 minutes)

```bash
# 1. Clone and install
git clone <repo>
cd llm-simulator
npm install

# 2. Start observability stack
docker-compose -f docker-compose.observability.yml up -d

# 3. Configure environment
cp .env.example .env
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318

# 4. Start application
npm start

# 5. Access dashboards
# Grafana: http://localhost:3000 (admin/admin)
# Jaeger: http://localhost:16686
# Prometheus: http://localhost:9090
```

### Verify Installation

```bash
# Check OTLP collector
curl http://localhost:4318/v1/traces

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Send test request
curl -X POST http://localhost:3000/v1/completions \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "openai",
    "model": "gpt-4",
    "prompt": "Hello, world!"
  }'

# View metrics
curl http://localhost:9464/metrics | grep llm_
```

---

## Key Metrics Quick Reference

### Request Metrics

```promql
# Request rate
sum(rate(llm_requests_total[5m]))

# Success rate
sum(rate(llm_requests_total{status="success"}[5m])) / sum(rate(llm_requests_total[5m]))

# Error rate
sum(rate(llm_requests_errors_total[5m])) / sum(rate(llm_requests_total[5m]))
```

### Latency Metrics

```promql
# P95 latency
histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le))

# P95 TTFT
histogram_quantile(0.95, sum(rate(llm_latency_ttft_bucket[5m])) by (le))

# Average latency by provider
sum(rate(llm_requests_duration_sum[5m])) by (provider) / sum(rate(llm_requests_duration_count[5m])) by (provider)
```

### Cost Metrics

```promql
# Hourly cost rate
sum(rate(llm_cost_total[1h])) * 3600

# Cost by provider
sum(rate(llm_cost_total[1h])) by (provider) * 3600

# Average cost per request
sum(rate(llm_cost_total[1h])) / sum(rate(llm_requests_total[1h]))
```

### Token Metrics

```promql
# Token rate
sum(rate(llm_tokens_total[5m]))

# Tokens by type
sum(rate(llm_tokens_total[5m])) by (token_type)

# Average tokens per request
sum(rate(llm_tokens_per_request_sum[5m])) / sum(rate(llm_tokens_per_request_count[5m]))
```

---

## Dashboard Gallery

### Executive Dashboard
- Request rate and trends
- Error rate with SLO threshold
- P95/P99 latency
- Hourly cost and 24h total
- Token usage summary

### Performance Dashboard
- Latency heatmaps
- TTFT distribution
- Provider comparison
- Queue depth
- Cache hit rate

### Cost Analytics Dashboard
- Cost breakdown by provider/model
- Hourly/daily trends
- Cost per request distribution
- Budget tracking
- Cost efficiency metrics

### Error Analysis Dashboard
- Error rate trends
- Error distribution by type
- Provider reliability
- Recent error logs
- Incident timeline

---

## Integration Options

### Supported Platforms

| Platform | Traces | Metrics | Logs | Configuration |
|----------|--------|---------|------|---------------|
| **Self-Hosted** | Jaeger | Prometheus | Loki | docker-compose.yml |
| **Grafana Cloud** | Tempo | Prometheus | Loki | GRAFANA_CLOUD_API_KEY |
| **Datadog** | APM | Metrics | Logs | DD_API_KEY |
| **New Relic** | APM | Metrics | Logs | NEW_RELIC_LICENSE_KEY |

### Platform-Specific Setup

**Grafana Cloud:**
```bash
export GRAFANA_CLOUD_API_KEY=<your-key>
export GRAFANA_CLOUD_INSTANCE_ID=<your-instance>
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod-us-central-0.grafana.net/otlp
```

**Datadog:**
```bash
export DD_API_KEY=<your-key>
export DD_SITE=datadoghq.com
export OTEL_EXPORTER_OTLP_ENDPOINT=http://datadog-agent:4318
```

**New Relic:**
```bash
export NEW_RELIC_LICENSE_KEY=<your-key>
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.nr-data.net:4318
```

---

## Operational Playbooks

### High Error Rate Response

1. **Check error distribution**: `sum(rate(llm_requests_errors_total[5m])) by (error_type, provider)`
2. **Review recent traces**: Filter by status=error in Jaeger
3. **Check provider status**: Visit status pages
4. **Review deployments**: `kubectl rollout history`
5. **Take action**: Rollback, failover, or scale

### Latency Spike Investigation

1. **Identify source**: Check P95 by provider/model
2. **Analyze TTFT**: Is it provider or application?
3. **Check queue depth**: Are we backing up?
4. **Review traces**: Find slow spans
5. **Optimize**: Switch provider, scale, or optimize

### Cost Spike Alert

1. **Identify driver**: Top providers/models by cost
2. **Check volume**: Request and token rates
3. **Review patterns**: Any unusual activity?
4. **Mitigate**: Enable limits, optimize prompts
5. **Investigate**: Review logs and traces

---

## Performance Tuning

### Cardinality Management

**Current cardinality**: ~1,500 time series
**Budget**: 10,000 time series
**Headroom**: 85%

**Best Practices:**
- Never use user IDs, request IDs, or trace IDs as labels
- Limit label values to predefined sets
- Use recording rules for high-frequency queries
- Monitor cardinality regularly

### Sampling Strategy

**Development**: 100% sampling
**Staging**: 50% sampling
**Production**: 10% base sampling + intelligent tail sampling

**Tail Sampling Rules:**
- Always sample errors
- Always sample requests > 5s
- Sample 10% of normal traffic

### Resource Requirements

| Component | CPU | Memory | Storage |
|-----------|-----|--------|---------|
| Application | 2 cores | 4 GB | 10 GB |
| OTLP Collector | 2 cores | 2 GB | 5 GB |
| Prometheus | 4 cores | 16 GB | 500 GB |
| Jaeger | 2 cores | 4 GB | 100 GB |
| Grafana | 2 cores | 2 GB | 10 GB |

---

## Troubleshooting

### No traces appearing?

```bash
# Check collector is running
docker ps | grep otel-collector

# Test endpoint
curl http://localhost:4318/v1/traces

# Check logs
docker logs otel-collector

# Verify sampling
echo $TRACE_SAMPLING_RATE  # Should be > 0
```

### Metrics not showing?

```bash
# Check metrics endpoint
curl http://localhost:9464/metrics | grep llm_

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets'

# Verify metric registration
# Check application logs for metric creation
```

### High cardinality warning?

```promql
# Check cardinality
count({__name__=~"llm_.*"}) by (__name__)

# Find high-cardinality metrics
topk(10, count by (__name__)({__name__=~".+"}))
```

---

## Contributing

### Adding New Metrics

1. Define metric in `src/metrics/`
2. Add to metrics dictionary
3. Update recording rules if needed
4. Add to relevant dashboard
5. Update documentation

### Adding New Dashboards

1. Create dashboard in Grafana UI
2. Export JSON
3. Save to `config/grafana/dashboards/`
4. Add to provisioning config
5. Document in README

### Modifying SLOs

1. Propose change with data/justification
2. Update SLO definitions document
3. Update alert rules
4. Update dashboards
5. Communicate to stakeholders

---

## Resources

### Documentation

- [OpenTelemetry Docs](https://opentelemetry.io/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Grafana Dashboards](https://grafana.com/docs/grafana/latest/dashboards/)
- [SLO Guide](https://sre.google/workbook/implementing-slos/)

### Tools

- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
- [TraceQL Reference](https://grafana.com/docs/tempo/latest/traceql/)
- [LogQL Guide](https://grafana.com/docs/loki/latest/logql/)

### Community

- [CNCF Slack #opentelemetry](https://cloud-native.slack.com)
- [Prometheus Community](https://prometheus.io/community/)
- [Grafana Community](https://community.grafana.com/)

---

## License

This observability architecture is part of LLM-Simulator and follows the same license.

---

## Changelog

### Version 1.0 (2025-11-26)

- Initial release
- Full OpenTelemetry integration
- 19 core metrics
- 5 pre-built dashboards
- 12 alert rules
- SLO framework
- Complete documentation

---

## Maintainers

- **Platform Engineering Team**
- **Site Reliability Engineering**

**Questions?** Open an issue or contact platform-eng@example.com

---

**Status**: Production Ready | **Last Updated**: 2025-11-26 | **Version**: 1.0

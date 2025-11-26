# Observability and Monitoring Architecture

**Version:** 1.0
**Last Updated:** 2025-11-26
**Status:** Production-Ready
**Owner:** Platform Engineering

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [OpenTelemetry Integration](#opentelemetry-integration)
4. [Metrics Catalog](#metrics-catalog)
5. [Distributed Tracing](#distributed-tracing)
6. [Structured Logging](#structured-logging)
7. [Alerting Rules](#alerting-rules)
8. [Dashboards](#dashboards)
9. [SLO/SLI Definitions](#slosli-definitions)
10. [Cardinality Management](#cardinality-management)
11. [Platform Integrations](#platform-integrations)
12. [Operational Runbooks](#operational-runbooks)

---

## Executive Summary

The LLM-Simulator observability architecture provides comprehensive monitoring and operational visibility across all system components. Built on OpenTelemetry standards and integrated with industry-standard observability platforms, this architecture enables:

- **Real-time Performance Monitoring**: Track request latency, token usage, and LLM-specific metrics
- **Distributed Request Tracing**: Full visibility into request flow across simulation components
- **Intelligent Alerting**: Proactive detection of anomalies and SLO violations
- **Cost Optimization**: Detailed tracking of simulated and actual LLM costs
- **Production-Grade Reliability**: SLO-based monitoring with 99.9% uptime target

### Key Capabilities

| Capability | Implementation | Integration |
|------------|---------------|-------------|
| Tracing | OpenTelemetry SDK | Jaeger, Tempo, Datadog |
| Metrics | Prometheus + OTLP | Prometheus, Grafana Cloud, New Relic |
| Logging | Structured JSON | Loki, Elasticsearch, Datadog |
| Alerting | Prometheus Alertmanager | PagerDuty, OpsGenie, Slack |
| Dashboards | Grafana | Grafana, Datadog, New Relic |

---

## Architecture Overview

### High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          LLM-Simulator Application                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │   API Layer  │  │  Simulator   │  │   Provider   │                  │
│  │              │  │   Engine     │  │   Adapters   │                  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘                  │
│         │                  │                  │                          │
│         └──────────────────┴──────────────────┘                          │
│                            │                                             │
│         ┌──────────────────┴──────────────────┐                          │
│         │    OpenTelemetry SDK (Node.js)      │                          │
│         │  ┌────────┐ ┌────────┐ ┌─────────┐ │                          │
│         │  │ Traces │ │Metrics │ │  Logs   │ │                          │
│         │  └────────┘ └────────┘ └─────────┘ │                          │
│         └──────────────────┬──────────────────┘                          │
└────────────────────────────┼────────────────────────────────────────────┘
                             │
                   ┌─────────┴─────────┐
                   │  OTLP Collector   │
                   │  (OpenTelemetry)  │
                   └─────────┬─────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Trace Backend  │ │ Metrics Backend │ │  Log Backend    │
│                 │ │                 │ │                 │
│  • Jaeger       │ │  • Prometheus   │ │  • Loki         │
│  • Tempo        │ │  • VictoriaMetrics│ │ • Elasticsearch │
│  • Datadog APM  │ │  • Datadog      │ │  • Datadog      │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                   ┌─────────┴─────────┐
                   │   Visualization   │
                   │                   │
                   │  • Grafana        │
                   │  • Datadog UI     │
                   │  • Custom Dashboards│
                   └───────────────────┘
                             │
                   ┌─────────┴─────────┐
                   │  Alerting Layer   │
                   │                   │
                   │  • Alertmanager   │
                   │  • PagerDuty      │
                   │  • OpsGenie       │
                   └───────────────────┘
```

### Data Flow Architecture

```
Request → Application Instrumentation → OTLP Collector → Storage → Query/Alert
   │              │                           │              │          │
   │              ├─ Traces                   ├─ Sampling    │          │
   │              ├─ Metrics (push/pull)      ├─ Batching    │          │
   │              └─ Logs (structured)        └─ Routing     │          │
   │                                                          │          │
   └──────────── Correlation IDs ────────────────────────────┴──────────┘
```

### Component Responsibilities

| Component | Responsibility | Technology |
|-----------|---------------|------------|
| **Instrumentation** | Capture telemetry at source | OpenTelemetry SDK |
| **Collector** | Receive, process, export telemetry | OTEL Collector |
| **Storage** | Persist telemetry data | Prometheus, Jaeger, Loki |
| **Query** | Retrieve and analyze data | PromQL, TraceQL, LogQL |
| **Visualization** | Display metrics and traces | Grafana, Datadog |
| **Alerting** | Notify on threshold violations | Alertmanager, PagerDuty |

---

## OpenTelemetry Integration

### SDK Configuration

```typescript
// config/observability.ts
import { NodeSDK } from '@opentelemetry/sdk-node';
import { getNodeAutoInstrumentations } from '@opentelemetry/auto-instrumentations-node';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-proto';
import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-proto';
import { PeriodicExportingMetricReader } from '@opentelemetry/sdk-metrics';
import { Resource } from '@opentelemetry/resources';
import { SemanticResourceAttributes } from '@opentelemetry/semantic-conventions';

export const initializeObservability = () => {
  const sdk = new NodeSDK({
    resource: new Resource({
      [SemanticResourceAttributes.SERVICE_NAME]: 'llm-simulator',
      [SemanticResourceAttributes.SERVICE_VERSION]: process.env.SERVICE_VERSION || '1.0.0',
      [SemanticResourceAttributes.DEPLOYMENT_ENVIRONMENT]: process.env.NODE_ENV || 'development',
      'service.instance.id': process.env.HOSTNAME || 'local',
      'service.namespace': 'llm-platform',
    }),

    traceExporter: new OTLPTraceExporter({
      url: process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT || 'http://localhost:4318/v1/traces',
      headers: {
        'Authorization': `Bearer ${process.env.OTEL_AUTH_TOKEN}`,
      },
    }),

    metricReader: new PeriodicExportingMetricReader({
      exporter: new OTLPMetricExporter({
        url: process.env.OTEL_EXPORTER_OTLP_METRICS_ENDPOINT || 'http://localhost:4318/v1/metrics',
        headers: {
          'Authorization': `Bearer ${process.env.OTEL_AUTH_TOKEN}`,
        },
      }),
      exportIntervalMillis: 60000, // 1 minute
    }),

    instrumentations: [
      getNodeAutoInstrumentations({
        '@opentelemetry/instrumentation-http': {
          enabled: true,
          ignoreIncomingPaths: ['/health', '/metrics'],
        },
        '@opentelemetry/instrumentation-express': { enabled: true },
        '@opentelemetry/instrumentation-mongodb': { enabled: true },
        '@opentelemetry/instrumentation-redis': { enabled: true },
      }),
    ],
  });

  sdk.start();

  // Graceful shutdown
  process.on('SIGTERM', () => {
    sdk.shutdown()
      .then(() => console.log('Tracing terminated'))
      .catch((error) => console.log('Error terminating tracing', error))
      .finally(() => process.exit(0));
  });

  return sdk;
};
```

### Custom Instrumentation

```typescript
// instrumentation/llm-tracer.ts
import { trace, context, SpanStatusCode } from '@opentelemetry/api';
import { LLMRequest, LLMResponse } from '../types';

const tracer = trace.getTracer('llm-simulator', '1.0.0');

export class LLMInstrumentation {

  static async traceCompletion(
    provider: string,
    model: string,
    operation: () => Promise<LLMResponse>
  ): Promise<LLMResponse> {
    return tracer.startActiveSpan(
      'llm.completion',
      {
        kind: trace.SpanKind.CLIENT,
        attributes: {
          'llm.provider': provider,
          'llm.model': model,
          'llm.operation_type': 'completion',
        },
      },
      async (span) => {
        try {
          const startTime = Date.now();
          const response = await operation();
          const duration = Date.now() - startTime;

          // Add response attributes
          span.setAttributes({
            'llm.response.tokens.prompt': response.usage.prompt_tokens,
            'llm.response.tokens.completion': response.usage.completion_tokens,
            'llm.response.tokens.total': response.usage.total_tokens,
            'llm.response.duration_ms': duration,
            'llm.response.finish_reason': response.choices[0].finish_reason,
            'llm.response.model': response.model,
            'llm.cost.estimated': this.calculateCost(response),
          });

          span.setStatus({ code: SpanStatusCode.OK });
          return response;
        } catch (error) {
          span.setStatus({
            code: SpanStatusCode.ERROR,
            message: error.message,
          });
          span.recordException(error);
          throw error;
        } finally {
          span.end();
        }
      }
    );
  }

  static async traceStreaming(
    provider: string,
    model: string,
    operation: () => AsyncGenerator<any>
  ): Promise<AsyncGenerator<any>> {
    const span = tracer.startSpan('llm.streaming', {
      kind: trace.SpanKind.CLIENT,
      attributes: {
        'llm.provider': provider,
        'llm.model': model,
        'llm.operation_type': 'streaming',
      },
    });

    const ctx = trace.setSpan(context.active(), span);

    return context.with(ctx, async function* () {
      try {
        let tokenCount = 0;
        let firstTokenTime: number | null = null;
        const startTime = Date.now();

        for await (const chunk of operation()) {
          if (firstTokenTime === null) {
            firstTokenTime = Date.now();
            span.setAttribute('llm.ttft_ms', firstTokenTime - startTime);
          }
          tokenCount++;
          yield chunk;
        }

        const totalDuration = Date.now() - startTime;
        span.setAttributes({
          'llm.response.tokens.total': tokenCount,
          'llm.response.duration_ms': totalDuration,
          'llm.response.tokens_per_second': tokenCount / (totalDuration / 1000),
        });

        span.setStatus({ code: SpanStatusCode.OK });
      } catch (error) {
        span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
        span.recordException(error);
        throw error;
      } finally {
        span.end();
      }
    }());
  }

  private static calculateCost(response: LLMResponse): number {
    // Simplified cost calculation - extend based on actual pricing
    const costPerToken = 0.00002; // $0.02 per 1K tokens
    return response.usage.total_tokens * costPerToken;
  }
}
```

---

## Metrics Catalog

### Metric Naming Convention

Format: `{namespace}_{subsystem}_{metric}_{unit}`

Example: `llm_simulator_requests_total`

### Core Metrics

#### 1. Request Metrics

```typescript
// metrics/request-metrics.ts
import { metrics } from '@opentelemetry/api';
import { MeterProvider } from '@opentelemetry/sdk-metrics';

const meter = metrics.getMeter('llm-simulator');

// Counter: Total requests
export const requestsTotal = meter.createCounter('llm.requests.total', {
  description: 'Total number of LLM requests',
  unit: '1',
});

// Histogram: Request duration
export const requestDuration = meter.createHistogram('llm.requests.duration', {
  description: 'LLM request duration in milliseconds',
  unit: 'ms',
  advice: {
    explicitBucketBoundaries: [10, 50, 100, 250, 500, 1000, 2500, 5000, 10000],
  },
});

// Counter: Request errors
export const requestErrors = meter.createCounter('llm.requests.errors.total', {
  description: 'Total number of failed LLM requests',
  unit: '1',
});

// Usage example
requestsTotal.add(1, {
  'provider': 'openai',
  'model': 'gpt-4',
  'operation': 'completion',
  'status': 'success',
});

requestDuration.record(1234, {
  'provider': 'openai',
  'model': 'gpt-4',
  'operation': 'completion',
});
```

#### 2. Token Metrics

```typescript
// metrics/token-metrics.ts

// Counter: Token usage
export const tokensTotal = meter.createCounter('llm.tokens.total', {
  description: 'Total number of tokens processed',
  unit: 'tokens',
});

// Histogram: Tokens per request distribution
export const tokensPerRequest = meter.createHistogram('llm.tokens.per_request', {
  description: 'Distribution of tokens per request',
  unit: 'tokens',
  advice: {
    explicitBucketBoundaries: [100, 500, 1000, 2000, 4000, 8000, 16000, 32000],
  },
});

// Usage example
tokensTotal.add(usage.total_tokens, {
  'provider': 'openai',
  'model': 'gpt-4',
  'token_type': 'total',
});

tokensTotal.add(usage.prompt_tokens, {
  'provider': 'openai',
  'model': 'gpt-4',
  'token_type': 'prompt',
});

tokensTotal.add(usage.completion_tokens, {
  'provider': 'openai',
  'model': 'gpt-4',
  'token_type': 'completion',
});
```

#### 3. Latency Metrics

```typescript
// metrics/latency-metrics.ts

// Histogram: Time to First Token (TTFT)
export const timeToFirstToken = meter.createHistogram('llm.latency.ttft', {
  description: 'Time to first token in streaming responses',
  unit: 'ms',
  advice: {
    explicitBucketBoundaries: [50, 100, 250, 500, 1000, 2000, 5000],
  },
});

// Histogram: Inter-Token Latency (ITL)
export const interTokenLatency = meter.createHistogram('llm.latency.inter_token', {
  description: 'Time between tokens in streaming responses',
  unit: 'ms',
  advice: {
    explicitBucketBoundaries: [5, 10, 25, 50, 100, 250, 500],
  },
});

// Histogram: End-to-End Latency (E2E)
export const endToEndLatency = meter.createHistogram('llm.latency.e2e', {
  description: 'Total request processing time',
  unit: 'ms',
  advice: {
    explicitBucketBoundaries: [100, 500, 1000, 2500, 5000, 10000, 30000],
  },
});

// Histogram: Provider API latency
export const providerApiLatency = meter.createHistogram('llm.latency.provider_api', {
  description: 'External provider API response time',
  unit: 'ms',
  advice: {
    explicitBucketBoundaries: [100, 250, 500, 1000, 2000, 5000, 10000],
  },
});
```

#### 4. Error Metrics

```typescript
// metrics/error-metrics.ts

// Counter: Errors by type
export const errorsByType = meter.createCounter('llm.errors.by_type', {
  description: 'Errors categorized by type',
  unit: '1',
});

// Counter: Rate limit errors
export const rateLimitErrors = meter.createCounter('llm.errors.rate_limit', {
  description: 'Number of rate limit errors',
  unit: '1',
});

// Counter: Timeout errors
export const timeoutErrors = meter.createCounter('llm.errors.timeout', {
  description: 'Number of timeout errors',
  unit: '1',
});

// Usage example
errorsByType.add(1, {
  'provider': 'openai',
  'error_type': 'rate_limit',
  'error_code': '429',
  'model': 'gpt-4',
});
```

#### 5. Cost Metrics

```typescript
// metrics/cost-metrics.ts

// Counter: Cumulative cost
export const cumulativeCost = meter.createCounter('llm.cost.total', {
  description: 'Total estimated cost in USD',
  unit: 'USD',
});

// Histogram: Cost per request
export const costPerRequest = meter.createHistogram('llm.cost.per_request', {
  description: 'Cost distribution per request',
  unit: 'USD',
  advice: {
    explicitBucketBoundaries: [0.001, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
  },
});

// Gauge: Current hourly rate
export const hourlyRate = meter.createObservableGauge('llm.cost.hourly_rate', {
  description: 'Current cost rate per hour',
  unit: 'USD/hour',
});

// Usage example
cumulativeCost.add(cost, {
  'provider': 'openai',
  'model': 'gpt-4',
  'cost_component': 'completion',
});
```

#### 6. System Metrics

```typescript
// metrics/system-metrics.ts

// Gauge: Active connections
export const activeConnections = meter.createObservableGauge('llm.connections.active', {
  description: 'Number of active provider connections',
  unit: '1',
});

// Gauge: Queue depth
export const queueDepth = meter.createObservableGauge('llm.queue.depth', {
  description: 'Number of requests in queue',
  unit: '1',
});

// Histogram: Memory usage
export const memoryUsage = meter.createHistogram('llm.system.memory_usage', {
  description: 'Application memory usage',
  unit: 'bytes',
});

// Counter: Cache hits/misses
export const cacheHits = meter.createCounter('llm.cache.hits', {
  description: 'Number of cache hits',
  unit: '1',
});

export const cacheMisses = meter.createCounter('llm.cache.misses', {
  description: 'Number of cache misses',
  unit: '1',
});
```

### Complete Metrics Reference Table

| Metric Name | Type | Unit | Labels | Description | Cardinality |
|-------------|------|------|--------|-------------|-------------|
| `llm.requests.total` | Counter | 1 | provider, model, operation, status | Total LLM requests | Low (16) |
| `llm.requests.duration` | Histogram | ms | provider, model, operation | Request duration | Low (12) |
| `llm.requests.errors.total` | Counter | 1 | provider, model, error_type, error_code | Total errors | Medium (48) |
| `llm.tokens.total` | Counter | tokens | provider, model, token_type | Total tokens processed | Low (12) |
| `llm.tokens.per_request` | Histogram | tokens | provider, model | Tokens per request dist | Low (8) |
| `llm.latency.ttft` | Histogram | ms | provider, model | Time to first token | Low (8) |
| `llm.latency.inter_token` | Histogram | ms | provider, model | Inter-token latency | Low (8) |
| `llm.latency.e2e` | Histogram | ms | provider, model, operation | End-to-end latency | Low (12) |
| `llm.latency.provider_api` | Histogram | ms | provider, endpoint | Provider API latency | Low (8) |
| `llm.cost.total` | Counter | USD | provider, model, cost_component | Cumulative cost | Low (12) |
| `llm.cost.per_request` | Histogram | USD | provider, model | Cost per request | Low (8) |
| `llm.cost.hourly_rate` | Gauge | USD/hour | provider | Current hourly rate | Low (4) |
| `llm.connections.active` | Gauge | 1 | provider | Active connections | Low (4) |
| `llm.queue.depth` | Gauge | 1 | priority | Queue depth | Low (3) |
| `llm.cache.hits` | Counter | 1 | cache_type | Cache hits | Low (3) |
| `llm.cache.misses` | Counter | 1 | cache_type | Cache misses | Low (3) |
| `llm.errors.rate_limit` | Counter | 1 | provider, model | Rate limit errors | Low (8) |
| `llm.errors.timeout` | Counter | 1 | provider, model | Timeout errors | Low (8) |
| `llm.system.memory_usage` | Histogram | bytes | component | Memory usage | Low (5) |

**Total Estimated Cardinality:** ~1,500 time series (assuming 4 providers, 5 models per provider)

---

## Distributed Tracing

### Trace Span Hierarchy

```
Root Span: HTTP Request
│
├─ Span: Request Validation
│  └─ Span: Schema Validation
│
├─ Span: Rate Limit Check
│  └─ Span: Redis GET
│
├─ Span: LLM Completion (Parent)
│  │
│  ├─ Span: Provider Selection
│  │
│  ├─ Span: Prompt Processing
│  │  ├─ Span: Template Rendering
│  │  └─ Span: Token Counting
│  │
│  ├─ Span: Provider API Call
│  │  ├─ Span: HTTP Request to OpenAI
│  │  └─ Span: Response Parsing
│  │
│  └─ Span: Response Processing
│     ├─ Span: Token Calculation
│     └─ Span: Cost Calculation
│
├─ Span: Cache Update
│  └─ Span: Redis SET
│
└─ Span: Response Serialization
```

### Trace Context Propagation

```typescript
// tracing/context-propagation.ts
import { context, trace, propagation } from '@opentelemetry/api';
import { W3CTraceContextPropagator } from '@opentelemetry/core';

// Configure propagation
propagation.setGlobalPropagator(new W3CTraceContextPropagator());

export function extractTraceContext(headers: Record<string, string>) {
  return propagation.extract(context.active(), headers);
}

export function injectTraceContext(ctx: any, headers: Record<string, string>) {
  propagation.inject(ctx, headers);
  return headers;
}

// Example: Propagate context to external HTTP call
async function callExternalApi(url: string, data: any) {
  const headers = {};
  injectTraceContext(context.active(), headers);

  return fetch(url, {
    method: 'POST',
    headers: {
      ...headers,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });
}
```

### Span Attributes Schema

#### Standard Attributes

| Attribute | Type | Example | Description |
|-----------|------|---------|-------------|
| `http.method` | string | `POST` | HTTP method |
| `http.url` | string | `/v1/completions` | Request URL |
| `http.status_code` | number | `200` | HTTP status code |
| `http.user_agent` | string | `llm-client/1.0` | User agent |

#### LLM-Specific Attributes

| Attribute | Type | Example | Description |
|-----------|------|---------|-------------|
| `llm.provider` | string | `openai` | LLM provider |
| `llm.model` | string | `gpt-4` | Model name |
| `llm.operation_type` | string | `completion` | Operation type |
| `llm.request.temperature` | number | `0.7` | Temperature setting |
| `llm.request.max_tokens` | number | `1000` | Max tokens requested |
| `llm.request.stream` | boolean | `true` | Streaming enabled |
| `llm.response.tokens.prompt` | number | `150` | Prompt tokens |
| `llm.response.tokens.completion` | number | `850` | Completion tokens |
| `llm.response.tokens.total` | number | `1000` | Total tokens |
| `llm.response.finish_reason` | string | `stop` | Why completion ended |
| `llm.response.model` | string | `gpt-4-0613` | Actual model used |
| `llm.cost.estimated` | number | `0.045` | Estimated cost USD |
| `llm.ttft_ms` | number | `234` | Time to first token |
| `llm.tokens_per_second` | number | `45.2` | Tokens per second |
| `llm.cache.hit` | boolean | `false` | Cache hit |
| `llm.retry.count` | number | `1` | Number of retries |
| `llm.retry.reason` | string | `rate_limit` | Retry reason |

### Sampling Strategy

```typescript
// tracing/sampling.ts
import { ParentBasedSampler, TraceIdRatioBasedSampler } from '@opentelemetry/sdk-trace-node';

export const sampler = new ParentBasedSampler({
  root: new TraceIdRatioBasedSampler(
    process.env.TRACE_SAMPLING_RATE ? parseFloat(process.env.TRACE_SAMPLING_RATE) : 0.1
  ),
});

// Environment-based sampling
// - Production: 10% sampling (0.1)
// - Staging: 50% sampling (0.5)
// - Development: 100% sampling (1.0)
```

### Trace Analysis Queries

#### TraceQL Examples (Tempo/Grafana)

```traceql
-- Find slow LLM requests (>5s)
{ span.llm.provider = "openai" && duration > 5s }

-- Find requests with high token usage
{ span.llm.response.tokens.total > 10000 }

-- Find errors from specific provider
{ span.llm.provider = "anthropic" && status = error }

-- Find cache misses with slow response
{ span.llm.cache.hit = false && duration > 2s }

-- Analyze TTFT distribution
{ span.llm.ttft_ms > 500 } | histogram(span.llm.ttft_ms)
```

---

## Structured Logging

### Log Schema Specification

```typescript
// logging/schema.ts

export interface LogEntry {
  // Standard Fields
  timestamp: string;           // ISO 8601 format
  level: 'debug' | 'info' | 'warn' | 'error' | 'fatal';
  message: string;

  // Correlation Fields
  trace_id: string;
  span_id: string;
  parent_span_id?: string;

  // Service Fields
  service: {
    name: string;
    version: string;
    instance_id: string;
    environment: string;
  };

  // Request Fields
  request?: {
    id: string;
    method: string;
    path: string;
    user_agent: string;
    ip_address: string;
  };

  // LLM Fields
  llm?: {
    provider: string;
    model: string;
    operation: string;
    tokens?: {
      prompt: number;
      completion: number;
      total: number;
    };
    duration_ms?: number;
    cost_usd?: number;
  };

  // Error Fields
  error?: {
    type: string;
    message: string;
    stack?: string;
    code?: string;
  };

  // Custom Fields
  [key: string]: any;
}
```

### Logger Implementation

```typescript
// logging/logger.ts
import winston from 'winston';
import { trace, context } from '@opentelemetry/api';

export class StructuredLogger {
  private logger: winston.Logger;

  constructor() {
    this.logger = winston.createLogger({
      level: process.env.LOG_LEVEL || 'info',
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.errors({ stack: true }),
        winston.format.json()
      ),
      defaultMeta: {
        service: {
          name: 'llm-simulator',
          version: process.env.SERVICE_VERSION || '1.0.0',
          instance_id: process.env.HOSTNAME || 'local',
          environment: process.env.NODE_ENV || 'development',
        },
      },
      transports: [
        new winston.transports.Console(),
        new winston.transports.File({
          filename: 'logs/error.log',
          level: 'error'
        }),
        new winston.transports.File({
          filename: 'logs/combined.log'
        }),
      ],
    });
  }

  private enrichWithTraceContext(meta: any = {}) {
    const span = trace.getSpan(context.active());
    if (span) {
      const spanContext = span.spanContext();
      return {
        ...meta,
        trace_id: spanContext.traceId,
        span_id: spanContext.spanId,
      };
    }
    return meta;
  }

  info(message: string, meta?: any) {
    this.logger.info(message, this.enrichWithTraceContext(meta));
  }

  error(message: string, error?: Error, meta?: any) {
    this.logger.error(message, this.enrichWithTraceContext({
      ...meta,
      error: error ? {
        type: error.name,
        message: error.message,
        stack: error.stack,
      } : undefined,
    }));
  }

  warn(message: string, meta?: any) {
    this.logger.warn(message, this.enrichWithTraceContext(meta));
  }

  debug(message: string, meta?: any) {
    this.logger.debug(message, this.enrichWithTraceContext(meta));
  }

  // LLM-specific logging
  logLLMRequest(provider: string, model: string, meta?: any) {
    this.info('LLM request initiated', {
      llm: {
        provider,
        model,
        operation: meta?.operation || 'completion',
      },
      ...meta,
    });
  }

  logLLMResponse(provider: string, model: string, usage: any, duration: number, cost: number) {
    this.info('LLM response received', {
      llm: {
        provider,
        model,
        tokens: {
          prompt: usage.prompt_tokens,
          completion: usage.completion_tokens,
          total: usage.total_tokens,
        },
        duration_ms: duration,
        cost_usd: cost,
      },
    });
  }

  logLLMError(provider: string, model: string, error: Error, meta?: any) {
    this.error('LLM request failed', error, {
      llm: {
        provider,
        model,
        operation: meta?.operation || 'completion',
      },
      ...meta,
    });
  }
}

export const logger = new StructuredLogger();
```

### Log Levels and Usage

| Level | Usage | Examples | Retention |
|-------|-------|----------|-----------|
| **DEBUG** | Development debugging | Variable values, state transitions | 1 day |
| **INFO** | Normal operations | Request started, cache hit, response sent | 7 days |
| **WARN** | Unexpected but handled | Retry attempt, fallback used, deprecation | 30 days |
| **ERROR** | Errors requiring attention | Failed request, invalid response, timeout | 90 days |
| **FATAL** | Critical system failures | Service crash, database unreachable | 180 days |

### LogQL Query Examples (Loki)

```logql
-- Find all errors from OpenAI provider
{service="llm-simulator"} | json | llm_provider="openai" | level="error"

-- Find high-cost requests (>$1)
{service="llm-simulator"} | json | llm_cost_usd > 1

-- Find slow requests by trace_id
{service="llm-simulator"} | json | llm_duration_ms > 5000 | line_format "{{.trace_id}}"

-- Rate limit errors in last hour
sum(rate({service="llm-simulator"} | json | error_code="429" [1h]))

-- Top error types
topk(10, sum by (error_type) (count_over_time({service="llm-simulator"} | json | level="error" [24h])))
```

---

## Alerting Rules

### Alert Severity Levels

| Severity | Response Time | Escalation | Examples |
|----------|--------------|------------|----------|
| **P0 - Critical** | Immediate | Page on-call + manager | Service down, data loss |
| **P1 - High** | 15 minutes | Page on-call | High error rate, SLO violation |
| **P2 - Medium** | 1 hour | Ticket to team | Elevated latency, cache degradation |
| **P3 - Low** | Next business day | Ticket to backlog | Minor performance degradation |

### Prometheus Alert Rules

```yaml
# config/alerts/llm-simulator-alerts.yaml

groups:
  - name: llm_simulator_slo_alerts
    interval: 30s
    rules:

      # CRITICAL: Service availability
      - alert: LLMSimulatorDown
        expr: up{job="llm-simulator"} == 0
        for: 1m
        labels:
          severity: critical
          component: platform
        annotations:
          summary: "LLM Simulator service is down"
          description: "Instance {{ $labels.instance }} has been down for more than 1 minute"
          runbook_url: "https://runbooks.example.com/llm-simulator-down"

      # CRITICAL: High error rate
      - alert: HighErrorRate
        expr: |
          (
            sum(rate(llm_requests_errors_total[5m]))
            /
            sum(rate(llm_requests_total[5m]))
          ) > 0.05
        for: 5m
        labels:
          severity: critical
          component: requests
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"
          runbook_url: "https://runbooks.example.com/high-error-rate"

      # HIGH: SLO violation - Latency
      - alert: LatencySLOViolation
        expr: |
          histogram_quantile(0.99,
            sum(rate(llm_requests_duration_bucket[5m])) by (le)
          ) > 5000
        for: 10m
        labels:
          severity: high
          component: performance
        annotations:
          summary: "P99 latency SLO violation"
          description: "P99 latency is {{ $value }}ms (SLO: 5000ms)"
          runbook_url: "https://runbooks.example.com/latency-slo-violation"

      # HIGH: Rate limit errors
      - alert: HighRateLimitErrors
        expr: |
          sum(rate(llm_errors_rate_limit[5m])) by (provider) > 0.1
        for: 5m
        labels:
          severity: high
          component: provider
        annotations:
          summary: "High rate of rate limit errors from {{ $labels.provider }}"
          description: "Rate limit errors: {{ $value }} errors/sec"
          runbook_url: "https://runbooks.example.com/rate-limit-errors"

  - name: llm_simulator_performance_alerts
    interval: 30s
    rules:

      # MEDIUM: Elevated latency
      - alert: ElevatedLatency
        expr: |
          histogram_quantile(0.95,
            sum(rate(llm_requests_duration_bucket[5m])) by (le, provider, model)
          ) > 3000
        for: 15m
        labels:
          severity: medium
          component: performance
        annotations:
          summary: "Elevated P95 latency for {{ $labels.provider }}/{{ $labels.model }}"
          description: "P95 latency: {{ $value }}ms (warning threshold: 3000ms)"
          runbook_url: "https://runbooks.example.com/elevated-latency"

      # MEDIUM: High TTFT
      - alert: HighTimeToFirstToken
        expr: |
          histogram_quantile(0.95,
            sum(rate(llm_latency_ttft_bucket[5m])) by (le, provider)
          ) > 2000
        for: 10m
        labels:
          severity: medium
          component: streaming
        annotations:
          summary: "High TTFT for {{ $labels.provider }}"
          description: "P95 TTFT: {{ $value }}ms (threshold: 2000ms)"

      # MEDIUM: Cache miss rate
      - alert: HighCacheMissRate
        expr: |
          (
            sum(rate(llm_cache_misses[5m]))
            /
            (sum(rate(llm_cache_hits[5m])) + sum(rate(llm_cache_misses[5m])))
          ) > 0.8
        for: 15m
        labels:
          severity: medium
          component: cache
        annotations:
          summary: "High cache miss rate"
          description: "Cache miss rate: {{ $value | humanizePercentage }}"

  - name: llm_simulator_cost_alerts
    interval: 1m
    rules:

      # HIGH: Unexpected cost spike
      - alert: CostSpike
        expr: |
          (
            rate(llm_cost_total[5m])
            /
            rate(llm_cost_total[1h] offset 1h)
          ) > 2
        for: 5m
        labels:
          severity: high
          component: cost
        annotations:
          summary: "Unexpected cost spike detected"
          description: "Current cost rate is {{ $value }}x higher than 1 hour ago"
          runbook_url: "https://runbooks.example.com/cost-spike"

      # MEDIUM: High hourly cost
      - alert: HighHourlyCost
        expr: llm_cost_hourly_rate > 100
        for: 10m
        labels:
          severity: medium
          component: cost
        annotations:
          summary: "High hourly cost rate"
          description: "Current rate: ${{ $value }}/hour"

  - name: llm_simulator_capacity_alerts
    interval: 30s
    rules:

      # HIGH: Queue depth
      - alert: HighQueueDepth
        expr: llm_queue_depth > 100
        for: 5m
        labels:
          severity: high
          component: capacity
        annotations:
          summary: "High request queue depth"
          description: "Queue depth: {{ $value }} requests"
          runbook_url: "https://runbooks.example.com/high-queue-depth"

      # MEDIUM: Memory pressure
      - alert: HighMemoryUsage
        expr: |
          (
            process_resident_memory_bytes{job="llm-simulator"}
            /
            1024 / 1024 / 1024
          ) > 2
        for: 10m
        labels:
          severity: medium
          component: system
        annotations:
          summary: "High memory usage"
          description: "Memory usage: {{ $value }}GB"

  - name: llm_simulator_business_alerts
    interval: 1m
    rules:

      # MEDIUM: Low throughput
      - alert: LowThroughput
        expr: |
          sum(rate(llm_requests_total[5m])) < 1
        for: 30m
        labels:
          severity: medium
          component: business
        annotations:
          summary: "Abnormally low request throughput"
          description: "Current rate: {{ $value }} req/sec (expected: >1 req/sec)"

      # LOW: Provider availability
      - alert: ProviderDegradation
        expr: |
          (
            sum(rate(llm_requests_errors_total[5m])) by (provider)
            /
            sum(rate(llm_requests_total[5m])) by (provider)
          ) > 0.1
        for: 15m
        labels:
          severity: low
          component: provider
        annotations:
          summary: "Provider {{ $labels.provider }} showing degradation"
          description: "Error rate: {{ $value | humanizePercentage }}"
```

### Alertmanager Configuration

```yaml
# config/alertmanager.yaml

global:
  resolve_timeout: 5m
  slack_api_url: '<slack_webhook_url>'
  pagerduty_url: 'https://events.pagerduty.com/v2/enqueue'

route:
  receiver: 'default'
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h

  routes:
    # Critical alerts to PagerDuty
    - match:
        severity: critical
      receiver: pagerduty-critical
      continue: true

    # Critical alerts also to Slack
    - match:
        severity: critical
      receiver: slack-critical

    # High severity to PagerDuty
    - match:
        severity: high
      receiver: pagerduty-high
      continue: true

    # High severity also to Slack
    - match:
        severity: high
      receiver: slack-high

    # Medium/Low to Slack only
    - match_re:
        severity: (medium|low)
      receiver: slack-general

receivers:
  - name: 'default'
    slack_configs:
      - channel: '#llm-alerts'
        title: 'LLM Simulator Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '<pagerduty_service_key>'
        severity: critical
        description: '{{ .CommonAnnotations.summary }}'

  - name: 'pagerduty-high'
    pagerduty_configs:
      - service_key: '<pagerduty_service_key>'
        severity: error
        description: '{{ .CommonAnnotations.summary }}'

  - name: 'slack-critical'
    slack_configs:
      - channel: '#llm-critical'
        color: danger
        title: '[CRITICAL] {{ .CommonAnnotations.summary }}'
        text: |
          {{ range .Alerts }}
          *Alert:* {{ .Annotations.summary }}
          *Description:* {{ .Annotations.description }}
          *Runbook:* {{ .Annotations.runbook_url }}
          {{ end }}

  - name: 'slack-high'
    slack_configs:
      - channel: '#llm-alerts'
        color: warning
        title: '[HIGH] {{ .CommonAnnotations.summary }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'slack-general'
    slack_configs:
      - channel: '#llm-alerts'
        color: good
        title: '{{ .CommonAnnotations.summary }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

inhibit_rules:
  # Inhibit medium/low alerts if critical is firing
  - source_match:
      severity: critical
    target_match_re:
      severity: (medium|low)
    equal: ['alertname', 'instance']
```

### Alert Response Matrix

| Alert | Severity | Response Team | Max Response Time | Required Actions |
|-------|----------|--------------|-------------------|------------------|
| LLMSimulatorDown | P0 | On-call SRE | Immediate | 1. Check service health<br>2. Review logs<br>3. Restart if needed |
| HighErrorRate | P0 | On-call SRE | 5 min | 1. Check error patterns<br>2. Review traces<br>3. Rollback if recent deploy |
| LatencySLOViolation | P1 | On-call SRE | 15 min | 1. Check provider status<br>2. Review performance metrics<br>3. Scale if needed |
| HighRateLimitErrors | P1 | On-call Dev | 15 min | 1. Check rate limit config<br>2. Enable backoff<br>3. Contact provider |
| ElevatedLatency | P2 | Dev Team | 1 hour | 1. Monitor trend<br>2. Investigate if persists<br>3. Consider optimization |
| CostSpike | P1 | Product + Eng | 30 min | 1. Verify usage pattern<br>2. Check for abuse<br>3. Enable cost limits |

---

## Dashboards

### Grafana Dashboard Structure

#### 1. Executive Overview Dashboard

```json
{
  "dashboard": {
    "title": "LLM Simulator - Executive Overview",
    "tags": ["llm", "overview"],
    "timezone": "browser",
    "refresh": "30s",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_requests_total[5m]))",
            "legendFormat": "Requests/sec"
          }
        ],
        "gridPos": {"x": 0, "y": 0, "w": 12, "h": 8}
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_requests_errors_total[5m])) / sum(rate(llm_requests_total[5m]))",
            "legendFormat": "Error Rate"
          }
        ],
        "gridPos": {"x": 12, "y": 0, "w": 12, "h": 8},
        "alert": {
          "conditions": [
            {
              "evaluator": {"params": [0.05], "type": "gt"},
              "operator": {"type": "and"},
              "query": {"params": ["A", "5m", "now"]},
              "reducer": {"params": [], "type": "avg"},
              "type": "query"
            }
          ]
        }
      },
      {
        "title": "P95 Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, sum(rate(llm_requests_duration_bucket[5m])) by (le))",
            "legendFormat": "P99"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 12, "h": 8}
      },
      {
        "title": "Hourly Cost",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(llm_cost_hourly_rate)",
            "legendFormat": "$/hour"
          }
        ],
        "gridPos": {"x": 12, "y": 8, "w": 6, "h": 4}
      },
      {
        "title": "Token Usage (24h)",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(increase(llm_tokens_total[24h]))",
            "legendFormat": "Total Tokens"
          }
        ],
        "gridPos": {"x": 18, "y": 8, "w": 6, "h": 4}
      }
    ]
  }
}
```

#### 2. Performance Deep Dive Dashboard

```json
{
  "dashboard": {
    "title": "LLM Simulator - Performance Deep Dive",
    "tags": ["llm", "performance"],
    "panels": [
      {
        "title": "Latency Heatmap",
        "type": "heatmap",
        "targets": [
          {
            "expr": "sum(increase(llm_requests_duration_bucket[1m])) by (le)",
            "format": "heatmap"
          }
        ],
        "gridPos": {"x": 0, "y": 0, "w": 24, "h": 8}
      },
      {
        "title": "Latency by Provider",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le, provider))",
            "legendFormat": "{{ provider }} - P95"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 12, "h": 8}
      },
      {
        "title": "Latency by Model",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le, model))",
            "legendFormat": "{{ model }} - P95"
          }
        ],
        "gridPos": {"x": 12, "y": 8, "w": 12, "h": 8}
      },
      {
        "title": "Time to First Token",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, sum(rate(llm_latency_ttft_bucket[5m])) by (le, provider))",
            "legendFormat": "{{ provider }} - P50"
          },
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_latency_ttft_bucket[5m])) by (le, provider))",
            "legendFormat": "{{ provider }} - P95"
          }
        ],
        "gridPos": {"x": 0, "y": 16, "w": 12, "h": 8}
      },
      {
        "title": "Inter-Token Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_latency_inter_token_bucket[5m])) by (le, provider))",
            "legendFormat": "{{ provider }} - P95"
          }
        ],
        "gridPos": {"x": 12, "y": 16, "w": 12, "h": 8}
      }
    ]
  }
}
```

#### 3. Cost Analytics Dashboard

```json
{
  "dashboard": {
    "title": "LLM Simulator - Cost Analytics",
    "tags": ["llm", "cost"],
    "panels": [
      {
        "title": "Cumulative Cost (24h)",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(increase(llm_cost_total[24h]))",
            "legendFormat": "Total Cost"
          }
        ],
        "gridPos": {"x": 0, "y": 0, "w": 6, "h": 4},
        "fieldConfig": {
          "defaults": {
            "unit": "currencyUSD"
          }
        }
      },
      {
        "title": "Cost by Provider",
        "type": "piechart",
        "targets": [
          {
            "expr": "sum(increase(llm_cost_total[24h])) by (provider)",
            "legendFormat": "{{ provider }}"
          }
        ],
        "gridPos": {"x": 6, "y": 0, "w": 6, "h": 8}
      },
      {
        "title": "Cost by Model",
        "type": "piechart",
        "targets": [
          {
            "expr": "sum(increase(llm_cost_total[24h])) by (model)",
            "legendFormat": "{{ model }}"
          }
        ],
        "gridPos": {"x": 12, "y": 0, "w": 6, "h": 8}
      },
      {
        "title": "Cost Trend",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_cost_total[1h]))",
            "legendFormat": "Cost Rate ($/hour)"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 24, "h": 8}
      },
      {
        "title": "Cost per Request Distribution",
        "type": "histogram",
        "targets": [
          {
            "expr": "sum(increase(llm_cost_per_request_bucket[1h])) by (le)",
            "format": "heatmap"
          }
        ],
        "gridPos": {"x": 0, "y": 16, "w": 12, "h": 8}
      },
      {
        "title": "Top Expensive Requests",
        "type": "table",
        "targets": [
          {
            "expr": "topk(10, llm_cost_per_request)",
            "format": "table"
          }
        ],
        "gridPos": {"x": 12, "y": 16, "w": 12, "h": 8}
      }
    ]
  }
}
```

#### 4. Error Analysis Dashboard

```json
{
  "dashboard": {
    "title": "LLM Simulator - Error Analysis",
    "tags": ["llm", "errors"],
    "panels": [
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_requests_errors_total[5m])) by (error_type)",
            "legendFormat": "{{ error_type }}"
          }
        ],
        "gridPos": {"x": 0, "y": 0, "w": 12, "h": 8}
      },
      {
        "title": "Errors by Provider",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_requests_errors_total[5m])) by (provider)",
            "legendFormat": "{{ provider }}"
          }
        ],
        "gridPos": {"x": 12, "y": 0, "w": 12, "h": 8}
      },
      {
        "title": "Rate Limit Errors",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_errors_rate_limit[5m])) by (provider)",
            "legendFormat": "{{ provider }}"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 12, "h": 8}
      },
      {
        "title": "Timeout Errors",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(llm_errors_timeout[5m])) by (provider)",
            "legendFormat": "{{ provider }}"
          }
        ],
        "gridPos": {"x": 12, "y": 8, "w": 12, "h": 8}
      },
      {
        "title": "Error Logs",
        "type": "logs",
        "targets": [
          {
            "expr": "{service=\"llm-simulator\"} | json | level=\"error\"",
            "datasource": "Loki"
          }
        ],
        "gridPos": {"x": 0, "y": 16, "w": 24, "h": 8}
      }
    ]
  }
}
```

### Dashboard Access Control

| Dashboard | Audience | Access Level | Update Frequency |
|-----------|----------|-------------|------------------|
| Executive Overview | Leadership, Product | View | 30s |
| Performance Deep Dive | Engineering, SRE | View + Edit | 10s |
| Cost Analytics | Finance, Product, Eng | View | 1m |
| Error Analysis | Engineering, SRE | View + Edit | 10s |
| Provider Comparison | Engineering | View | 1m |
| SLO Dashboard | SRE, Product | View | 30s |

---

## SLO/SLI Definitions

### Service Level Objectives

#### Availability SLO

| Metric | Target | Measurement Window | Error Budget |
|--------|--------|-------------------|--------------|
| Service Availability | 99.9% | 30 days | 43 minutes |
| API Success Rate | 99.5% | 7 days | 50.4 minutes |

**SLI:** `(successful_requests / total_requests) * 100`

```promql
# Availability SLI
sum(rate(llm_requests_total{status="success"}[30d]))
/
sum(rate(llm_requests_total[30d]))
```

#### Latency SLO

| Metric | Target | Measurement Window |
|--------|--------|-------------------|
| P95 Request Latency | < 5s | 7 days |
| P99 Request Latency | < 10s | 7 days |
| P95 TTFT | < 2s | 7 days |

**SLI:** `P95(request_duration)`

```promql
# Latency SLI
histogram_quantile(0.95,
  sum(rate(llm_requests_duration_bucket[7d])) by (le)
)
```

#### Quality SLO

| Metric | Target | Measurement Window |
|--------|--------|-------------------|
| Cache Hit Rate | > 60% | 24 hours |
| Successful Completion Rate | > 99% | 7 days |

**SLI:** `cache_hits / (cache_hits + cache_misses)`

```promql
# Cache Hit Rate SLI
sum(rate(llm_cache_hits[24h]))
/
(sum(rate(llm_cache_hits[24h])) + sum(rate(llm_cache_misses[24h])))
```

### Error Budget Policy

```yaml
# Error budget policy
error_budget:
  availability_slo: 99.9%
  measurement_window: 30d

  actions:
    # 100% budget remaining
    - threshold: 1.0
      action: "Normal operations. Continue feature development."

    # 75% budget consumed
    - threshold: 0.25
      action: "Warning. Review recent changes. Increase monitoring."

    # 90% budget consumed
    - threshold: 0.10
      action: "Alert. Freeze non-critical deployments. Focus on reliability."

    # 100% budget consumed
    - threshold: 0.0
      action: "Critical. Feature freeze. All hands on reliability."
```

### SLO Dashboard Configuration

```json
{
  "dashboard": {
    "title": "LLM Simulator - SLO Dashboard",
    "tags": ["llm", "slo"],
    "panels": [
      {
        "title": "Availability SLO (30d)",
        "type": "gauge",
        "targets": [
          {
            "expr": "(sum(rate(llm_requests_total{status=\"success\"}[30d])) / sum(rate(llm_requests_total[30d]))) * 100",
            "legendFormat": "Availability %"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 99,
            "max": 100,
            "thresholds": {
              "mode": "absolute",
              "steps": [
                {"value": 99, "color": "red"},
                {"value": 99.5, "color": "yellow"},
                {"value": 99.9, "color": "green"}
              ]
            }
          }
        },
        "gridPos": {"x": 0, "y": 0, "w": 6, "h": 8}
      },
      {
        "title": "Error Budget Remaining",
        "type": "gauge",
        "targets": [
          {
            "expr": "1 - ((1 - (sum(rate(llm_requests_total{status=\"success\"}[30d])) / sum(rate(llm_requests_total[30d])))) / (1 - 0.999))",
            "legendFormat": "Budget %"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percentunit",
            "min": 0,
            "max": 1,
            "thresholds": {
              "mode": "absolute",
              "steps": [
                {"value": 0, "color": "red"},
                {"value": 0.25, "color": "yellow"},
                {"value": 0.5, "color": "green"}
              ]
            }
          }
        },
        "gridPos": {"x": 6, "y": 0, "w": 6, "h": 8}
      },
      {
        "title": "Latency SLO Compliance",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[7d])) by (le))",
            "legendFormat": "P95 Latency"
          }
        ],
        "gridPos": {"x": 12, "y": 0, "w": 12, "h": 8},
        "thresholds": [
          {
            "value": 5000,
            "colorMode": "critical",
            "op": "gt",
            "line": true,
            "lineColor": "red"
          }
        ]
      },
      {
        "title": "SLO Burn Rate (1h)",
        "type": "stat",
        "targets": [
          {
            "expr": "(1 - (sum(rate(llm_requests_total{status=\"success\"}[1h])) / sum(rate(llm_requests_total[1h])))) / (1 - 0.999)",
            "legendFormat": "Burn Rate"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 6, "h": 4}
      }
    ]
  }
}
```

---

## Cardinality Management

### Cardinality Estimation

```
Total Cardinality = Product of label values across all dimensions

Example for llm_requests_total:
- providers: 4 (openai, anthropic, cohere, google)
- models: 5 per provider = 20 total
- operations: 3 (completion, chat, embedding)
- status: 2 (success, error)

Total: 4 × 5 × 3 × 2 = 120 time series
```

### Cardinality Control Strategy

#### 1. Label Value Constraints

```typescript
// metrics/cardinality-control.ts

const ALLOWED_PROVIDERS = ['openai', 'anthropic', 'cohere', 'google'];
const ALLOWED_OPERATIONS = ['completion', 'chat', 'embedding', 'streaming'];
const ALLOWED_STATUSES = ['success', 'error'];

export function normalizeProvider(provider: string): string {
  const normalized = provider.toLowerCase();
  return ALLOWED_PROVIDERS.includes(normalized) ? normalized : 'other';
}

export function normalizeModel(provider: string, model: string): string {
  // Normalize to base model, strip version suffixes
  const baseModel = model.split('-')[0];
  return `${provider}/${baseModel}`;
}

export function normalizeErrorType(error: Error): string {
  // Map error types to predefined categories
  const errorMap: Record<string, string> = {
    'RateLimitError': 'rate_limit',
    'TimeoutError': 'timeout',
    'AuthenticationError': 'auth',
    'InvalidRequestError': 'invalid_request',
  };

  return errorMap[error.name] || 'unknown';
}
```

#### 2. High-Cardinality Label Handling

**Do NOT use as labels:**
- User IDs
- Request IDs
- Trace IDs
- Timestamps
- Free-form text
- IP addresses (unless aggregated)

**Use trace attributes instead:**
```typescript
// Good: Use as span attribute
span.setAttribute('user.id', userId);

// Bad: Use as metric label
requestsTotal.add(1, { 'user_id': userId }); // DON'T DO THIS
```

#### 3. Metric Retention Policy

| Metric Type | Raw Resolution | Downsampled | Retention |
|-------------|---------------|-------------|-----------|
| Counter | 15s | 5m (30d), 1h (1y) | 90 days |
| Histogram | 15s | 5m (30d), 1h (1y) | 90 days |
| Gauge | 15s | 5m (30d), 1h (1y) | 90 days |

#### 4. Cardinality Monitoring

```promql
# Monitor cardinality of each metric
count({__name__=~"llm_.*"}) by (__name__)

# Alert on high cardinality
count({__name__="llm_requests_total"}) > 1000
```

#### 5. Prometheus Configuration

```yaml
# prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

# Cardinality limits
storage:
  tsdb:
    max-block-duration: 2h
    retention.time: 90d
    retention.size: 100GB

# Recording rules for downsampling
rule_files:
  - /etc/prometheus/recording-rules.yaml

# Example recording rule
recording_rules:
  - record: llm:requests:rate5m
    expr: sum(rate(llm_requests_total[5m])) by (provider, model)

  - record: llm:latency:p95_5m
    expr: histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le, provider))
```

### Cardinality Budget

| Metric Family | Max Cardinality | Current | Headroom |
|---------------|----------------|---------|----------|
| llm_requests_* | 500 | 120 | 76% |
| llm_tokens_* | 200 | 48 | 76% |
| llm_latency_* | 300 | 96 | 68% |
| llm_cost_* | 200 | 48 | 76% |
| llm_errors_* | 400 | 96 | 76% |
| **Total** | **1,600** | **408** | **75%** |

---

## Platform Integrations

### Supported Observability Platforms

#### 1. Grafana Cloud

```typescript
// config/grafana-cloud.ts

export const grafanaCloudConfig = {
  metrics: {
    endpoint: process.env.GRAFANA_CLOUD_PROMETHEUS_URL,
    username: process.env.GRAFANA_CLOUD_INSTANCE_ID,
    password: process.env.GRAFANA_CLOUD_API_KEY,
  },

  traces: {
    endpoint: process.env.GRAFANA_CLOUD_TEMPO_URL,
    auth: `Bearer ${process.env.GRAFANA_CLOUD_API_KEY}`,
  },

  logs: {
    endpoint: process.env.GRAFANA_CLOUD_LOKI_URL,
    username: process.env.GRAFANA_CLOUD_INSTANCE_ID,
    password: process.env.GRAFANA_CLOUD_API_KEY,
  },
};
```

#### 2. Datadog

```typescript
// config/datadog.ts

import { DatadogExporter } from '@opentelemetry/exporter-datadog';

export const datadogConfig = {
  traces: new DatadogExporter({
    serviceName: 'llm-simulator',
    agentUrl: process.env.DD_AGENT_URL || 'http://localhost:8126',
    tags: {
      env: process.env.NODE_ENV,
      version: process.env.SERVICE_VERSION,
    },
  }),

  metrics: {
    apiKey: process.env.DD_API_KEY,
    site: process.env.DD_SITE || 'datadoghq.com',
    service: 'llm-simulator',
  },
};
```

#### 3. New Relic

```typescript
// config/newrelic.ts

export const newRelicConfig = {
  app_name: ['LLM Simulator'],
  license_key: process.env.NEW_RELIC_LICENSE_KEY,
  logging: {
    level: 'info',
  },

  distributed_tracing: {
    enabled: true,
  },

  transaction_tracer: {
    enabled: true,
    transaction_threshold: 0.5,
  },
};
```

#### 4. Self-Hosted Stack

```yaml
# docker-compose.yaml

version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.retention.time=90d'

  jaeger:
    image: jaegertracing/all-in-one:latest
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    ports:
      - "16686:16686"  # UI
      - "4318:4318"    # OTLP HTTP

  loki:
    image: grafana/loki:latest
    volumes:
      - ./config/loki.yaml:/etc/loki/config.yaml
      - loki-data:/loki
    ports:
      - "3100:3100"

  grafana:
    image: grafana/grafana:latest
    volumes:
      - grafana-data:/var/lib/grafana
      - ./config/grafana/provisioning:/etc/grafana/provisioning
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin

  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    volumes:
      - ./config/otel-collector.yaml:/etc/otel/config.yaml
    command: ["--config=/etc/otel/config.yaml"]
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP

volumes:
  prometheus-data:
  loki-data:
  grafana-data:
```

### OTLP Collector Configuration

```yaml
# config/otel-collector.yaml

receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 10s
    send_batch_size: 1024

  memory_limiter:
    check_interval: 1s
    limit_mib: 512

  resource:
    attributes:
      - key: service.name
        value: llm-simulator
        action: upsert

  attributes:
    actions:
      - key: environment
        value: ${env:ENVIRONMENT}
        action: insert

exporters:
  # Prometheus
  prometheus:
    endpoint: "0.0.0.0:9464"

  # Jaeger
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  # Loki
  loki:
    endpoint: http://loki:3100/loki/api/v1/push

  # Datadog (optional)
  datadog:
    api:
      key: ${env:DD_API_KEY}
      site: datadoghq.com

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource, attributes]
      exporters: [jaeger, datadog]

    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [prometheus, datadog]

    logs:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource, attributes]
      exporters: [loki, datadog]
```

---

## Operational Runbooks

### Runbook: High Error Rate

**Alert:** `HighErrorRate`
**Severity:** Critical (P0)
**SLO Impact:** Availability

#### Diagnosis Steps

1. **Check error distribution**
   ```promql
   sum(rate(llm_requests_errors_total[5m])) by (error_type, provider)
   ```

2. **Review recent traces**
   ```traceql
   { status = error } | duration > 100ms
   ```

3. **Check provider status**
   - Visit provider status pages
   - Review provider-specific error rates

4. **Review recent deployments**
   ```bash
   kubectl rollout history deployment/llm-simulator
   ```

#### Resolution Steps

1. **If provider issue:**
   - Enable fallback provider
   - Increase retry backoff
   - Contact provider support

2. **If application issue:**
   - Review recent code changes
   - Check application logs for stack traces
   - Consider rollback if recent deployment

3. **If infrastructure issue:**
   - Check resource utilization
   - Scale horizontally if needed
   - Review network connectivity

#### Recovery Validation

```promql
# Confirm error rate is below threshold
(sum(rate(llm_requests_errors_total[5m])) / sum(rate(llm_requests_total[5m]))) < 0.01
```

### Runbook: Latency SLO Violation

**Alert:** `LatencySLOViolation`
**Severity:** High (P1)
**SLO Impact:** Performance

#### Diagnosis Steps

1. **Check latency breakdown**
   ```promql
   histogram_quantile(0.99, sum(rate(llm_requests_duration_bucket[5m])) by (le, provider, model))
   ```

2. **Analyze TTFT**
   ```promql
   histogram_quantile(0.95, sum(rate(llm_latency_ttft_bucket[5m])) by (le, provider))
   ```

3. **Check provider API latency**
   ```promql
   histogram_quantile(0.95, sum(rate(llm_latency_provider_api_bucket[5m])) by (le, provider))
   ```

4. **Review queue depth**
   ```promql
   llm_queue_depth
   ```

#### Resolution Steps

1. **If provider latency:**
   - Switch to alternative provider
   - Reduce max_tokens if appropriate
   - Implement request prioritization

2. **If queue backup:**
   - Scale worker processes
   - Increase concurrency limits
   - Implement request shedding

3. **If resource constrained:**
   - Scale infrastructure
   - Optimize application code
   - Review memory/CPU usage

### Runbook: Cost Spike

**Alert:** `CostSpike`
**Severity:** High (P1)
**SLO Impact:** Budget

#### Diagnosis Steps

1. **Identify cost source**
   ```promql
   topk(10, sum(rate(llm_cost_total[5m])) by (provider, model))
   ```

2. **Check request patterns**
   ```promql
   sum(rate(llm_requests_total[5m])) by (provider, model)
   ```

3. **Review token usage**
   ```promql
   sum(rate(llm_tokens_total[5m])) by (provider, model, token_type)
   ```

4. **Check for abuse**
   - Review request sources
   - Check for unusual patterns
   - Analyze trace samples

#### Resolution Steps

1. **Immediate mitigation:**
   - Enable cost limits
   - Reduce max_tokens
   - Implement rate limiting

2. **Investigation:**
   - Identify cost driver
   - Review recent changes
   - Check for misconfiguration

3. **Long-term:**
   - Optimize prompts
   - Implement caching
   - Switch to cost-effective models

---

## Appendix

### Environment Variables

```bash
# OpenTelemetry
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4318/v1/traces
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://localhost:4318/v1/metrics
OTEL_AUTH_TOKEN=<token>
TRACE_SAMPLING_RATE=0.1

# Service
SERVICE_NAME=llm-simulator
SERVICE_VERSION=1.0.0
NODE_ENV=production

# Logging
LOG_LEVEL=info

# Metrics
PROMETHEUS_PORT=9464

# Platform-specific
DD_API_KEY=<datadog-api-key>
DD_SITE=datadoghq.com
GRAFANA_CLOUD_API_KEY=<grafana-api-key>
NEW_RELIC_LICENSE_KEY=<newrelic-key>
```

### Quick Reference

#### Common PromQL Queries

```promql
# Request rate
sum(rate(llm_requests_total[5m]))

# Error rate
sum(rate(llm_requests_errors_total[5m])) / sum(rate(llm_requests_total[5m]))

# P95 latency
histogram_quantile(0.95, sum(rate(llm_requests_duration_bucket[5m])) by (le))

# Token usage rate
sum(rate(llm_tokens_total[5m]))

# Cost per hour
sum(rate(llm_cost_total[1h])) * 3600
```

#### Common TraceQL Queries

```traceql
# Slow requests
{ duration > 5s }

# Errors
{ status = error }

# Specific provider
{ span.llm.provider = "openai" }

# High token usage
{ span.llm.response.tokens.total > 10000 }
```

#### Common LogQL Queries

```logql
# All errors
{service="llm-simulator"} | json | level="error"

# Specific provider errors
{service="llm-simulator"} | json | llm_provider="openai" | level="error"

# High latency requests
{service="llm-simulator"} | json | llm_duration_ms > 5000

# Cost analysis
{service="llm-simulator"} | json | llm_cost_usd > 0.1
```

### Resource Requirements

| Component | CPU | Memory | Storage | Network |
|-----------|-----|--------|---------|---------|
| Application | 2-4 cores | 4-8 GB | 10 GB | 1 Gbps |
| OTEL Collector | 2 cores | 2 GB | 5 GB | 1 Gbps |
| Prometheus | 4 cores | 16 GB | 500 GB | 1 Gbps |
| Jaeger | 2 cores | 4 GB | 100 GB | 1 Gbps |
| Loki | 2 cores | 4 GB | 200 GB | 1 Gbps |
| Grafana | 2 cores | 2 GB | 10 GB | 100 Mbps |

### Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-11-26 | Initial release | Platform Engineering |

---

**Document Status:** Production Ready
**Next Review:** 2026-02-26
**Owner:** Platform Engineering Team
**Contact:** platform-eng@example.com

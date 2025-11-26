# Observability Implementation Guide

**Version:** 1.0
**Last Updated:** 2025-11-26
**Audience:** Platform Engineers, SREs, Backend Developers

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Instrumentation Examples](#instrumentation-examples)
5. [Testing Observability](#testing-observability)
6. [Production Deployment](#production-deployment)
7. [Troubleshooting](#troubleshooting)

---

## Quick Start

### 5-Minute Setup (Development)

```bash
# 1. Install dependencies
npm install --save \
  @opentelemetry/sdk-node \
  @opentelemetry/auto-instrumentations-node \
  @opentelemetry/exporter-trace-otlp-proto \
  @opentelemetry/exporter-metrics-otlp-proto \
  @opentelemetry/api

# 2. Start observability stack
docker-compose -f docker-compose.observability.yml up -d

# 3. Set environment variables
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
export OTEL_SERVICE_NAME=llm-simulator
export OTEL_TRACES_SAMPLER=always_on

# 4. Initialize in your app
import { initializeObservability } from './config/observability';
initializeObservability();

# 5. Access dashboards
# Grafana: http://localhost:3000 (admin/admin)
# Jaeger: http://localhost:16686
# Prometheus: http://localhost:9090
```

---

## Installation

### Node.js Dependencies

```json
{
  "dependencies": {
    "@opentelemetry/sdk-node": "^0.45.0",
    "@opentelemetry/api": "^1.7.0",
    "@opentelemetry/auto-instrumentations-node": "^0.40.0",
    "@opentelemetry/exporter-trace-otlp-proto": "^0.45.0",
    "@opentelemetry/exporter-metrics-otlp-proto": "^0.45.0",
    "@opentelemetry/resources": "^1.18.0",
    "@opentelemetry/semantic-conventions": "^1.18.0",
    "@opentelemetry/sdk-metrics": "^1.18.0",
    "winston": "^3.11.0"
  }
}
```

### Infrastructure Components

```bash
# Using Docker Compose
docker-compose -f docker-compose.observability.yml up -d

# Or individual containers
docker run -d --name prometheus -p 9090:9090 prom/prometheus
docker run -d --name jaeger -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one
docker run -d --name loki -p 3100:3100 grafana/loki
docker run -d --name grafana -p 3000:3000 grafana/grafana
```

---

## Configuration

### Project Structure

```
llm-simulator/
├── src/
│   ├── config/
│   │   └── observability.ts          # Main OTEL config
│   ├── instrumentation/
│   │   ├── llm-tracer.ts             # LLM-specific tracing
│   │   └── custom-spans.ts           # Custom span utilities
│   ├── metrics/
│   │   ├── request-metrics.ts        # Request counters/histograms
│   │   ├── token-metrics.ts          # Token tracking
│   │   ├── latency-metrics.ts        # Latency measurements
│   │   ├── error-metrics.ts          # Error tracking
│   │   └── cost-metrics.ts           # Cost monitoring
│   └── logging/
│       ├── logger.ts                 # Structured logger
│       └── schema.ts                 # Log schema definitions
├── config/
│   ├── prometheus.yml                # Prometheus config
│   ├── otel-collector.yaml           # OTEL Collector config
│   ├── loki.yaml                     # Loki config
│   └── alerts/
│       └── llm-simulator-alerts.yaml # Alert rules
└── docker-compose.observability.yml  # Local dev stack
```

### Environment Variables

```bash
# config/.env.development

# Service identification
SERVICE_NAME=llm-simulator
SERVICE_VERSION=1.0.0
NODE_ENV=development
HOSTNAME=localhost

# OpenTelemetry endpoints
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4318/v1/traces
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://localhost:4318/v1/metrics
OTEL_AUTH_TOKEN=

# Sampling
TRACE_SAMPLING_RATE=1.0  # 100% in dev

# Logging
LOG_LEVEL=debug

# Metrics
PROMETHEUS_PORT=9464

# Optional: Cloud platforms
DD_API_KEY=
DD_SITE=datadoghq.com
GRAFANA_CLOUD_API_KEY=
NEW_RELIC_LICENSE_KEY=
```

```bash
# config/.env.production

# Service identification
SERVICE_NAME=llm-simulator
SERVICE_VERSION=${CI_COMMIT_TAG}
NODE_ENV=production
HOSTNAME=${HOSTNAME}

# OpenTelemetry endpoints
OTEL_EXPORTER_OTLP_ENDPOINT=https://otel-collector.production.internal:4318
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=https://otel-collector.production.internal:4318/v1/traces
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=https://otel-collector.production.internal:4318/v1/metrics
OTEL_AUTH_TOKEN=${OTEL_AUTH_TOKEN}

# Sampling
TRACE_SAMPLING_RATE=0.1  # 10% in production

# Logging
LOG_LEVEL=info

# Metrics
PROMETHEUS_PORT=9464
```

### Docker Compose Configuration

```yaml
# docker-compose.observability.yml

version: '3.8'

services:
  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector-contrib:0.90.0
    container_name: otel-collector
    command: ["--config=/etc/otel/config.yaml"]
    volumes:
      - ./config/otel-collector.yaml:/etc/otel/config.yaml
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
      - "9464:9464"   # Prometheus exporter
    networks:
      - observability

  # Prometheus
  prometheus:
    image: prom/prometheus:v2.48.0
    container_name: prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=90d'
      - '--web.enable-lifecycle'
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml
      - ./config/alerts:/etc/prometheus/alerts
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"
    networks:
      - observability
    depends_on:
      - otel-collector

  # Jaeger (all-in-one)
  jaeger:
    image: jaegertracing/all-in-one:1.52
    container_name: jaeger
    environment:
      - COLLECTOR_OTLP_ENABLED=true
      - SPAN_STORAGE_TYPE=badger
      - BADGER_EPHEMERAL=false
      - BADGER_DIRECTORY_VALUE=/badger/data
      - BADGER_DIRECTORY_KEY=/badger/key
    volumes:
      - jaeger-data:/badger
    ports:
      - "16686:16686"  # Jaeger UI
      - "14250:14250"  # gRPC
    networks:
      - observability

  # Loki
  loki:
    image: grafana/loki:2.9.3
    container_name: loki
    command: -config.file=/etc/loki/config.yaml
    volumes:
      - ./config/loki.yaml:/etc/loki/config.yaml
      - loki-data:/loki
    ports:
      - "3100:3100"
    networks:
      - observability

  # Grafana
  grafana:
    image: grafana/grafana:10.2.2
    container_name: grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_SERVER_ROOT_URL=http://localhost:3000
      - GF_INSTALL_PLUGINS=grafana-piechart-panel
    volumes:
      - ./config/grafana/provisioning:/etc/grafana/provisioning
      - ./config/grafana/dashboards:/var/lib/grafana/dashboards
      - grafana-data:/var/lib/grafana
    ports:
      - "3000:3000"
    networks:
      - observability
    depends_on:
      - prometheus
      - loki
      - jaeger

  # Alertmanager
  alertmanager:
    image: prom/alertmanager:v0.26.0
    container_name: alertmanager
    command:
      - '--config.file=/etc/alertmanager/config.yml'
      - '--storage.path=/alertmanager'
    volumes:
      - ./config/alertmanager.yaml:/etc/alertmanager/config.yml
      - alertmanager-data:/alertmanager
    ports:
      - "9093:9093"
    networks:
      - observability

networks:
  observability:
    driver: bridge

volumes:
  prometheus-data:
  jaeger-data:
  loki-data:
  grafana-data:
  alertmanager-data:
```

---

## Instrumentation Examples

### Basic Application Setup

```typescript
// src/index.ts

import { initializeObservability } from './config/observability';
import { logger } from './logging/logger';

// Initialize observability FIRST, before other imports
const sdk = initializeObservability();

// Now import and start your app
import { startServer } from './server';

async function main() {
  try {
    logger.info('Starting LLM Simulator', {
      version: process.env.SERVICE_VERSION,
      environment: process.env.NODE_ENV,
    });

    await startServer();

    logger.info('Server started successfully');
  } catch (error) {
    logger.error('Failed to start server', error);
    process.exit(1);
  }
}

main();
```

### HTTP Request Instrumentation

```typescript
// src/middleware/observability.ts

import { trace, context, SpanStatusCode } from '@opentelemetry/api';
import { Request, Response, NextFunction } from 'express';
import { logger } from '../logging/logger';
import { requestsTotal, requestDuration } from '../metrics/request-metrics';

const tracer = trace.getTracer('llm-simulator');

export function observabilityMiddleware(req: Request, res: Response, next: NextFunction) {
  const startTime = Date.now();

  // Start span for this request
  const span = tracer.startSpan(`HTTP ${req.method} ${req.path}`, {
    kind: trace.SpanKind.SERVER,
    attributes: {
      'http.method': req.method,
      'http.url': req.url,
      'http.target': req.path,
      'http.user_agent': req.headers['user-agent'] || 'unknown',
      'http.client_ip': req.ip,
    },
  });

  // Set span as active
  const ctx = trace.setSpan(context.active(), span);

  // Log request
  logger.info('Request received', {
    request: {
      id: req.id,
      method: req.method,
      path: req.path,
      user_agent: req.headers['user-agent'],
      ip_address: req.ip,
    },
  });

  // Capture response
  res.on('finish', () => {
    const duration = Date.now() - startTime;

    // Add response attributes to span
    span.setAttributes({
      'http.status_code': res.statusCode,
      'http.response_content_length': res.get('content-length') || 0,
    });

    // Set span status
    if (res.statusCode >= 500) {
      span.setStatus({ code: SpanStatusCode.ERROR });
    } else {
      span.setStatus({ code: SpanStatusCode.OK });
    }

    // Record metrics
    requestsTotal.add(1, {
      method: req.method,
      path: req.route?.path || req.path,
      status: res.statusCode >= 500 ? 'error' : 'success',
    });

    requestDuration.record(duration, {
      method: req.method,
      path: req.route?.path || req.path,
    });

    // Log response
    logger.info('Request completed', {
      request: {
        id: req.id,
        method: req.method,
        path: req.path,
      },
      response: {
        status: res.statusCode,
        duration_ms: duration,
      },
    });

    // End span
    span.end();
  });

  // Continue with active context
  context.with(ctx, () => next());
}
```

### LLM Completion Instrumentation

```typescript
// src/services/llm-service.ts

import { LLMInstrumentation } from '../instrumentation/llm-tracer';
import { logger } from '../logging/logger';
import {
  requestsTotal,
  requestDuration,
  requestErrors,
} from '../metrics/request-metrics';
import {
  tokensTotal,
  tokensPerRequest,
} from '../metrics/token-metrics';
import {
  timeToFirstToken,
  endToEndLatency,
  providerApiLatency,
} from '../metrics/latency-metrics';
import {
  cumulativeCost,
  costPerRequest,
} from '../metrics/cost-metrics';

export class LLMService {
  async complete(provider: string, model: string, prompt: string, options: any) {
    const startTime = Date.now();

    // Log request
    logger.logLLMRequest(provider, model, {
      operation: 'completion',
      prompt_length: prompt.length,
      options,
    });

    try {
      // Traced completion
      const response = await LLMInstrumentation.traceCompletion(
        provider,
        model,
        async () => {
          // Call provider API
          const apiStartTime = Date.now();
          const result = await this.callProviderAPI(provider, model, prompt, options);
          const apiDuration = Date.now() - apiStartTime;

          // Record provider API latency
          providerApiLatency.record(apiDuration, {
            provider,
            endpoint: '/v1/completions',
          });

          return result;
        }
      );

      // Calculate metrics
      const duration = Date.now() - startTime;
      const cost = this.calculateCost(provider, model, response.usage);

      // Record all metrics
      requestsTotal.add(1, {
        provider,
        model,
        operation: 'completion',
        status: 'success',
      });

      requestDuration.record(duration, {
        provider,
        model,
        operation: 'completion',
      });

      endToEndLatency.record(duration, {
        provider,
        model,
        operation: 'completion',
      });

      tokensTotal.add(response.usage.total_tokens, {
        provider,
        model,
        token_type: 'total',
      });

      tokensTotal.add(response.usage.prompt_tokens, {
        provider,
        model,
        token_type: 'prompt',
      });

      tokensTotal.add(response.usage.completion_tokens, {
        provider,
        model,
        token_type: 'completion',
      });

      tokensPerRequest.record(response.usage.total_tokens, {
        provider,
        model,
      });

      cumulativeCost.add(cost, {
        provider,
        model,
        cost_component: 'total',
      });

      costPerRequest.record(cost, {
        provider,
        model,
      });

      // Log success
      logger.logLLMResponse(
        provider,
        model,
        response.usage,
        duration,
        cost
      );

      return response;
    } catch (error) {
      // Record error metrics
      requestsTotal.add(1, {
        provider,
        model,
        operation: 'completion',
        status: 'error',
      });

      requestErrors.add(1, {
        provider,
        model,
        error_type: this.normalizeErrorType(error),
        error_code: error.status || '500',
      });

      // Log error
      logger.logLLMError(provider, model, error, {
        operation: 'completion',
        prompt_length: prompt.length,
      });

      throw error;
    }
  }

  async *streamCompletion(provider: string, model: string, prompt: string, options: any) {
    const startTime = Date.now();
    let firstTokenTime: number | null = null;
    let lastTokenTime = startTime;
    let tokenCount = 0;

    logger.logLLMRequest(provider, model, {
      operation: 'streaming',
      prompt_length: prompt.length,
    });

    try {
      const stream = await LLMInstrumentation.traceStreaming(
        provider,
        model,
        async function* () {
          yield* this.callProviderStreamingAPI(provider, model, prompt, options);
        }.bind(this)
      );

      for await (const chunk of stream) {
        const now = Date.now();

        if (firstTokenTime === null) {
          firstTokenTime = now;
          const ttft = firstTokenTime - startTime;

          // Record TTFT
          timeToFirstToken.record(ttft, { provider, model });

          logger.debug('First token received', {
            llm: { provider, model, ttft_ms: ttft },
          });
        } else {
          // Record inter-token latency
          const itl = now - lastTokenTime;
          // interTokenLatency.record(itl, { provider, model });
        }

        lastTokenTime = now;
        tokenCount++;
        yield chunk;
      }

      const totalDuration = Date.now() - startTime;
      const tokensPerSecond = tokenCount / (totalDuration / 1000);

      // Record metrics
      requestsTotal.add(1, {
        provider,
        model,
        operation: 'streaming',
        status: 'success',
      });

      endToEndLatency.record(totalDuration, {
        provider,
        model,
        operation: 'streaming',
      });

      logger.info('Streaming completed', {
        llm: {
          provider,
          model,
          operation: 'streaming',
          tokens: tokenCount,
          duration_ms: totalDuration,
          tokens_per_second: tokensPerSecond,
        },
      });
    } catch (error) {
      requestErrors.add(1, {
        provider,
        model,
        error_type: this.normalizeErrorType(error),
        error_code: error.status || '500',
      });

      logger.logLLMError(provider, model, error, {
        operation: 'streaming',
      });

      throw error;
    }
  }

  private async callProviderAPI(provider: string, model: string, prompt: string, options: any) {
    // Implementation...
  }

  private async *callProviderStreamingAPI(provider: string, model: string, prompt: string, options: any) {
    // Implementation...
  }

  private calculateCost(provider: string, model: string, usage: any): number {
    // Pricing per 1K tokens
    const pricing = {
      'openai/gpt-4': { prompt: 0.03, completion: 0.06 },
      'openai/gpt-3.5-turbo': { prompt: 0.0015, completion: 0.002 },
      'anthropic/claude-3-opus': { prompt: 0.015, completion: 0.075 },
      'anthropic/claude-3-sonnet': { prompt: 0.003, completion: 0.015 },
    };

    const key = `${provider}/${model}`;
    const price = pricing[key] || { prompt: 0.01, completion: 0.01 };

    const promptCost = (usage.prompt_tokens / 1000) * price.prompt;
    const completionCost = (usage.completion_tokens / 1000) * price.completion;

    return promptCost + completionCost;
  }

  private normalizeErrorType(error: any): string {
    const errorMap: Record<string, string> = {
      'RateLimitError': 'rate_limit',
      'TimeoutError': 'timeout',
      'AuthenticationError': 'auth',
      'InvalidRequestError': 'invalid_request',
    };

    return errorMap[error.name] || 'unknown';
  }
}
```

### Custom Business Metrics

```typescript
// src/metrics/business-metrics.ts

import { metrics } from '@opentelemetry/api';

const meter = metrics.getMeter('llm-simulator');

// User engagement metric
export const activeUsers = meter.createObservableGauge('llm.users.active', {
  description: 'Number of active users',
  unit: '1',
});

// Feature usage
export const featureUsage = meter.createCounter('llm.features.usage', {
  description: 'Feature usage counter',
  unit: '1',
});

// Conversation metrics
export const conversationLength = meter.createHistogram('llm.conversation.length', {
  description: 'Number of turns in conversation',
  unit: '1',
  advice: {
    explicitBucketBoundaries: [1, 5, 10, 20, 50, 100],
  },
});

// Track usage
export function trackFeatureUsage(feature: string, userId: string) {
  featureUsage.add(1, {
    feature,
    // Don't use userId as label (high cardinality)
    // Instead, use as span attribute in trace
  });
}

export function trackConversationEnd(turnCount: number, provider: string) {
  conversationLength.record(turnCount, {
    provider,
  });
}
```

---

## Testing Observability

### Unit Tests

```typescript
// tests/observability.test.ts

import { describe, it, expect, beforeEach, afterEach } from '@jest/globals';
import { trace } from '@opentelemetry/api';
import { NodeTracerProvider } from '@opentelemetry/sdk-trace-node';
import { InMemorySpanExporter } from '@opentelemetry/sdk-trace-base';
import { LLMInstrumentation } from '../src/instrumentation/llm-tracer';

describe('LLM Instrumentation', () => {
  let provider: NodeTracerProvider;
  let exporter: InMemorySpanExporter;

  beforeEach(() => {
    exporter = new InMemorySpanExporter();
    provider = new NodeTracerProvider();
    provider.addSpanProcessor(new SimpleSpanProcessor(exporter));
    provider.register();
  });

  afterEach(() => {
    provider.shutdown();
  });

  it('should create span for LLM completion', async () => {
    const mockOperation = async () => ({
      choices: [{ finish_reason: 'stop' }],
      usage: { prompt_tokens: 100, completion_tokens: 50, total_tokens: 150 },
      model: 'gpt-4',
    });

    await LLMInstrumentation.traceCompletion('openai', 'gpt-4', mockOperation);

    const spans = exporter.getFinishedSpans();
    expect(spans).toHaveLength(1);
    expect(spans[0].name).toBe('llm.completion');
    expect(spans[0].attributes['llm.provider']).toBe('openai');
    expect(spans[0].attributes['llm.model']).toBe('gpt-4');
    expect(spans[0].attributes['llm.response.tokens.total']).toBe(150);
  });

  it('should record error in span when operation fails', async () => {
    const mockOperation = async () => {
      throw new Error('Provider timeout');
    };

    await expect(
      LLMInstrumentation.traceCompletion('openai', 'gpt-4', mockOperation)
    ).rejects.toThrow('Provider timeout');

    const spans = exporter.getFinishedSpans();
    expect(spans[0].status.code).toBe(SpanStatusCode.ERROR);
    expect(spans[0].events).toContainEqual(
      expect.objectContaining({
        name: 'exception',
      })
    );
  });
});
```

### Integration Tests

```typescript
// tests/integration/observability.integration.test.ts

import { describe, it, expect } from '@jest/globals';
import request from 'supertest';
import { app } from '../src/server';

describe('Observability Integration', () => {
  it('should inject trace context in outgoing requests', async () => {
    const response = await request(app)
      .post('/v1/completions')
      .send({
        provider: 'openai',
        model: 'gpt-4',
        prompt: 'Hello, world!',
      });

    // Check for trace headers in response
    expect(response.headers).toHaveProperty('traceparent');
    expect(response.headers['traceparent']).toMatch(/^00-[0-9a-f]{32}-[0-9a-f]{16}-[0-9a-f]{2}$/);
  });

  it('should emit metrics for successful request', async () => {
    // Send request
    await request(app)
      .post('/v1/completions')
      .send({
        provider: 'openai',
        model: 'gpt-4',
        prompt: 'Test prompt',
      });

    // Query metrics endpoint
    const metricsResponse = await request(app).get('/metrics');
    const metricsText = metricsResponse.text;

    // Verify metrics exist
    expect(metricsText).toContain('llm_requests_total');
    expect(metricsText).toContain('llm_requests_duration');
    expect(metricsText).toContain('provider="openai"');
    expect(metricsText).toContain('model="gpt-4"');
  });
});
```

### Load Testing with Observability

```typescript
// tests/load/k6-observability.js

import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const hasTraceId = new Rate('has_trace_id');

export const options = {
  stages: [
    { duration: '1m', target: 10 },
    { duration: '3m', target: 50 },
    { duration: '1m', target: 0 },
  ],
  thresholds: {
    'errors': ['rate<0.01'], // < 1% errors
    'has_trace_id': ['rate>0.99'], // > 99% have trace IDs
    'http_req_duration': ['p(95)<5000'], // 95% < 5s
  },
};

export default function () {
  const payload = JSON.stringify({
    provider: 'openai',
    model: 'gpt-4',
    prompt: 'Test prompt for load testing',
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  const response = http.post('http://localhost:3000/v1/completions', payload, params);

  // Check response
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'has trace header': (r) => r.headers['Traceparent'] !== undefined,
  });

  errorRate.add(!success);
  hasTraceId.add(response.headers['Traceparent'] !== undefined);
}
```

---

## Production Deployment

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-simulator
  labels:
    app: llm-simulator
    version: v1
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-simulator
  template:
    metadata:
      labels:
        app: llm-simulator
        version: v1
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9464"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: llm-simulator
        image: llm-simulator:latest
        ports:
        - containerPort: 3000
          name: http
        - containerPort: 9464
          name: metrics
        env:
        - name: SERVICE_NAME
          value: "llm-simulator"
        - name: SERVICE_VERSION
          value: "1.0.0"
        - name: NODE_ENV
          value: "production"
        - name: OTEL_EXPORTER_OTLP_ENDPOINT
          value: "http://otel-collector:4318"
        - name: TRACE_SAMPLING_RATE
          value: "0.1"
        - name: LOG_LEVEL
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 5
```

### Service Monitor (Prometheus Operator)

```yaml
# k8s/servicemonitor.yaml

apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: llm-simulator
  labels:
    app: llm-simulator
spec:
  selector:
    matchLabels:
      app: llm-simulator
  endpoints:
  - port: metrics
    interval: 15s
    path: /metrics
    scrapeTimeout: 10s
```

### Grafana Dashboard ConfigMap

```yaml
# k8s/grafana-dashboard.yaml

apiVersion: v1
kind: ConfigMap
metadata:
  name: llm-simulator-dashboard
  labels:
    grafana_dashboard: "1"
data:
  llm-simulator-overview.json: |
    {
      "dashboard": {
        "title": "LLM Simulator - Production Overview",
        "uid": "llm-sim-prod",
        ...
      }
    }
```

---

## Troubleshooting

### No Traces Appearing

**Check:**
1. OTLP collector is running: `docker ps | grep otel`
2. Endpoint is correct: `echo $OTEL_EXPORTER_OTLP_ENDPOINT`
3. Sampling is enabled: `echo $TRACE_SAMPLING_RATE`
4. SDK initialized: Check logs for "Tracing initialized"

**Debug:**
```bash
# Test OTLP endpoint
curl -v http://localhost:4318/v1/traces

# Check collector logs
docker logs otel-collector

# Enable debug logging
export OTEL_LOG_LEVEL=debug
```

### High Cardinality Issues

**Symptoms:**
- Prometheus running out of memory
- Slow query performance
- High storage usage

**Solution:**
```promql
# Check cardinality
count({__name__=~"llm_.*"}) by (__name__)

# Find high-cardinality metrics
topk(10, count by (__name__, job)({__name__=~".+"}))
```

**Fix:**
- Remove high-cardinality labels
- Use recording rules
- Implement label value limits

### Missing Metrics

**Check:**
1. Metric is registered: `curl localhost:9464/metrics | grep llm_`
2. Metric has values: Check for `{} 0` (no labels/values)
3. Scrape is working: Check Prometheus targets

**Debug:**
```typescript
// Add debug logging
console.log('Recording metric:', metricName, labels, value);

// Check metric export
import { metrics } from '@opentelemetry/api';
const meterProvider = metrics.getMeterProvider();
console.log(meterProvider);
```

### Logs Not Structured

**Issue:** Logs appearing as plain text instead of JSON

**Solution:**
```typescript
// Ensure JSON format
const logger = winston.createLogger({
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.json()  // <- Must be JSON
  ),
  transports: [
    new winston.transports.Console({
      format: winston.format.json()  // <- Also here
    })
  ]
});
```

### Trace Context Not Propagating

**Issue:** Parent-child span relationships broken

**Solution:**
```typescript
import { context, trace } from '@opentelemetry/api';

// Always use context.with() for async operations
const span = tracer.startSpan('operation');
const ctx = trace.setSpan(context.active(), span);

await context.with(ctx, async () => {
  // Your async operation here
  await someAsyncFunction();
});

span.end();
```

---

## Checklist

### Pre-Production Checklist

- [ ] OpenTelemetry SDK initialized
- [ ] All critical paths instrumented
- [ ] Metrics exposed on /metrics endpoint
- [ ] Structured logging implemented
- [ ] Trace sampling configured
- [ ] SLO dashboards created
- [ ] Alert rules configured
- [ ] Runbooks documented
- [ ] Load testing completed
- [ ] Observability tested in staging
- [ ] On-call team trained
- [ ] Cost estimates reviewed

### Go-Live Checklist

- [ ] Collector endpoints configured
- [ ] Sampling rate set appropriately
- [ ] Log level set to INFO
- [ ] Dashboards accessible
- [ ] Alerts integrated with PagerDuty
- [ ] Baseline metrics captured
- [ ] SLOs defined and monitored
- [ ] Team has access to tools
- [ ] Documentation published
- [ ] Rollback plan ready

---

**Maintained by:** Platform Engineering
**Version:** 1.0
**Last Updated:** 2025-11-26

# Observability Architecture Diagrams

**Version:** 1.0
**Last Updated:** 2025-11-26

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Data Flow](#data-flow)
3. [Metrics Pipeline](#metrics-pipeline)
4. [Tracing Pipeline](#tracing-pipeline)
5. [Logging Pipeline](#logging-pipeline)
6. [Alert Flow](#alert-flow)
7. [Dashboard Architecture](#dashboard-architecture)
8. [Deployment Architecture](#deployment-architecture)

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                                │
│                                                                          │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
│  │   API Server   │  │   Simulator    │  │    Provider    │            │
│  │                │  │     Engine     │  │    Adapters    │            │
│  │  • REST API    │  │  • Routing     │  │  • OpenAI      │            │
│  │  • Validation  │  │  • Processing  │  │  • Anthropic   │            │
│  │  • Rate Limit  │  │  • Caching     │  │  • Cohere      │            │
│  └────────┬───────┘  └───────┬────────┘  └───────┬────────┘            │
│           │                   │                    │                     │
│           └───────────────────┴────────────────────┘                     │
│                               │                                          │
│  ┌────────────────────────────┴────────────────────────────┐            │
│  │           OpenTelemetry SDK Instrumentation              │            │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │            │
│  │  │   Tracer     │  │  Meter       │  │   Logger     │  │            │
│  │  │              │  │              │  │              │  │            │
│  │  │ • Spans      │  │ • Counters   │  │ • Structured │  │            │
│  │  │ • Context    │  │ • Histograms │  │ • Correlated │  │            │
│  │  │ • Attributes │  │ • Gauges     │  │ • JSON       │  │            │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │            │
│  │         │                  │                  │          │            │
│  │         └──────────────────┼──────────────────┘          │            │
│  └────────────────────────────┼─────────────────────────────┘            │
└─────────────────────────────┼─────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   OTLP Protocol   │
                    │   (HTTP/gRPC)     │
                    └─────────┬─────────┘
                              │
                ┌─────────────┴─────────────┐
                │  OpenTelemetry Collector  │
                │                           │
                │  ┌──────────────────┐     │
                │  │    Receivers     │     │
                │  │  • OTLP (gRPC)   │     │
                │  │  • OTLP (HTTP)   │     │
                │  │  • Prometheus    │     │
                │  └────────┬─────────┘     │
                │           │               │
                │  ┌────────┴─────────┐     │
                │  │   Processors     │     │
                │  │  • Batch         │     │
                │  │  • Sample        │     │
                │  │  • Filter        │     │
                │  │  • Transform     │     │
                │  └────────┬─────────┘     │
                │           │               │
                │  ┌────────┴─────────┐     │
                │  │    Exporters     │     │
                │  │  • Prometheus    │     │
                │  │  • Jaeger        │     │
                │  │  • Loki          │     │
                │  │  • Datadog       │     │
                │  └────────┬─────────┘     │
                └───────────┼───────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
         ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Trace Storage  │ │ Metric Storage  │ │   Log Storage   │
│                 │ │                 │ │                 │
│  ┌───────────┐  │ │  ┌───────────┐  │ │  ┌───────────┐  │
│  │  Jaeger   │  │ │  │Prometheus │  │ │  │   Loki    │  │
│  │           │  │ │  │           │  │ │  │           │  │
│  │ • Badger  │  │ │  │ • TSDB    │  │ │  │ • Chunks  │  │
│  │ • Cassandra│ │ │  │ • 90d     │  │ │  │ • 30d     │  │
│  └───────────┘  │ │  │           │  │ │  └───────────┘  │
│                 │ │  └───────────┘  │ │                 │
│  OR             │ │                 │ │  OR             │
│                 │ │  OR             │ │                 │
│  ┌───────────┐  │ │                 │ │  ┌───────────┐  │
│  │   Tempo   │  │ │  ┌───────────┐  │ │  │Elasticsrch│  │
│  │           │  │ │  │ VictoriaM │  │ │  │           │  │
│  │ • S3/GCS  │  │ │  │           │  │ │  │ • Indices │  │
│  │ • Parquet │  │ │  │ • Long-term│ │ │  │ • 90d     │  │
│  └───────────┘  │ │  └───────────┘  │ │  └───────────┘  │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                   ┌─────────┴─────────┐
                   │   Query Layer     │
                   │                   │
                   │  • TraceQL        │
                   │  • PromQL         │
                   │  • LogQL          │
                   └─────────┬─────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│    Grafana      │ │  Alertmanager   │ │   Custom Apps   │
│                 │ │                 │ │                 │
│  • Dashboards   │ │  • Rules        │ │  • API Access   │
│  • Explore      │ │  • Routing      │ │  • Integrations │
│  • Alerts       │ │  • Silencing    │ │  • Reports      │
│  • Teams        │ │  • Escalation   │ │  • Analytics    │
└─────────────────┘ └────────┬────────┘ └─────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   PagerDuty     │ │     Slack       │ │   OpsGenie      │
│                 │ │                 │ │                 │
│  • On-call      │ │  • Channels     │ │  • Schedules    │
│  • Escalation   │ │  • Threads      │ │  • Integrations │
│  • Incidents    │ │  • Notifications│ │  • Workflows    │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

---

## Data Flow

### Request to Observability Flow

```
User Request
     │
     ▼
┌──────────────────────────────────────┐
│ 1. API Gateway / Load Balancer      │
│    • Injects/propagates trace header │
│    • traceparent: 00-<trace>-<span>  │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 2. Application Handler               │
│    • Extract trace context           │
│    • Start root span                 │
│    • Record request start time       │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 3. Business Logic                    │
│    • Create child spans              │
│    • Set span attributes             │
│    • Record metrics                  │
│    • Log with correlation            │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 4. LLM Provider Call                 │
│    • Propagate context to external   │
│    • Record provider latency         │
│    • Track token usage               │
│    • Calculate cost                  │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 5. Response Processing               │
│    • End all spans                   │
│    • Record final metrics            │
│    • Log completion                  │
│    • Return to client                │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 6. OTLP Export (async)               │
│    • Batch telemetry data            │
│    • Send to collector               │
│    • Handle backpressure             │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│ 7. Collector Processing              │
│    • Apply sampling decisions        │
│    • Enrich with metadata            │
│    • Filter sensitive data           │
│    • Route to appropriate backend    │
└──────────────┬───────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
     ▼                   ▼
┌─────────┐         ┌─────────┐
│ Storage │         │ Alerts  │
└─────────┘         └─────────┘
```

---

## Metrics Pipeline

```
Application
     │
     │ Record Metrics
     ▼
┌──────────────────────────────────────┐
│   Metric Instruments                 │
│                                      │
│  Counter:   llm.requests.total       │
│  Histogram: llm.requests.duration    │
│  Gauge:     llm.queue.depth          │
└──────────────┬───────────────────────┘
               │
               │ Aggregate (in-memory)
               ▼
┌──────────────────────────────────────┐
│   Metric Reader                      │
│                                      │
│  • Periodic export (60s)             │
│  • Delta aggregation                 │
│  • Cardinality limits                │
└──────────────┬───────────────────────┘
               │
               │ OTLP/HTTP
               ▼
┌──────────────────────────────────────┐
│   OTLP Collector                     │
│                                      │
│  Processors:                         │
│  • Memory limiter                    │
│  • Batch (10s/1024 metrics)          │
│  • Filter (drop go_* metrics)        │
│  • Resource (add environment)        │
└──────────────┬───────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
     ▼                   ▼
┌─────────────┐    ┌──────────────┐
│ Prometheus  │    │  Remote Write│
│             │    │              │
│ • Scrape    │    │ • Grafana    │
│   OR        │    │   Cloud      │
│ • Push      │    │ • Datadog    │
│             │    │ • New Relic  │
│ • TSDB      │    │              │
│ • 90d       │    └──────────────┘
│ • Local     │
└──────┬──────┘
       │
       │ PromQL
       ▼
┌──────────────────────────────────────┐
│   Recording Rules                    │
│                                      │
│  • Pre-compute expensive queries     │
│  • Downsample for long-term          │
│  • SLO calculations                  │
│    → llm:requests:rate5m             │
│    → llm:latency:p95                 │
│    → llm:slo:availability:30d        │
└──────────────┬───────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
     ▼                   ▼
┌─────────────┐    ┌──────────────┐
│  Grafana    │    │ Alertmanager │
│  Dashboards │    │   Rules      │
└─────────────┘    └──────────────┘
```

---

## Tracing Pipeline

```
Application Request
     │
     ▼
┌──────────────────────────────────────┐
│   Trace Context                      │
│                                      │
│  traceparent: 00-<traceid>-<spanid>  │
│  tracestate: vendor=value            │
└──────────────┬───────────────────────┘
               │
               │ Propagate
               ▼
┌──────────────────────────────────────┐
│   Span Hierarchy                     │
│                                      │
│  Root Span: POST /v1/completions     │
│    ├─ Child: Validation              │
│    ├─ Child: LLM Completion          │
│    │   ├─ Child: Provider Call       │
│    │   └─ Child: Response Parse      │
│    └─ Child: Cache Update            │
└──────────────┬───────────────────────┘
               │
               │ Span.End()
               ▼
┌──────────────────────────────────────┐
│   Span Processor                     │
│                                      │
│  • BatchSpanProcessor                │
│  • Max queue: 2048                   │
│  • Export interval: 5s               │
│  • Max batch: 512                    │
└──────────────┬───────────────────────┘
               │
               │ OTLP/gRPC
               ▼
┌──────────────────────────────────────┐
│   OTLP Collector                     │
│                                      │
│  Span Processor:                     │
│  • Add resource attributes           │
│  • Redact sensitive data             │
│  • Add K8s metadata                  │
│                                      │
│  Tail Sampling:                      │
│  • Always: errors                    │
│  • Always: latency > 5s              │
│  • Probabilistic: 10%                │
└──────────────┬───────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
     ▼                   ▼
┌─────────────┐    ┌──────────────┐
│   Jaeger    │    │    Tempo     │
│             │    │              │
│ • Badger DB │    │ • Object     │
│   OR        │    │   Storage    │
│ • Cassandra │    │ • S3/GCS     │
│             │    │              │
│ • 7d traces │    │ • 30d traces │
└──────┬──────┘    └──────┬───────┘
       │                  │
       │ TraceQL / Search │
       ▼                  ▼
┌──────────────────────────────────────┐
│   Visualization                      │
│                                      │
│  • Trace timeline                    │
│  • Service graph                     │
│  • Span details                      │
│  • Error highlighting                │
│  • Exemplars (link to metrics)       │
└──────────────────────────────────────┘
```

---

## Logging Pipeline

```
Application Code
     │
     │ logger.info(...)
     ▼
┌──────────────────────────────────────┐
│   Structured Logger (Winston)       │
│                                      │
│  • JSON formatter                    │
│  • Add timestamp                     │
│  • Add trace_id/span_id              │
│  • Add service metadata              │
└──────────────┬───────────────────────┘
               │
               │ JSON log line
               ▼
┌──────────────────────────────────────┐
│   Log Outputs                        │
│                                      │
│  • stdout (Docker captures)          │
│  • File: /var/log/app.log            │
│  • OTLP exporter (optional)          │
└──────────────┬───────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
     ▼                   ▼
┌─────────────┐    ┌──────────────┐
│   Fluentd   │    │     OTLP     │
│   / Fluent  │    │  Collector   │
│    Bit      │    │              │
│             │    │ • Parse JSON │
│ • Tail logs │    │ • Add labels │
│ • Parse     │    │ • Route      │
│ • Filter    │    │              │
└──────┬──────┘    └──────┬───────┘
       │                  │
       │ HTTP Push        │
       ▼                  ▼
┌──────────────────────────────────────┐
│   Loki / Elasticsearch               │
│                                      │
│  Loki:                               │
│  • Index labels only                 │
│  • Compress chunks                   │
│  • S3 backend                        │
│  • 30d retention                     │
│                                      │
│  Elasticsearch:                      │
│  • Full-text index                   │
│  • Multiple indices                  │
│  • ILM policies                      │
│  • 90d retention                     │
└──────────────┬───────────────────────┘
               │
               │ LogQL / DSL
               ▼
┌──────────────────────────────────────┐
│   Query & Analysis                   │
│                                      │
│  Grafana Explore:                    │
│  • Log browser                       │
│  • Filter by labels                  │
│  • Correlate with traces             │
│  • Live tail                         │
│                                      │
│  Kibana (if ES):                     │
│  • Discover                          │
│  • Visualizations                    │
│  • Dashboards                        │
└──────────────────────────────────────┘
```

---

## Alert Flow

```
Metric/Log Data
     │
     │ Time series
     ▼
┌──────────────────────────────────────┐
│   Prometheus                         │
│                                      │
│  • Evaluate rules every 15s          │
│  • Load alert rules                  │
│  • Execute PromQL queries            │
└──────────────┬───────────────────────┘
               │
               │ Condition Met
               ▼
┌──────────────────────────────────────┐
│   Alert Evaluation                   │
│                                      │
│  IF: error_rate > 0.05               │
│  FOR: 5m                             │
│  THEN: FIRING                        │
└──────────────┬───────────────────────┘
               │
               │ Alert sent
               ▼
┌──────────────────────────────────────┐
│   Alertmanager                       │
│                                      │
│  1. Group similar alerts             │
│  2. Apply silences                   │
│  3. Check inhibition rules           │
│  4. Route by label                   │
└──────────────┬───────────────────────┘
               │
     ┌─────────┼─────────┐
     │         │         │
     ▼         ▼         ▼
┌─────────┐ ┌────────┐ ┌────────┐
│ PagerDuty│ │ Slack  │ │Email  │
│         │ │        │ │       │
│severity:│ │#alerts │ │team@  │
│critical │ │channel │ │       │
└────┬────┘ └────────┘ └───────┘
     │
     │ Page sent
     ▼
┌─────────────┐
│  On-call    │
│  Engineer   │
│             │
│ • Ack alert │
│ • Investigate│
│ • Resolve   │
└─────────────┘
```

---

## Dashboard Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Grafana                              │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Dashboard: Executive Overview                    │  │
│  │                                                   │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐        │  │
│  │  │ Request  │  │  Error   │  │ P95      │        │  │
│  │  │  Rate    │  │  Rate    │  │ Latency  │        │  │
│  │  │ (Graph)  │  │ (Graph)  │  │ (Gauge)  │        │  │
│  │  └──────────┘  └──────────┘  └──────────┘        │  │
│  │                                                   │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐        │  │
│  │  │ Hourly   │  │  Token   │  │  Cache   │        │  │
│  │  │  Cost    │  │  Usage   │  │ Hit Rate │        │  │
│  │  │  (Stat)  │  │ (Graph)  │  │  (Stat)  │        │  │
│  │  └──────────┘  └──────────┘  └──────────┘        │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  Data Sources:                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │Prometheus│  │  Loki    │  │  Tempo   │              │
│  │ (Metrics)│  │  (Logs)  │  │ (Traces) │              │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘              │
└────────┼─────────────┼─────────────┼────────────────────┘
         │             │             │
         │             │             │
         ▼             ▼             ▼
    ┌────────────────────────────────────┐
    │   Unified Query Interface          │
    │                                    │
    │  • PromQL for metrics              │
    │  • LogQL for logs                  │
    │  • TraceQL for traces              │
    │  • Cross-datasource joins          │
    └────────────────────────────────────┘
```

---

## Deployment Architecture

### Kubernetes Deployment

```
┌─────────────────────────────────────────────────────────┐
│                 Kubernetes Cluster                      │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Namespace: llm-simulator                         │  │
│  │                                                   │  │
│  │  ┌──────────────────────────────────────────┐    │  │
│  │  │  Deployment: llm-simulator               │    │  │
│  │  │  Replicas: 3                             │    │  │
│  │  │                                          │    │  │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐        │    │  │
│  │  │  │ Pod 1  │ │ Pod 2  │ │ Pod 3  │        │    │  │
│  │  │  │        │ │        │ │        │        │    │  │
│  │  │  │ App    │ │ App    │ │ App    │        │    │  │
│  │  │  │ OTEL   │ │ OTEL   │ │ OTEL   │        │    │  │
│  │  │  │ SDK    │ │ SDK    │ │ SDK    │        │    │  │
│  │  │  └───┬────┘ └───┬────┘ └───┬────┘        │    │  │
│  │  └──────┼──────────┼──────────┼─────────────┘    │  │
│  │         │          │          │                  │  │
│  │         └──────────┴──────────┘                  │  │
│  │                    │                             │  │
│  │         ┌──────────┴──────────┐                  │  │
│  │         │                     │                  │  │
│  │         ▼                     ▼                  │  │
│  │  ┌─────────────┐       ┌──────────────┐         │  │
│  │  │   Service   │       │  ServiceMon  │         │  │
│  │  │             │       │              │         │  │
│  │  │ :3000 (app) │       │ Prometheus   │         │  │
│  │  │ :9464 (met) │       │  scrapes     │         │  │
│  │  └─────────────┘       └──────────────┘         │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Namespace: observability                         │  │
│  │                                                   │  │
│  │  ┌─────────────┐  ┌──────────────┐               │  │
│  │  │ OTEL        │  │ Prometheus   │               │  │
│  │  │ Collector   │  │              │               │  │
│  │  │             │  │ • TSDB       │               │  │
│  │  │ • Receives  │  │ • Rules      │               │  │
│  │  │ • Processes │  │ • Alerts     │               │  │
│  │  │ • Exports   │  │              │               │  │
│  │  └──────┬──────┘  └──────┬───────┘               │  │
│  │         │                │                       │  │
│  │  ┌──────┴──────┐  ┌──────┴───────┐               │  │
│  │  │   Tempo     │  │ Loki         │               │  │
│  │  │             │  │              │               │  │
│  │  │ • Traces    │  │ • Logs       │               │  │
│  │  │ • S3        │  │ • S3         │               │  │
│  │  └─────────────┘  └──────────────┘               │  │
│  │                                                   │  │
│  │  ┌─────────────┐  ┌──────────────┐               │  │
│  │  │  Grafana    │  │Alertmanager  │               │  │
│  │  │             │  │              │               │  │
│  │  │ • Dashboards│  │ • Routing    │               │  │
│  │  │ • Explore   │  │ • PagerDuty  │               │  │
│  │  └─────────────┘  └──────────────┘               │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Persistent Volumes                               │  │
│  │                                                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐          │  │
│  │  │Prometheus│ │  Tempo   │ │   Loki   │          │  │
│  │  │  PVC     │ │   PVC    │ │   PVC    │          │  │
│  │  │ 500GB    │ │  100GB   │ │  200GB   │          │  │
│  │  └──────────┘ └──────────┘ └──────────┘          │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

**Maintained by:** Platform Engineering
**Version:** 1.0
**Last Updated:** 2025-11-26

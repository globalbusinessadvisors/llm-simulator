# SPARC Specification: Phase 4 - Operations

## S - Specification

### Overview
Implement production-grade operational capabilities including backup/recovery, secrets management, disaster recovery planning, and operational documentation to ensure reliable and maintainable production deployments.

### Objectives
1. Implement automated backup strategy with Velero
2. Integrate secrets management (HashiCorp Vault / Cloud KMS)
3. Define and document RTO/RPO objectives
4. Create disaster recovery procedures
5. Develop operational runbooks

### Requirements

#### 4.1 Backup Strategy
- **MUST** implement Velero for Kubernetes backup
- **MUST** backup PersistentVolumeClaims for StatefulSet deployments
- **MUST** schedule daily backups with 30-day retention
- **SHOULD** support cross-region backup replication
- **SHOULD** automate backup verification/restore testing
- **MUST** document backup recovery procedures

#### 4.2 Secrets Management
- **MUST** integrate with at least one secrets provider (Vault/AWS/Azure/GCP)
- **MUST** remove all hardcoded secrets from configuration
- **MUST** support automatic secret rotation
- **SHOULD** use Kubernetes External Secrets Operator
- **SHOULD** encrypt secrets at rest and in transit
- **MUST** audit secret access

#### 4.3 Disaster Recovery
- **MUST** define RTO (Recovery Time Objective) ≤ 1 hour
- **MUST** define RPO (Recovery Point Objective) ≤ 15 minutes
- **MUST** document failover procedures
- **SHOULD** implement automated failover for critical components
- **SHOULD** test DR procedures quarterly
- **MUST** maintain DR runbook

#### 4.4 Connection Draining
- **MUST** implement graceful shutdown with request completion
- **MUST** configure preStop hooks for Kubernetes
- **SHOULD** track in-flight requests during shutdown
- **SHOULD** support configurable drain timeout
- **MUST** disable readiness before draining

#### 4.5 Operational Documentation
- **MUST** create runbooks for common operations
- **MUST** document incident response procedures
- **MUST** create troubleshooting guides
- **SHOULD** implement operational dashboards
- **SHOULD** create capacity planning documentation

### Success Criteria
- Automated daily backups running and verified
- Secrets injected from external provider (no hardcoded values)
- DR procedures documented and tested
- Graceful shutdown completes all in-flight requests
- Runbooks cover 90% of operational scenarios

---

## P - Pseudocode

### 4.1 Velero Backup Configuration

```
// deploy/velero/backup-schedule.yaml
RESOURCE Schedule:
    metadata:
        name: llm-simulator-daily
        namespace: velero
    spec:
        schedule: "0 2 * * *"  // Daily at 2 AM UTC
        template:
            includedNamespaces:
                - llm-simulator
            includedResources:
                - configmaps
                - secrets
                - persistentvolumeclaims
                - deployments
                - statefulsets
                - services
            storageLocation: default
            volumeSnapshotLocations:
                - default
            ttl: 720h  // 30 days retention

// deploy/velero/backup-storage.yaml
RESOURCE BackupStorageLocation:
    metadata:
        name: default
        namespace: velero
    spec:
        provider: aws  // or azure, gcp
        bucket: llm-simulator-backups
        config:
            region: us-west-2
            s3ForcePathStyle: "true"
        credential:
            name: velero-credentials
            key: cloud

// Backup verification script
FUNCTION verify_backup(backup_name):
    // List backup contents
    contents = velero backup describe $backup_name --details

    // Verify expected resources
    ASSERT "llm-simulator" IN contents.namespaces
    ASSERT "configmaps" IN contents.resources
    ASSERT "persistentvolumeclaims" IN contents.resources

    // Check for errors
    ASSERT contents.errors == 0
    ASSERT contents.warnings < 5

    // Test restore to temporary namespace
    restore_name = "verify-{backup_name}-{timestamp}"
    velero restore create $restore_name \
        --from-backup $backup_name \
        --namespace-mappings llm-simulator:llm-simulator-verify

    // Verify restore success
    WAIT_FOR restore $restore_name to complete
    ASSERT restore.status == "Completed"

    // Clean up verification namespace
    kubectl delete namespace llm-simulator-verify

    RETURN success
```

### 4.2 Secrets Management with External Secrets Operator

```
// deploy/external-secrets/secret-store.yaml
RESOURCE SecretStore:
    metadata:
        name: vault-backend
        namespace: llm-simulator
    spec:
        provider:
            vault:
                server: "https://vault.example.com"
                path: "secret"
                version: "v2"
                auth:
                    kubernetes:
                        mountPath: "kubernetes"
                        role: "llm-simulator"
                        serviceAccountRef:
                            name: llm-simulator

// deploy/external-secrets/external-secret.yaml
RESOURCE ExternalSecret:
    metadata:
        name: llm-simulator-secrets
        namespace: llm-simulator
    spec:
        refreshInterval: 1h
        secretStoreRef:
            name: vault-backend
            kind: SecretStore
        target:
            name: llm-simulator-api-keys
            creationPolicy: Owner
        data:
            - secretKey: admin-api-key
              remoteRef:
                key: llm-simulator/api-keys
                property: admin
            - secretKey: user-api-key
              remoteRef:
                key: llm-simulator/api-keys
                property: user
            - secretKey: otlp-auth-token
              remoteRef:
                key: llm-simulator/telemetry
                property: otlp-token

// Application configuration to use secrets
STRUCT SecretsConfig:
    api_keys_secret: String  // Name of K8s secret
    refresh_interval: Duration

FUNCTION load_secrets(config: SecretsConfig) -> ApiKeys:
    // Read from mounted secret
    secret_path = "/var/run/secrets/llm-simulator"

    keys = ApiKeys::new()

    // Load admin key
    admin_key = read_file(f"{secret_path}/admin-api-key")
    keys.add("admin", admin_key, Role::Admin)

    // Load user key
    user_key = read_file(f"{secret_path}/user-api-key")
    keys.add("user", user_key, Role::User)

    // Start secret refresh watcher
    spawn_secret_watcher(secret_path, config.refresh_interval, |new_secrets| {
        keys.refresh(new_secrets)
        log.info("API keys refreshed from secret store")
    })

    RETURN keys

// Secret rotation handler
FUNCTION handle_secret_rotation(old_keys, new_keys):
    // Grace period for old keys
    grace_period = Duration::from_secs(300)  // 5 minutes

    // Add new keys immediately
    for key in new_keys:
        api_keys.add(key)

    // Schedule old key removal
    spawn(async {
        sleep(grace_period).await
        for key in old_keys - new_keys:
            api_keys.remove(key)
            log.info("Removed rotated API key", key_id=key.id)
    })
```

### 4.3 Disaster Recovery Procedures

```
// DR Configuration
STRUCT DRConfig:
    rto: Duration = Duration::from_secs(3600)   // 1 hour
    rpo: Duration = Duration::from_secs(900)    // 15 minutes
    primary_region: String
    failover_region: String
    backup_bucket: String
    notification_channels: Vec<String>

// Failover procedure (automated)
FUNCTION initiate_failover(reason: String, config: DRConfig):
    log.critical("Initiating DR failover", reason=reason)

    // Step 1: Notify team
    notify_channels(config.notification_channels, FailoverAlert {
        reason: reason,
        timestamp: now(),
        primary_region: config.primary_region,
        failover_region: config.failover_region,
    })

    // Step 2: Get latest backup
    latest_backup = velero_get_latest_backup(config.backup_bucket)
    IF latest_backup.age > config.rpo:
        log.warn("Latest backup exceeds RPO", age=latest_backup.age)

    // Step 3: Restore to failover region
    restore_result = velero_restore(
        backup: latest_backup,
        target_region: config.failover_region,
        namespace_mapping: None,  // Keep same namespace
    )

    // Step 4: Verify restoration
    health_check_result = health_check_with_retry(
        endpoint: config.failover_endpoint,
        retries: 10,
        interval: Duration::from_secs(30),
    )

    IF NOT health_check_result.healthy:
        log.critical("Failover health check failed")
        notify_channels(config.notification_channels, FailoverFailed { ... })
        RETURN FailoverResult::Failed

    // Step 5: Update DNS/traffic routing
    update_traffic_routing(
        from: config.primary_region,
        to: config.failover_region,
    )

    // Step 6: Verify traffic flowing
    verify_traffic(config.failover_endpoint)

    // Step 7: Log completion
    elapsed = now() - start
    log.info("Failover completed", duration=elapsed)

    IF elapsed > config.rto:
        log.warn("Failover exceeded RTO", rto=config.rto, actual=elapsed)

    notify_channels(config.notification_channels, FailoverComplete {
        duration: elapsed,
        backup_age: latest_backup.age,
        ...
    })

    RETURN FailoverResult::Success

// Failback procedure (manual with automation assistance)
FUNCTION initiate_failback(config: DRConfig):
    log.info("Initiating failback to primary region")

    // Step 1: Verify primary region health
    primary_health = health_check(config.primary_endpoint)
    IF NOT primary_health.healthy:
        RETURN FailbackResult::PrimaryNotReady

    // Step 2: Sync data from failover to primary
    // (Application-specific - may involve data reconciliation)
    sync_result = sync_data(
        source: config.failover_region,
        target: config.primary_region,
    )

    // Step 3: Gradually shift traffic back
    for percentage in [10, 25, 50, 75, 100]:
        update_traffic_split(
            primary: percentage,
            failover: 100 - percentage,
        )

        // Monitor for errors
        sleep(Duration::from_mins(5))

        error_rate = get_error_rate(config.primary_endpoint)
        IF error_rate > 0.01:  // >1% errors
            log.warn("High error rate during failback, pausing")
            RETURN FailbackResult::Paused(percentage)

    // Step 4: Complete failback
    log.info("Failback completed successfully")
    RETURN FailbackResult::Success
```

### 4.4 Connection Draining Implementation

```
// src/server/shutdown.rs

STRUCT ShutdownState:
    in_flight_requests: AtomicU64
    draining: AtomicBool
    drain_timeout: Duration

IMPL ShutdownState:
    FUNCTION new(drain_timeout: Duration) -> Self:
        Self {
            in_flight_requests: AtomicU64::new(0),
            draining: AtomicBool::new(false),
            drain_timeout,
        }

    FUNCTION request_started(&self):
        self.in_flight_requests.fetch_add(1, Ordering::SeqCst)

    FUNCTION request_completed(&self):
        self.in_flight_requests.fetch_sub(1, Ordering::SeqCst)

    FUNCTION is_draining(&self) -> bool:
        self.draining.load(Ordering::SeqCst)

    FUNCTION start_drain(&self):
        self.draining.store(true, Ordering::SeqCst)

    FUNCTION in_flight_count(&self) -> u64:
        self.in_flight_requests.load(Ordering::SeqCst)

// Request tracking middleware
FUNCTION request_tracking_middleware(state: ShutdownState, request, next):
    // Reject new requests if draining
    IF state.is_draining():
        RETURN error_response(503, "Service is shutting down")

    // Track request start
    state.request_started()

    // Execute request
    response = next(request).await

    // Track request completion
    state.request_completed()

    RETURN response

// Enhanced shutdown handler
FUNCTION graceful_shutdown(state: ShutdownState, server_handle):
    // Step 1: Mark as draining (stop accepting new requests)
    log.info("Starting graceful shutdown, marking as draining")
    state.start_drain()

    // Step 2: Disable readiness check
    // (Kubernetes will stop sending traffic)
    set_not_ready()

    // Step 3: Wait for in-flight requests with timeout
    drain_start = Instant::now()

    WHILE state.in_flight_count() > 0:
        IF drain_start.elapsed() > state.drain_timeout:
            log.warn(
                "Drain timeout exceeded, forcing shutdown",
                remaining_requests=state.in_flight_count()
            )
            BREAK

        log.info(
            "Waiting for in-flight requests",
            count=state.in_flight_count(),
            elapsed=drain_start.elapsed()
        )

        sleep(Duration::from_millis(100))

    // Step 4: Shutdown server
    log.info("All requests drained, shutting down server")
    server_handle.graceful_shutdown(None)

    // Step 5: Cleanup telemetry
    shutdown_telemetry()

    log.info("Server shutdown complete")

// Kubernetes preStop hook script
SCRIPT prestop.sh:
    #!/bin/bash
    # Signal application to start draining
    curl -X POST http://localhost:8080/admin/drain

    # Wait for drain to complete (up to 55 seconds)
    # Kubernetes terminationGracePeriodSeconds should be 60
    for i in {1..55}; do
        READY=$(curl -s http://localhost:8080/ready | jq -r '.ready')
        if [ "$READY" == "false" ]; then
            echo "Drain complete after ${i} seconds"
            exit 0
        fi
        sleep 1
    done

    echo "Drain timeout, proceeding with termination"
    exit 0
```

### 4.5 Operational Runbooks

```
// docs/runbooks/common-operations.md

RUNBOOK: Scaling the Service

TRIGGER: High CPU usage (>80%) or high latency (P99 > 3s)

STEPS:
    1. Check current replica count:
       kubectl get deployment llm-simulator -n llm-simulator

    2. Check HPA status:
       kubectl get hpa llm-simulator -n llm-simulator

    3. If HPA is at max replicas:
       a. Consider increasing HPA maxReplicas
       b. Or vertically scale (increase CPU/memory limits)

    4. Manual scale (if needed):
       kubectl scale deployment llm-simulator --replicas=<N>

    5. Monitor after scaling:
       watch kubectl top pods -n llm-simulator

ROLLBACK:
    kubectl scale deployment llm-simulator --replicas=<original>

---

RUNBOOK: Investigating High Error Rate

TRIGGER: Alert "LLMSimulatorHighErrorRate" fired

STEPS:
    1. Check error breakdown:
       curl http://localhost:8080/metrics | grep errors_total

    2. Check recent logs:
       kubectl logs -l app=llm-simulator --since=10m | grep -i error

    3. Check chaos engineering status:
       curl http://localhost:8080/admin/chaos/status

    4. If chaos is enabled unexpectedly:
       curl -X POST http://localhost:8080/admin/chaos/disable

    5. Check circuit breaker status:
       curl http://localhost:8080/admin/stats | jq '.circuit_breakers'

    6. If circuit breaker is open:
       Wait for recovery timeout, or restart pods if urgent

    7. Check external dependencies:
       - OTLP collector status
       - Prometheus scrape success

ESCALATION:
    If error rate persists > 15 minutes, page on-call engineer

---

RUNBOOK: Restoring from Backup

TRIGGER: Data corruption or accidental deletion

PREREQUISITES:
    - Velero CLI installed
    - kubectl access to cluster
    - Backup storage credentials

STEPS:
    1. List available backups:
       velero backup get

    2. Describe target backup:
       velero backup describe <backup-name> --details

    3. Create restore:
       velero restore create --from-backup <backup-name>

    4. Monitor restore progress:
       velero restore describe <restore-name>

    5. Verify restored resources:
       kubectl get all -n llm-simulator

    6. Verify application health:
       curl http://llm-simulator.example.com/health

VALIDATION:
    - Health check returns "healthy"
    - Sample API request succeeds
    - Metrics endpoint accessible

ROLLBACK:
    If restore fails, try previous backup or escalate

---

RUNBOOK: Secret Rotation

TRIGGER: Scheduled rotation or security incident

STEPS:
    1. Generate new secret values:
       # Use your secret generation tool

    2. Update secrets in Vault/secret manager:
       vault kv put secret/llm-simulator/api-keys admin=<new> user=<new>

    3. Wait for External Secrets Operator to sync:
       kubectl get externalsecret llm-simulator-secrets -w

    4. Verify secret updated:
       kubectl get secret llm-simulator-api-keys -o yaml

    5. Application will auto-reload secrets (5 min grace period)

    6. Test with new credentials:
       curl -H "Authorization: Bearer <new-key>" http://localhost:8080/v1/models

    7. Invalidate old credentials after grace period

EMERGENCY ROTATION:
    If compromised, skip grace period:
    kubectl rollout restart deployment/llm-simulator
```

---

## A - Architecture

### Backup Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Backup Architecture                             │
└─────────────────────────────────────────────────────────────────────┘

  Primary Region (us-west-2)              Backup Region (us-east-1)
  ┌─────────────────────────┐            ┌─────────────────────────┐
  │   LLM-Simulator         │            │   LLM-Simulator         │
  │   Namespace             │            │   (Standby)             │
  │                         │            │                         │
  │  ┌─────────────────┐   │            │                         │
  │  │  StatefulSet    │   │            │                         │
  │  │  + PVC (50Gi)   │   │            │                         │
  │  └────────┬────────┘   │            │                         │
  │           │             │            │                         │
  │  ┌────────▼────────┐   │            │                         │
  │  │   Velero Agent  │   │            │                         │
  │  └────────┬────────┘   │            │                         │
  └───────────┼─────────────┘            └─────────────────────────┘
              │
              │ Daily backup (2 AM UTC)
              │ Incremental snapshots
              ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │                    S3 / Blob Storage                             │
  │                                                                  │
  │  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐ │
  │  │ Backup Day 1   │    │ Backup Day 2   │    │ Backup Day N   │ │
  │  │ - manifests    │    │ - manifests    │    │ - manifests    │ │
  │  │ - PVC snapshot │    │ - PVC snapshot │    │ - PVC snapshot │ │
  │  └────────────────┘    └────────────────┘    └────────────────┘ │
  │                                                                  │
  │  Cross-Region Replication ─────────────────────────────────────▶ │
  │                                                                  │
  └─────────────────────────────────────────────────────────────────┘
              │
              │ On-demand restore
              ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │              Failover Region (Activated on DR)                   │
  │                                                                  │
  │  Velero Restore ──▶ Create namespace                            │
  │                 ──▶ Restore ConfigMaps/Secrets                   │
  │                 ──▶ Restore StatefulSet                          │
  │                 ──▶ Restore PVC from snapshot                    │
  │                 ──▶ Update DNS/Ingress                           │
  │                                                                  │
  └─────────────────────────────────────────────────────────────────┘
```

### Secrets Management Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                   Secrets Management Flow                           │
└─────────────────────────────────────────────────────────────────────┘

   ┌───────────────────────┐
   │    HashiCorp Vault    │
   │    (Secret Store)     │
   │                       │
   │  secret/llm-simulator │
   │  ├── api-keys         │
   │  │   ├── admin        │
   │  │   └── user         │
   │  └── telemetry        │
   │      └── otlp-token   │
   └───────────┬───────────┘
               │
               │ Kubernetes Auth
               ▼
   ┌───────────────────────┐
   │  External Secrets     │
   │  Operator             │
   │                       │
   │  Syncs every 1 hour   │
   │  Creates K8s Secrets  │
   └───────────┬───────────┘
               │
               │ Creates/Updates
               ▼
   ┌───────────────────────┐
   │  Kubernetes Secret    │
   │  llm-simulator-keys   │
   │                       │
   │  data:                │
   │    admin-api-key: *** │
   │    user-api-key: ***  │
   │    otlp-token: ***    │
   └───────────┬───────────┘
               │
               │ Volume Mount
               ▼
   ┌───────────────────────┐
   │  LLM-Simulator Pod    │
   │                       │
   │  /var/run/secrets/    │
   │  └── llm-simulator/   │
   │      ├── admin-key    │
   │      ├── user-key     │
   │      └── otlp-token   │
   │                       │
   │  File watcher detects │
   │  changes and reloads  │
   └───────────────────────┘
```

### Connection Draining Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                   Graceful Shutdown Sequence                        │
└─────────────────────────────────────────────────────────────────────┘

  Time ──────────────────────────────────────────────────────────────▶

  T+0: SIGTERM received
  │
  ├── preStop hook executes
  │   └── POST /admin/drain
  │
  ├── Server marks draining=true
  │   └── Readiness probe returns false
  │
  ├── Kubernetes removes pod from Service endpoints
  │   └── No new traffic routed to pod
  │
  ├── In-flight request tracking
  │   │
  │   │  ┌─────────────────────────────────────────┐
  │   │  │ In-flight: 50 → 30 → 15 → 5 → 0        │
  │   │  │                                         │
  │   │  │ [====================================] │
  │   │  │           Progress bar                  │
  │   │  └─────────────────────────────────────────┘
  │   │
  │   └── Polls every 100ms, logs progress
  │
  ├── All requests drained (or timeout at T+55s)
  │
  ├── Telemetry shutdown
  │   ├── Flush metrics
  │   ├── Export pending traces
  │   └── Close connections
  │
  └── T+60s: terminationGracePeriodSeconds expires
      └── SIGKILL if still running (should not happen)

  Timeline:
  ────────────────────────────────────────────────────────────────────
  0s        5s         30s        55s        60s
  │         │          │          │          │
  SIGTERM   preStop    Draining   Drain      SIGKILL
            complete   in-flight  complete   (forced)
```

### File Structure

```
deploy/
├── velero/
│   ├── backup-schedule.yaml        # Daily backup schedule
│   ├── backup-storage.yaml         # S3/Blob storage config
│   └── restore-template.yaml       # Restore template
├── external-secrets/
│   ├── secret-store.yaml           # Vault connection
│   └── external-secret.yaml        # Secret sync config
├── dr/
│   ├── failover-runbook.md         # DR procedures
│   └── failback-runbook.md         # Recovery procedures
└── scripts/
    ├── backup-verify.sh            # Backup verification
    ├── failover.sh                 # Automated failover
    └── secret-rotate.sh            # Secret rotation

docs/
├── runbooks/
│   ├── common-operations.md        # Day-to-day operations
│   ├── incident-response.md        # Incident handling
│   ├── troubleshooting.md          # Problem resolution
│   └── capacity-planning.md        # Scaling guidance
└── architecture/
    └── dr-architecture.md          # DR design document

src/
└── server/
    └── shutdown.rs                 # Connection draining logic
```

---

## R - Refinement

### Edge Cases

1. **Backup During High Load**
   - Schedule backups during low-traffic periods
   - Use incremental snapshots to minimize impact
   - Monitor backup duration and adjust schedule

2. **Secret Sync Failure**
   - Keep previous secret version as fallback
   - Alert on sync failures
   - Manual sync capability for emergencies

3. **Partial Failover**
   - Health checks before declaring failover complete
   - Automatic rollback if health checks fail
   - Manual override capability

4. **Long-Running Requests During Drain**
   - Configurable drain timeout per request type
   - Force-close after timeout with error response
   - Log requests that exceeded drain timeout

5. **Split-Brain During DR**
   - Use distributed lock for failover coordination
   - Single source of truth for active region
   - Automatic detection and resolution

### Error Handling

```rust
// Backup verification error handling
pub async fn verify_backup(backup_name: &str) -> Result<BackupStatus, BackupError> {
    let backup = velero_client.get_backup(backup_name).await
        .map_err(|e| BackupError::NotFound(e))?;

    if backup.status.phase != "Completed" {
        return Err(BackupError::IncompleteBackup {
            phase: backup.status.phase,
            errors: backup.status.errors,
        });
    }

    // Test restore to verification namespace
    let restore = velero_client.create_restore(RestoreSpec {
        backup_name: backup_name.to_string(),
        namespace_mappings: hashmap! {
            "llm-simulator" => "llm-simulator-verify"
        },
        ..Default::default()
    }).await?;

    // Wait for restore with timeout
    let result = tokio::time::timeout(
        Duration::from_mins(30),
        wait_for_restore(&restore.name),
    ).await.map_err(|_| BackupError::RestoreTimeout)?;

    // Cleanup verification namespace
    cleanup_namespace("llm-simulator-verify").await?;

    Ok(BackupStatus {
        name: backup_name.to_string(),
        age: backup.creation_timestamp.elapsed(),
        verified: result.is_ok(),
        errors: result.err().map(|e| e.to_string()),
    })
}

// Graceful shutdown error handling
pub async fn graceful_shutdown(state: &ShutdownState) -> ShutdownResult {
    state.start_drain();

    let drain_result = tokio::time::timeout(
        state.drain_timeout,
        drain_in_flight_requests(state),
    ).await;

    match drain_result {
        Ok(Ok(())) => {
            tracing::info!("Clean shutdown, all requests completed");
            ShutdownResult::Clean
        }
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Error during drain");
            ShutdownResult::Error(e)
        }
        Err(_) => {
            let remaining = state.in_flight_count();
            tracing::warn!(
                remaining_requests = remaining,
                "Drain timeout exceeded, forcing shutdown"
            );
            ShutdownResult::Timeout { remaining_requests: remaining }
        }
    }
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    // Test connection draining
    #[tokio::test]
    async fn test_connection_draining() {
        let state = ShutdownState::new(Duration::from_secs(5));

        // Simulate in-flight requests
        state.request_started();
        state.request_started();

        // Start drain
        let drain_handle = tokio::spawn({
            let state = state.clone();
            async move { graceful_shutdown(&state).await }
        });

        // Verify new requests rejected
        assert!(state.is_draining());

        // Complete in-flight requests
        tokio::time::sleep(Duration::from_millis(100)).await;
        state.request_completed();
        state.request_completed();

        // Drain should complete
        let result = drain_handle.await.unwrap();
        assert!(matches!(result, ShutdownResult::Clean));
    }

    // Test drain timeout
    #[tokio::test]
    async fn test_drain_timeout() {
        let state = ShutdownState::new(Duration::from_millis(100));

        // Start request that won't complete
        state.request_started();

        let result = graceful_shutdown(&state).await;

        assert!(matches!(result, ShutdownResult::Timeout { .. }));
    }

    // Test secret rotation
    #[tokio::test]
    async fn test_secret_rotation_grace_period() {
        let keys = ApiKeys::new();
        let old_key = "old-key-123";
        let new_key = "new-key-456";

        keys.add("user", old_key, Role::User);

        // Rotate keys
        handle_secret_rotation(&keys, vec![old_key], vec![new_key]).await;

        // Both keys should work during grace period
        assert!(keys.validate(old_key).is_some());
        assert!(keys.validate(new_key).is_some());

        // After grace period, old key invalid
        tokio::time::sleep(Duration::from_secs(301)).await;
        assert!(keys.validate(old_key).is_none());
        assert!(keys.validate(new_key).is_some());
    }
}
```

---

## C - Completion

### Definition of Done

- [ ] Velero installed and configured in cluster
- [ ] Daily backup schedule running
- [ ] Backup verification automated (weekly)
- [ ] External Secrets Operator deployed
- [ ] Secrets syncing from Vault/cloud provider
- [ ] Secret rotation tested and documented
- [ ] RTO/RPO defined and documented
- [ ] Failover procedure documented and tested
- [ ] Connection draining implemented
- [ ] preStop hook configured in deployment
- [ ] Runbooks created for common operations
- [ ] Incident response procedure documented
- [ ] DR test completed successfully

### Verification Checklist

```bash
# 1. Verify Velero installation
velero version
velero backup-location get

# 2. Verify backup schedule
velero schedule get
velero backup get | head -5

# 3. Test backup restore
velero backup create test-backup --include-namespaces llm-simulator
velero restore create --from-backup test-backup --namespace-mappings llm-simulator:test-restore
kubectl get all -n test-restore

# 4. Verify External Secrets
kubectl get externalsecrets -n llm-simulator
kubectl get secrets llm-simulator-api-keys -n llm-simulator

# 5. Test secret refresh
# Update secret in Vault, wait for sync
kubectl get secret llm-simulator-api-keys -o yaml | grep -v "^  [a-z]"

# 6. Test graceful shutdown
kubectl exec -it <pod> -- curl -X POST localhost:8080/admin/drain
kubectl logs <pod> | grep -i drain

# 7. Verify preStop hook
kubectl describe pod <pod> | grep -A5 preStop

# 8. Test failover (DR drill)
./deploy/scripts/failover.sh --dry-run
```

### RTO/RPO Documentation

| Metric | Target | Measurement |
|--------|--------|-------------|
| **RTO** | ≤ 1 hour | Time from incident to service restoration |
| **RPO** | ≤ 15 minutes | Maximum data loss (backup frequency) |
| **Backup Frequency** | Daily + hourly incremental | Full daily, incremental hourly |
| **Backup Retention** | 30 days | Oldest recoverable point |
| **Failover Time** | ≤ 15 minutes | Automated failover duration |
| **Failback Time** | ≤ 2 hours | Manual with gradual traffic shift |

### Rollback Plan

1. **Backup Issues**: Revert to previous Velero configuration
2. **Secret Management**: Fall back to Kubernetes native secrets
3. **DR Failover**: Failback procedure documented
4. **Connection Draining**: Disable with `DRAIN_ENABLED=false` env var

### Monitoring Operations

- Alert: Backup job failure
- Alert: Secret sync failure for >2 hours
- Alert: DR region health check failure
- Alert: Shutdown drain timeout exceeded
- Dashboard: Backup success rate, age of latest backup
- Dashboard: Secret sync latency, rotation events
- Dashboard: Drain duration, remaining requests at shutdown


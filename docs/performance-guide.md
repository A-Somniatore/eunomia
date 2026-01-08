# Eunomia Performance Tuning Guide

> **Version**: 1.0.0  
> **Last Updated**: 2026-01-08

This guide provides recommendations for optimizing Eunomia's performance in production environments.

---

## Table of Contents

1. [Performance Targets](#performance-targets)
2. [Bundle Optimization](#bundle-optimization)
3. [Policy Optimization](#policy-optimization)
4. [gRPC Server Tuning](#grpc-server-tuning)
5. [Cache Configuration](#cache-configuration)
6. [Distribution Settings](#distribution-settings)
7. [Resource Allocation](#resource-allocation)
8. [Monitoring Performance](#monitoring-performance)
9. [Benchmarking](#benchmarking)

---

## Performance Targets

### Latency SLOs

Based on our benchmarks, these are the target latencies for critical operations:

| Operation            | p50 Target | p95 Target | p99 Target |
| -------------------- | ---------- | ---------- | ---------- |
| Bundle Creation      | < 1ms      | < 5ms      | < 10ms     |
| Bundle Signing       | < 1ms      | < 2ms      | < 5ms      |
| Bundle Verification  | < 1ms      | < 2ms      | < 5ms      |
| Checksum Computation | < 500µs    | < 1ms      | < 2ms      |
| Serialization        | < 1ms      | < 2ms      | < 5ms      |
| Single Instance Push | < 100ms    | < 500ms    | < 1s       |
| 10-Instance Push     | < 500ms    | < 2s       | < 5s       |

### Throughput Targets

| Operation               | Minimum Target |
| ----------------------- | -------------- |
| Bundle Signing          | > 500 ops/sec  |
| Bundle Verification     | > 1000 ops/sec |
| Checksum Computation    | > 5000 ops/sec |
| Sustained Push Workflow | > 100 ops/sec  |

---

## Bundle Optimization

### Minimize Bundle Size

Smaller bundles transfer faster and reduce memory usage.

```bash
# Check current bundle size
ls -lh bundle.tar.gz

# Target: < 1MB for most services
# Warning: > 5MB may impact distribution latency
```

**Strategies:**

1. **Remove test files from bundles**

   ```bash
   eunomia build --policy-dir policies/ --exclude "*_test.rego"
   ```

2. **Exclude development data**

   ```bash
   eunomia build --exclude "data/dev-*.json" --exclude "fixtures/"
   ```

3. **Minimize data files**

   - Only include data that policies actually reference
   - Use external data loading for large datasets
   - Consider data compression for large JSON files

4. **Split large policies**
   - One bundle per service (not monolithic)
   - Shared policies in a common bundle

### Bundle Compression

Enable compression for network transfers:

```bash
# Push with compression
eunomia push bundle.tar.gz --compress

# Compression reduces transfer time by ~60-80% for typical policies
```

### Pre-compute Checksums

```bash
# Generate checksum during build
eunomia build --policy-dir policies/ --checksum sha256

# The checksum is stored in .manifest for quick verification
```

---

## Policy Optimization

### Avoid Expensive Patterns

**❌ Expensive: Unbounded Recursion**

```rego
# This can be very slow for large datasets
all_ancestors[ancestor] {
    parent[x] = y
    all_ancestors[y]
    ancestor := y
}
```

**✅ Better: Bounded Iteration**

```rego
# Limit depth explicitly
ancestors[ancestor] {
    parent[x] = ancestor
}
ancestors[ancestor] {
    parent[x] = y
    parent[y] = ancestor
}
```

**❌ Expensive: Large Comprehensions**

```rego
# Creates full array in memory
all_users := [u | u := data.users[_]]
count(all_users) > 1000
```

**✅ Better: Early Termination**

```rego
# Stops at first match
some user in data.users
user.role == "admin"
```

### Use Indexing

OPA automatically indexes certain patterns. Structure policies to leverage indexing:

```rego
# ✅ Indexable - equality on input
allow if {
    input.operation_id == "getUser"
    # ... other conditions
}

# ✅ Indexable - membership test
allow if {
    input.operation_id in {"getUser", "listUsers", "searchUsers"}
}

# ❌ Not indexable - computed key
allow if {
    data.permissions[concat(":", [input.service, input.operation_id])]
}
```

### Minimize Policy Evaluation Time

```rego
# ✅ Put cheap checks first (short-circuit evaluation)
allow if {
    input.caller.type == "user"        # Cheap: field access
    "admin" in input.caller.roles      # Cheap: set membership
    valid_timestamp(input.timestamp)   # Expensive: time parsing
}

# ❌ Avoid expensive checks early
allow if {
    complex_permission_check(input)    # Expensive
    input.caller.type == "user"        # This could have filtered earlier
}
```

### Profile Policies

```bash
# Run tests with profiling
eunomia test policies/ --profile

# Output shows time spent in each rule
# Rule: users_service.authz.allow
#   Evaluations: 1000
#   Total time: 150ms
#   Avg time: 150µs
```

---

## gRPC Server Tuning

### Message Size Limits

Configure for your bundle sizes:

```rust
// In distributor configuration
GrpcServerConfig {
    // Default: 4MB, increase for large bundles
    max_message_size: 16 * 1024 * 1024,  // 16MB

    // Receive limit (for health check responses)
    max_receive_message_size: 1 * 1024 * 1024,  // 1MB
}
```

### Keepalive Settings

```rust
GrpcServerConfig {
    // Send keepalive pings every 30s
    keepalive_interval: Duration::from_secs(30),

    // Wait 10s for keepalive response
    keepalive_timeout: Duration::from_secs(10),

    // Allow keepalive with no active streams
    permit_keepalive_without_calls: true,
}
```

### Connection Settings

```rust
GrpcServerConfig {
    // Maximum concurrent streams per connection
    max_concurrent_streams: 100,

    // Initial connection window size
    initial_connection_window_size: 1024 * 1024,  // 1MB

    // Initial stream window size
    initial_stream_window_size: 512 * 1024,  // 512KB
}
```

### Timeout Configuration

```bash
# CLI timeout settings
eunomia push bundle.tar.gz \
    --timeout 60s \              # Overall operation timeout
    --connect-timeout 10s \      # Connection establishment
    --request-timeout 30s        # Individual request timeout
```

---

## Cache Configuration

### Bundle Cache Settings

The `BundleCache` uses LRU eviction with configurable limits:

```rust
BundleCacheConfig {
    // Maximum cache size (default: 100MB)
    max_size_bytes: 100 * 1024 * 1024,

    // Maximum bundle age before eviction (default: 7 days)
    max_age: Duration::from_secs(7 * 24 * 60 * 60),

    // Cache directory (default: system cache dir)
    cache_dir: PathBuf::from("/var/cache/eunomia/bundles"),
}
```

### Cache Tuning Recommendations

| Environment   | max_size_bytes | max_age | Notes             |
| ------------- | -------------- | ------- | ----------------- |
| Development   | 50MB           | 1 day   | Frequent updates  |
| Staging       | 100MB          | 3 days  | Match production  |
| Production    | 500MB          | 7 days  | High availability |
| Edge/Embedded | 20MB           | 1 day   | Limited storage   |

### Cache Operations

```bash
# Clear cache
eunomia cache clear

# Show cache statistics
eunomia cache stats
# Output:
#   Total size: 45MB / 100MB
#   Entries: 12
#   Oldest: 2 days ago
#   Hit rate: 94%

# Prune old entries
eunomia cache prune --max-age 3d
```

---

## Distribution Settings

### Concurrent Push Configuration

```bash
# Push to multiple instances in parallel
eunomia push bundle.tar.gz \
    --endpoints http://arch-1:8080,http://arch-2:8080,http://arch-3:8080 \
    --max-concurrent 5  # Default: 3
```

**Recommendations:**

| Cluster Size | max_concurrent | Rationale         |
| ------------ | -------------- | ----------------- |
| 1-5          | 3              | Low overhead      |
| 6-20         | 5              | Balanced          |
| 21-50        | 10             | Network bound     |
| 50+          | 20             | Consider batching |

### Deployment Strategy Selection

| Strategy  | When to Use                  | Performance Impact    |
| --------- | ---------------------------- | --------------------- |
| Immediate | Small clusters, non-critical | Fastest, highest risk |
| Canary    | Production, risk-averse      | Slowest, safest       |
| Rolling   | Medium clusters              | Balanced              |

```bash
# Immediate - all at once
eunomia push bundle.tar.gz --strategy immediate

# Canary - 10% first, then rest
eunomia push bundle.tar.gz --strategy canary --canary-percent 10

# Rolling - 2 at a time
eunomia push bundle.tar.gz --strategy rolling --batch-size 2
```

### Retry Configuration

```bash
eunomia push bundle.tar.gz \
    --max-retries 3 \           # Number of retry attempts
    --retry-delay 1s \          # Initial delay between retries
    --retry-backoff-factor 2    # Exponential backoff multiplier
```

---

## Resource Allocation

### Memory Recommendations

| Component                    | Minimum | Recommended | High Load |
| ---------------------------- | ------- | ----------- | --------- |
| CLI Operations               | 64MB    | 256MB       | 512MB     |
| Distribution (10 instances)  | 128MB   | 512MB       | 1GB       |
| Distribution (100 instances) | 512MB   | 2GB         | 4GB       |

### CPU Recommendations

| Operation         | CPU Bound     | Recommended Cores |
| ----------------- | ------------- | ----------------- |
| Policy Validation | Yes           | 2-4               |
| Bundle Signing    | Yes           | 2                 |
| Distribution      | I/O bound     | 1-2               |
| Concurrent Push   | Network bound | 2-4               |

### Kubernetes Resource Limits

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: eunomia-distributor
spec:
  template:
    spec:
      containers:
        - name: distributor
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "1Gi"
              cpu: "1000m"
```

---

## Monitoring Performance

### Key Metrics to Track

```bash
# Enable metrics endpoint
eunomia serve --metrics-port 9090
```

#### Compiler Metrics

| Metric                                               | Type      | Labels              | Description               | Alert Threshold    |
| ---------------------------------------------------- | --------- | ------------------- | ------------------------- | ------------------ |
| `eunomia_compiler_compilations_total`                | Counter   | `service`, `status` | Total policy compilations | failure rate > 10% |
| `eunomia_compiler_compilation_duration_milliseconds` | Histogram | `service`           | Compilation duration      | p99 > 1000ms       |
| `eunomia_compiler_bundle_size_bytes`                 | Histogram | `service`           | Compiled bundle size      | p99 > 10MB         |
| `eunomia_compiler_policies_processed_total`          | Counter   | `service`           | Total policies processed  | -                  |

#### Distributor Metrics

| Metric                                               | Type      | Labels                         | Description              | Alert Threshold   |
| ---------------------------------------------------- | --------- | ------------------------------ | ------------------------ | ----------------- |
| `eunomia_distributor_pushes_total`                   | Counter   | `service`, `version`, `status` | Total bundle pushes      | failure rate > 5% |
| `eunomia_distributor_push_duration_milliseconds`     | Histogram | `service`                      | Push duration            | p99 > 5000ms      |
| `eunomia_distributor_push_batch_size`                | Histogram | `service`                      | Instances per push batch | -                 |
| `eunomia_distributor_rollbacks_total`                | Counter   | `service`, `status`            | Total rollbacks          | > 5/hour          |
| `eunomia_distributor_rollback_duration_milliseconds` | Histogram | `service`                      | Rollback duration        | p99 > 5000ms      |
| `eunomia_distributor_deployments_total`              | Counter   | `service`                      | Total deployments        | -                 |
| `eunomia_distributor_health_checks_total`            | Counter   | `instance`, `status`           | Health check results     | unhealthy > 0     |

### Prometheus Queries

```promql
# Compilation success rate
sum(rate(eunomia_compiler_compilations_total{status="success"}[5m]))
  / sum(rate(eunomia_compiler_compilations_total[5m])) * 100

# Compilation p99 latency
histogram_quantile(0.99,
  sum(rate(eunomia_compiler_compilation_duration_milliseconds_bucket[5m])) by (le))

# Push success rate
sum(rate(eunomia_distributor_pushes_total{status="success"}[5m]))
  / sum(rate(eunomia_distributor_pushes_total[5m])) * 100

# Push p50/p99 latency by service
histogram_quantile(0.50,
  sum(rate(eunomia_distributor_push_duration_milliseconds_bucket[5m])) by (le, service))
histogram_quantile(0.99,
  sum(rate(eunomia_distributor_push_duration_milliseconds_bucket[5m])) by (le, service))

# Bundle size p95
histogram_quantile(0.95,
  sum(rate(eunomia_compiler_bundle_size_bytes_bucket[1h])) by (le, service))

# Rollbacks per hour
sum(increase(eunomia_distributor_rollbacks_total[1h])) by (service)

# Unhealthy instances
sum(rate(eunomia_distributor_health_checks_total{status="unhealthy"}[5m])) by (instance)
```

### Grafana Dashboards

Pre-built Grafana dashboards are available in [`grafana/dashboards/`](../grafana/dashboards/):

| Dashboard                                                             | Description                        | UID                   |
| --------------------------------------------------------------------- | ---------------------------------- | --------------------- |
| [Eunomia Overview](../grafana/dashboards/eunomia-overview.json)       | High-level overview of all metrics | `eunomia-overview`    |
| [Eunomia Compiler](../grafana/dashboards/eunomia-compiler.json)       | Detailed compilation metrics       | `eunomia-compiler`    |
| [Eunomia Distributor](../grafana/dashboards/eunomia-distributor.json) | Detailed distribution metrics      | `eunomia-distributor` |

**Installation:**

```bash
# Option 1: Import via Grafana UI
# Navigate to Dashboards → Import → Upload JSON file

# Option 2: Kubernetes ConfigMap (for Grafana provisioning)
kubectl create configmap eunomia-dashboards \
  --from-file=grafana/dashboards/ \
  -n monitoring

# Option 3: Copy to Grafana provisioning directory
cp grafana/dashboards/*.json /var/lib/grafana/dashboards/eunomia/
```

**Key Panels:**

1. **Summary Stats** - Total compilations, pushes, rollbacks, success rates
2. **Compilation Rate** - By service with success/failure breakdown
3. **Push Latency** - P50/P90/P99 percentiles over time
4. **Rollback Analysis** - Frequency and duration
5. **Instance Health** - Health check timeline by instance
6. **Bundle Sizes** - Distribution and trends

See the [dashboard README](../grafana/dashboards/README.md) for detailed setup instructions and alerting rules.

---

## Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --package eunomia-distributor

# Run specific benchmark
cargo bench --package eunomia-distributor -- bundle_signing

# Generate HTML report
cargo bench --package eunomia-distributor -- --save-baseline main
```

### Benchmark Categories

| Benchmark              | What It Measures             |
| ---------------------- | ---------------------------- |
| `bundle_creation`      | Time to create bundle struct |
| `bundle_signing`       | Ed25519 signing performance  |
| `bundle_verification`  | Signature verification       |
| `checksum_computation` | SHA-256 hashing              |
| `serialization`        | Bundle to/from bytes         |
| `key_operations`       | Key generation and export    |
| `end_to_end`           | Complete workflow            |

### Interpreting Results

```
bundle_signing/sign_bundle
                        time:   [234.56 µs 238.12 µs 241.89 µs]
                        thrpt:  [4.1343 Kelem/s 4.1998 Kelem/s 4.2627 Kelem/s]
                 change: [-2.1234% +0.5678% +3.2109%] (p = 0.42 > 0.05)
                        No change in performance detected.
```

- **time**: [lower bound, estimate, upper bound]
- **thrpt**: Throughput (operations per second)
- **change**: Comparison to baseline

### Load Testing

```bash
# Simulate concurrent deployments
eunomia load-test \
    --bundles 100 \
    --concurrent 10 \
    --duration 5m \
    --endpoints http://arch-1:8080,http://arch-2:8080
```

---

## Performance Checklist

### Before Production Deployment

- [ ] Bundle size < 5MB
- [ ] Policy tests complete in < 30s
- [ ] No unbounded recursion in policies
- [ ] gRPC timeouts configured
- [ ] Cache sized appropriately
- [ ] Resource limits set
- [ ] Monitoring enabled
- [ ] Benchmarks run and baselines established

### Optimization Priority

1. **First**: Bundle size (network impact)
2. **Second**: Policy complexity (evaluation time)
3. **Third**: Concurrency settings (throughput)
4. **Fourth**: Cache tuning (hit rate)
5. **Fifth**: gRPC settings (connection management)

### Quick Wins

1. Enable compression: `--compress`
2. Exclude test files: `--exclude "*_test.rego"`
3. Use canary deployments for large clusters
4. Set appropriate timeouts
5. Monitor cache hit rate

---

## Appendix: Configuration Reference

### Environment Variables

| Variable                 | Description               | Default        |
| ------------------------ | ------------------------- | -------------- |
| `EUNOMIA_MAX_CONCURRENT` | Max parallel pushes       | 3              |
| `EUNOMIA_TIMEOUT`        | Default operation timeout | 30s            |
| `EUNOMIA_CACHE_SIZE`     | Bundle cache max size     | 100MB          |
| `EUNOMIA_CACHE_DIR`      | Cache directory           | System default |
| `EUNOMIA_COMPRESS`       | Enable compression        | false          |
| `EUNOMIA_RETRY_COUNT`    | Retry attempts            | 3              |

### CLI Flags

```bash
eunomia push --help
# Performance-related flags:
#   --timeout <DURATION>        Operation timeout [default: 30s]
#   --connect-timeout <DUR>     Connection timeout [default: 10s]
#   --max-concurrent <N>        Max parallel operations [default: 3]
#   --compress                  Enable bundle compression
#   --batch-size <N>            Rolling deployment batch size
#   --max-retries <N>           Retry attempts [default: 3]
```

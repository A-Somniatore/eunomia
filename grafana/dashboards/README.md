# Eunomia Grafana Dashboards

Pre-built Grafana dashboards for monitoring the Eunomia authorization platform.

## Dashboards

| Dashboard | Description | UID |
|-----------|-------------|-----|
| [eunomia-overview.json](eunomia-overview.json) | High-level overview of all Eunomia metrics | `eunomia-overview` |
| [eunomia-compiler.json](eunomia-compiler.json) | Detailed policy compilation metrics | `eunomia-compiler` |
| [eunomia-distributor.json](eunomia-distributor.json) | Detailed policy distribution metrics | `eunomia-distributor` |

## Prerequisites

- Grafana 10.0+ (dashboards use schema version 38)
- Prometheus data source configured
- Eunomia metrics exposed via `/metrics` endpoint

## Installation

### Option 1: Import via Grafana UI

1. Open Grafana and navigate to **Dashboards** → **Import**
2. Click **Upload JSON file**
3. Select the dashboard JSON file
4. Select your Prometheus data source
5. Click **Import**

### Option 2: Provisioning

Add to your Grafana provisioning configuration:

```yaml
# /etc/grafana/provisioning/dashboards/eunomia.yaml
apiVersion: 1
providers:
  - name: 'Eunomia'
    orgId: 1
    folder: 'Eunomia'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    options:
      path: /var/lib/grafana/dashboards/eunomia
```

Then copy the dashboard JSON files to `/var/lib/grafana/dashboards/eunomia/`.

### Option 3: Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: eunomia-dashboards
  labels:
    grafana_dashboard: "1"
data:
  eunomia-overview.json: |
    # Paste dashboard JSON here
```

## Dashboard Details

### Eunomia Overview

The overview dashboard provides a single-pane view of the entire platform:

**Summary Row:**
- Total Compilations
- Total Pushes
- Total Rollbacks
- Push Success Rate
- Compilation Success Rate
- Push Latency P99

**Compilation Metrics:**
- Compilation rate by service (success/failure)
- Compilation duration by service (p50/p99)

**Distribution Metrics:**
- Push rate by service (success/failure)
- Push duration by service (p50/p99)

**Rollback Metrics:**
- Rollbacks per hour by service
- Rollback duration distribution

**Instance Health:**
- Health check rate by instance

### Eunomia Compiler

Detailed compilation metrics with service filtering:

**Summary Row:**
- Total compilations
- Success rate
- P50/P99 latency

**Panels:**
- Compilation rate by service and status
- Compilation duration percentiles
- Duration distribution histogram
- Bundle size distribution
- Policies processed rate
- Failure analysis (pie chart + table)

### Eunomia Distributor

Detailed distribution metrics with service filtering:

**Summary Row:**
- Total pushes
- Success rate
- Rollback count
- P50/P99 latency
- Total deployments

**Panels:**
- Push rate by service and status
- Push rate by bundle version
- Push duration percentiles
- Push batch size
- Rollback rate by status
- Rollback duration percentiles
- Health check rate by instance
- Instance health timeline
- Deployments per hour by service

## Variables

All dashboards support these variables:

| Variable | Description |
|----------|-------------|
| `datasource` | Prometheus data source |
| `service` | Filter by service name (supports multi-select) |

## Metrics Reference

### Compiler Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `eunomia_compiler_compilations_total` | Counter | `service`, `status` | Total policy compilations |
| `eunomia_compiler_compilation_duration_milliseconds` | Histogram | `service` | Compilation duration |
| `eunomia_compiler_bundle_size_bytes` | Histogram | `service` | Compiled bundle size |
| `eunomia_compiler_policies_processed_total` | Counter | `service` | Total policies processed |

### Distributor Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `eunomia_distributor_pushes_total` | Counter | `service`, `version`, `status` | Total bundle pushes |
| `eunomia_distributor_push_duration_milliseconds` | Histogram | `service` | Push duration |
| `eunomia_distributor_push_batch_size` | Histogram | `service` | Number of instances in push batch |
| `eunomia_distributor_rollbacks_total` | Counter | `service`, `status` | Total rollbacks |
| `eunomia_distributor_rollback_duration_milliseconds` | Histogram | `service` | Rollback duration |
| `eunomia_distributor_deployments_total` | Counter | `service` | Total deployments |
| `eunomia_distributor_health_checks_total` | Counter | `instance`, `status` | Health check results |

## Alerting

Sample alerting rules for Prometheus Alertmanager:

```yaml
groups:
  - name: eunomia
    rules:
      - alert: EunomiaHighCompilationFailureRate
        expr: |
          sum(rate(eunomia_compiler_compilations_total{status="failure"}[5m]))
          / sum(rate(eunomia_compiler_compilations_total[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High compilation failure rate
          description: "More than 10% of compilations are failing"

      - alert: EunomiaHighPushLatency
        expr: |
          histogram_quantile(0.99, 
            sum(rate(eunomia_distributor_push_duration_milliseconds_bucket[5m])) by (le)
          ) > 5000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High push latency
          description: "P99 push latency exceeds 5 seconds"

      - alert: EunomiaInstanceUnhealthy
        expr: |
          sum(rate(eunomia_distributor_health_checks_total{status="unhealthy"}[5m])) by (instance) > 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: Unhealthy instance detected
          description: "Instance {{ $labels.instance }} is reporting unhealthy"

      - alert: EunomiaRollbackSpike
        expr: |
          sum(increase(eunomia_distributor_rollbacks_total[1h])) > 5
        for: 0m
        labels:
          severity: warning
        annotations:
          summary: Rollback spike detected
          description: "More than 5 rollbacks in the last hour"
```

## Customization

### Adjusting Thresholds

Edit the JSON files to customize threshold values:

```json
"thresholds": {
  "mode": "absolute",
  "steps": [
    { "color": "green", "value": null },
    { "color": "yellow", "value": 500 },
    { "color": "red", "value": 1000 }
  ]
}
```

### Adding Panels

Use Grafana's UI to add panels, then export the updated JSON.

### Changing Time Ranges

Modify the default time range in the JSON:

```json
"time": {
  "from": "now-6h",
  "to": "now"
}
```

## Troubleshooting

### No Data Showing

1. Verify Prometheus is scraping Eunomia metrics:
   ```bash
   curl http://eunomia:9090/metrics | grep eunomia_
   ```

2. Check Prometheus targets are healthy

3. Verify the data source is correctly configured in Grafana

### Missing Metrics

Some metrics may not appear until the corresponding operation occurs:
- `rollbacks_total` - Only after a rollback
- `deployments_total` - Only after a deployment

### Dashboard Import Errors

- Ensure Grafana version is 10.0 or higher
- Check for valid JSON syntax
- Verify data source name matches your configuration

## Contributing

To update dashboards:

1. Make changes in Grafana UI
2. Export as JSON (Settings → JSON Model)
3. Save to this directory
4. Update this README if adding new panels or metrics

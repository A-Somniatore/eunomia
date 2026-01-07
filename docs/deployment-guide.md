# Eunomia Production Deployment Guide

> **Version**: 1.0.0  
> **Last Updated**: 2026-01-08

This guide covers deploying Eunomia in production environments.

---

## Table of Contents

1. [Deployment Overview](#deployment-overview)
2. [Prerequisites](#prerequisites)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Docker Compose Deployment](#docker-compose-deployment)
5. [Configuration Reference](#configuration-reference)
6. [TLS and mTLS Setup](#tls-and-mtls-setup)
7. [Health Checks](#health-checks)
8. [High Availability](#high-availability)
9. [Backup and Recovery](#backup-and-recovery)
10. [Operational Runbook](#operational-runbook)

---

## Deployment Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Production Environment                           │
│                                                                          │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐   │
│  │   OCI Registry  │     │    Eunomia      │     │   Archimedes    │   │
│  │   (Bundles)     │────▶│   Distributor   │────▶│   Instances     │   │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘   │
│          ▲                       │                        │             │
│          │                       │                        │             │
│  ┌───────┴───────┐      ┌───────┴───────┐       ┌───────┴───────┐     │
│  │   CI/CD       │      │   Metrics     │       │   Service     │     │
│  │   Pipeline    │      │   (Prometheus)│       │   Mesh        │     │
│  └───────────────┘      └───────────────┘       └───────────────┘     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Role | Scaling |
|-----------|------|---------|
| OCI Registry | Bundle storage | Managed service or replicated |
| Eunomia Distributor | Push coordination | 2-3 replicas for HA |
| Archimedes Instances | Policy evaluation | Auto-scaled per service |

---

## Prerequisites

### Required

- Kubernetes 1.24+ or Docker 20.10+
- OCI-compatible registry (e.g., Harbor, ECR, GCR)
- TLS certificates for mTLS
- SPIFFE/SPIRE or equivalent for workload identity

### Optional but Recommended

- Prometheus for metrics
- Grafana for dashboards
- External secrets operator for key management
- Service mesh (Istio/Linkerd) for mTLS

---

## Kubernetes Deployment

### Namespace Setup

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: eunomia
  labels:
    app.kubernetes.io/name: eunomia
    app.kubernetes.io/part-of: themis-platform
```

### ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: eunomia-config
  namespace: eunomia
data:
  # Distributor settings
  EUNOMIA_LOG_LEVEL: "info"
  EUNOMIA_METRICS_PORT: "9090"
  EUNOMIA_GRPC_PORT: "8080"
  
  # Registry settings
  EUNOMIA_REGISTRY_URL: "https://registry.example.com"
  
  # Distribution settings
  EUNOMIA_MAX_CONCURRENT: "5"
  EUNOMIA_TIMEOUT: "30s"
  EUNOMIA_COMPRESS: "true"
  
  # Cache settings
  EUNOMIA_CACHE_SIZE: "500MB"
  EUNOMIA_CACHE_MAX_AGE: "7d"
```

### Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: eunomia-secrets
  namespace: eunomia
type: Opaque
stringData:
  # Registry authentication
  EUNOMIA_REGISTRY_TOKEN: "your-registry-token"
  
  # Bundle signing key (base64-encoded Ed25519 private key)
  EUNOMIA_SIGNING_KEY: "base64-encoded-private-key"
```

For production, use External Secrets Operator:

```yaml
# external-secret.yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: eunomia-secrets
  namespace: eunomia
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-backend
    kind: ClusterSecretStore
  target:
    name: eunomia-secrets
    creationPolicy: Owner
  data:
  - secretKey: EUNOMIA_REGISTRY_TOKEN
    remoteRef:
      key: secret/eunomia/registry
      property: token
  - secretKey: EUNOMIA_SIGNING_KEY
    remoteRef:
      key: secret/eunomia/signing
      property: private_key
```

### TLS Secrets

```yaml
# tls-secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: eunomia-tls
  namespace: eunomia
type: kubernetes.io/tls
data:
  tls.crt: <base64-encoded-cert>
  tls.key: <base64-encoded-key>
  ca.crt: <base64-encoded-ca>
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: eunomia-distributor
  namespace: eunomia
  labels:
    app.kubernetes.io/name: eunomia-distributor
    app.kubernetes.io/component: distributor
spec:
  replicas: 2
  selector:
    matchLabels:
      app.kubernetes.io/name: eunomia-distributor
  template:
    metadata:
      labels:
        app.kubernetes.io/name: eunomia-distributor
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: eunomia-distributor
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: distributor
        image: ghcr.io/a-somniatore/eunomia:1.0.0
        imagePullPolicy: IfNotPresent
        ports:
        - name: grpc
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        envFrom:
        - configMapRef:
            name: eunomia-config
        - secretRef:
            name: eunomia-secrets
        volumeMounts:
        - name: tls
          mountPath: /etc/eunomia/tls
          readOnly: true
        - name: cache
          mountPath: /var/cache/eunomia
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          grpc:
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          grpc:
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
      volumes:
      - name: tls
        secret:
          secretName: eunomia-tls
      - name: cache
        emptyDir:
          sizeLimit: 500Mi
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchLabels:
                  app.kubernetes.io/name: eunomia-distributor
              topologyKey: kubernetes.io/hostname
```

### Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: eunomia-distributor
  namespace: eunomia
  labels:
    app.kubernetes.io/name: eunomia-distributor
spec:
  type: ClusterIP
  ports:
  - name: grpc
    port: 8080
    targetPort: grpc
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: metrics
    protocol: TCP
  selector:
    app.kubernetes.io/name: eunomia-distributor
```

### ServiceAccount and RBAC

```yaml
# rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: eunomia-distributor
  namespace: eunomia
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: eunomia-distributor
  namespace: eunomia
rules:
# For Kubernetes service discovery
- apiGroups: [""]
  resources: ["endpoints", "services"]
  verbs: ["get", "list", "watch"]
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: eunomia-distributor
  namespace: eunomia
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: eunomia-distributor
subjects:
- kind: ServiceAccount
  name: eunomia-distributor
  namespace: eunomia
```

### PodDisruptionBudget

```yaml
# pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: eunomia-distributor
  namespace: eunomia
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: eunomia-distributor
```

### HorizontalPodAutoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: eunomia-distributor
  namespace: eunomia
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: eunomia-distributor
  minReplicas: 2
  maxReplicas: 5
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Apply All Resources

```bash
# Apply in order
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml  # Or external-secret.yaml
kubectl apply -f tls-secrets.yaml
kubectl apply -f rbac.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f pdb.yaml
kubectl apply -f hpa.yaml

# Verify deployment
kubectl get pods -n eunomia
kubectl logs -l app.kubernetes.io/name=eunomia-distributor -n eunomia
```

---

## Docker Compose Deployment

For development or small deployments:

```yaml
# docker-compose.yaml
version: '3.8'

services:
  eunomia-distributor:
    image: ghcr.io/a-somniatore/eunomia:1.0.0
    container_name: eunomia-distributor
    ports:
      - "8080:8080"   # gRPC
      - "9090:9090"   # Metrics
    environment:
      EUNOMIA_LOG_LEVEL: info
      EUNOMIA_GRPC_PORT: "8080"
      EUNOMIA_METRICS_PORT: "9090"
      EUNOMIA_REGISTRY_URL: https://registry.example.com
      EUNOMIA_REGISTRY_TOKEN: ${REGISTRY_TOKEN}
      EUNOMIA_SIGNING_KEY: ${SIGNING_KEY}
      EUNOMIA_MAX_CONCURRENT: "5"
      EUNOMIA_COMPRESS: "true"
    volumes:
      - ./certs:/etc/eunomia/tls:ro
      - eunomia-cache:/var/cache/eunomia
    healthcheck:
      test: ["CMD", "grpc_health_probe", "-addr=:8080"]
      interval: 10s
      timeout: 5s
      retries: 3
    restart: unless-stopped
    networks:
      - themis-network

  # Optional: Local registry for development
  registry:
    image: registry:2
    container_name: eunomia-registry
    ports:
      - "5000:5000"
    volumes:
      - registry-data:/var/lib/registry
    networks:
      - themis-network

volumes:
  eunomia-cache:
  registry-data:

networks:
  themis-network:
    driver: bridge
```

```bash
# Start services
docker-compose up -d

# Check logs
docker-compose logs -f eunomia-distributor

# Stop services
docker-compose down
```

---

## Configuration Reference

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `EUNOMIA_LOG_LEVEL` | Log verbosity (trace, debug, info, warn, error) | `info` | No |
| `EUNOMIA_GRPC_PORT` | gRPC server port | `8080` | No |
| `EUNOMIA_METRICS_PORT` | Prometheus metrics port | `9090` | No |
| `EUNOMIA_REGISTRY_URL` | OCI registry URL | - | Yes |
| `EUNOMIA_REGISTRY_TOKEN` | Registry authentication token | - | Conditional |
| `EUNOMIA_REGISTRY_USERNAME` | Registry username (basic auth) | - | Conditional |
| `EUNOMIA_REGISTRY_PASSWORD` | Registry password (basic auth) | - | Conditional |
| `EUNOMIA_SIGNING_KEY` | Ed25519 private key (base64) | - | Yes |
| `EUNOMIA_TLS_CERT` | Path to TLS certificate | - | For TLS |
| `EUNOMIA_TLS_KEY` | Path to TLS private key | - | For TLS |
| `EUNOMIA_TLS_CA` | Path to CA certificate | - | For mTLS |
| `EUNOMIA_MAX_CONCURRENT` | Max parallel push operations | `3` | No |
| `EUNOMIA_TIMEOUT` | Default operation timeout | `30s` | No |
| `EUNOMIA_COMPRESS` | Enable bundle compression | `false` | No |
| `EUNOMIA_CACHE_SIZE` | Bundle cache size limit | `100MB` | No |
| `EUNOMIA_CACHE_MAX_AGE` | Max age for cached bundles | `7d` | No |
| `EUNOMIA_CACHE_DIR` | Cache directory path | System default | No |

### CLI Configuration File

Create `~/.eunomia/config.toml` or `/etc/eunomia/config.toml`:

```toml
[distributor]
max_concurrent = 5
timeout = "30s"
compress = true

[registry]
url = "https://registry.example.com"
# token = "..." # Prefer environment variable

[tls]
cert = "/etc/eunomia/tls/tls.crt"
key = "/etc/eunomia/tls/tls.key"
ca = "/etc/eunomia/tls/ca.crt"

[cache]
size = "500MB"
max_age = "7d"
dir = "/var/cache/eunomia"

[metrics]
enabled = true
port = 9090
```

---

## TLS and mTLS Setup

### Generate Certificates

Using `cfssl`:

```bash
# CA configuration
cat > ca-csr.json << EOF
{
  "CN": "Eunomia CA",
  "key": { "algo": "ecdsa", "size": 256 },
  "names": [{ "O": "Somniatore", "OU": "Platform" }]
}
EOF

# Generate CA
cfssl gencert -initca ca-csr.json | cfssljson -bare ca

# Server certificate
cat > server-csr.json << EOF
{
  "CN": "eunomia-distributor",
  "key": { "algo": "ecdsa", "size": 256 },
  "hosts": [
    "eunomia-distributor",
    "eunomia-distributor.eunomia.svc",
    "eunomia-distributor.eunomia.svc.cluster.local",
    "localhost",
    "127.0.0.1"
  ],
  "names": [{ "O": "Somniatore", "OU": "Eunomia" }]
}
EOF

cfssl gencert -ca=ca.pem -ca-key=ca-key.pem server-csr.json | cfssljson -bare server

# Client certificate (for CLI/CI)
cat > client-csr.json << EOF
{
  "CN": "eunomia-client",
  "key": { "algo": "ecdsa", "size": 256 },
  "names": [{ "O": "Somniatore", "OU": "CI" }]
}
EOF

cfssl gencert -ca=ca.pem -ca-key=ca-key.pem client-csr.json | cfssljson -bare client
```

### SPIFFE/SPIRE Integration

For workload identity:

```yaml
# spiffe-registration.yaml
apiVersion: spire.spiffe.io/v1alpha1
kind: ClusterSPIFFEID
metadata:
  name: eunomia-distributor
spec:
  spiffeIDTemplate: "spiffe://{{ .TrustDomain }}/ns/{{ .PodMeta.Namespace }}/sa/{{ .PodSpec.ServiceAccountName }}"
  podSelector:
    matchLabels:
      app.kubernetes.io/name: eunomia-distributor
```

---

## Health Checks

### gRPC Health Check

Eunomia implements the [gRPC Health Checking Protocol](https://github.com/grpc/grpc/blob/master/doc/health-checking.md).

```bash
# Using grpc_health_probe
grpc_health_probe -addr=localhost:8080

# Using grpcurl
grpcurl -plaintext localhost:8080 grpc.health.v1.Health/Check
```

### HTTP Health Endpoints

If HTTP health is needed, deploy a sidecar:

```yaml
- name: health-proxy
  image: envoyproxy/envoy:v1.28.0
  ports:
  - name: http-health
    containerPort: 8081
  # Configure Envoy to proxy HTTP to gRPC health
```

### Kubernetes Probes

```yaml
# gRPC native probes (K8s 1.24+)
livenessProbe:
  grpc:
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10

# Or exec-based
livenessProbe:
  exec:
    command: ["/bin/grpc_health_probe", "-addr=:8080"]
  initialDelaySeconds: 10
  periodSeconds: 10
```

---

## High Availability

### Multi-Replica Deployment

- Deploy minimum 2 replicas
- Use pod anti-affinity to spread across nodes
- Configure PodDisruptionBudget

### Multi-Region Setup

For global deployments:

```
┌─────────────────────────────────────────────────────────────────┐
│                       Global Load Balancer                       │
│                        (geo-routing)                             │
└───────────────────┬─────────────────────┬───────────────────────┘
                    │                     │
        ┌───────────┴───────────┐ ┌───────┴───────────┐
        │      US-WEST          │ │      EU-WEST      │
        │  ┌─────────────────┐  │ │  ┌─────────────┐  │
        │  │ Registry Mirror │  │ │  │  Registry   │  │
        │  └────────┬────────┘  │ │  └──────┬──────┘  │
        │           │           │ │         │         │
        │  ┌────────┴────────┐  │ │  ┌──────┴──────┐  │
        │  │   Distributor   │  │ │  │ Distributor │  │
        │  │   (2 replicas)  │  │ │  │ (2 replicas)│  │
        │  └─────────────────┘  │ │  └─────────────┘  │
        └───────────────────────┘ └───────────────────┘
```

### Disaster Recovery

1. **Registry Replication**: Use OCI registry geo-replication
2. **State Recovery**: Bundle cache can be rebuilt from registry
3. **Configuration Backup**: Store in version control

---

## Backup and Recovery

### What to Backup

| Data | Method | Frequency |
|------|--------|-----------|
| Signing Keys | External secrets manager | On rotation |
| Registry Bundles | Registry replication | Continuous |
| Configuration | Version control | On change |
| Audit Logs | Log aggregation | Continuous |

### Recovery Procedures

**Distributor Recovery:**
```bash
# 1. Restore secrets
kubectl apply -f secrets.yaml

# 2. Redeploy
kubectl rollout restart deployment/eunomia-distributor -n eunomia

# 3. Verify
kubectl get pods -n eunomia
eunomia status --endpoints http://eunomia-distributor.eunomia:8080
```

**Cache Rebuild:**
```bash
# Cache is automatically rebuilt on cache miss
# Or manually prime cache:
eunomia fetch users-service --version latest
eunomia fetch orders-service --version latest
```

---

## Operational Runbook

### Common Operations

**Deploy New Policy Version:**
```bash
# 1. Build and sign bundle
eunomia build --policy-dir policies/ --version 1.2.0 --output bundle.tar.gz
eunomia sign bundle.tar.gz --key-file private.key

# 2. Publish to registry
eunomia publish bundle.tar.gz --service users-service

# 3. Push to instances (canary first)
eunomia push bundle.tar.gz --strategy canary --canary-percent 10

# 4. If healthy, complete rollout
eunomia push bundle.tar.gz --strategy immediate
```

**Emergency Rollback:**
```bash
# Immediate rollback to previous version
eunomia rollback --service users-service --force

# Rollback to specific version
eunomia rollback --service users-service --version 1.1.0
```

**Check Deployment Status:**
```bash
# Status of all services
eunomia status

# Status with instance details
eunomia status --service users-service --verbose

# JSON output for automation
eunomia status --format json
```

### Alerts and Escalation

| Alert | Severity | Action |
|-------|----------|--------|
| Push failure rate > 10% | Warning | Check instance health |
| Push failure rate > 50% | Critical | Pause deployments, investigate |
| All instances unhealthy | Critical | Trigger rollback |
| Registry unreachable | Warning | Use cached bundles |
| Certificate expiring < 7d | Warning | Rotate certificates |

### Maintenance Windows

```bash
# 1. Scale down to single replica
kubectl scale deployment/eunomia-distributor --replicas=1 -n eunomia

# 2. Perform maintenance
# ...

# 3. Scale back up
kubectl scale deployment/eunomia-distributor --replicas=2 -n eunomia

# 4. Verify health
kubectl get pods -n eunomia
```

---

## Appendix: Complete Kubernetes Manifests

All manifests in a single file for GitOps:

```bash
# Download complete manifests
curl -O https://raw.githubusercontent.com/A-Somniatore/eunomia/main/deploy/kubernetes/all-in-one.yaml

# Apply with kustomize
kubectl apply -k deploy/kubernetes/overlays/production
```

### Directory Structure

```
deploy/
├── kubernetes/
│   ├── base/
│   │   ├── kustomization.yaml
│   │   ├── namespace.yaml
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── rbac.yaml
│   └── overlays/
│       ├── development/
│       │   └── kustomization.yaml
│       ├── staging/
│       │   └── kustomization.yaml
│       └── production/
│           ├── kustomization.yaml
│           ├── hpa.yaml
│           └── pdb.yaml
└── docker-compose/
    ├── docker-compose.yaml
    └── .env.example
```

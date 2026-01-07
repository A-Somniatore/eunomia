# Eunomia Troubleshooting Guide

> **Version**: 1.0.0  
> **Last Updated**: 2026-01-08

This guide covers common issues, debugging techniques, and resolution steps for Eunomia.

---

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Policy Validation Errors](#policy-validation-errors)
3. [Bundle Compilation Issues](#bundle-compilation-issues)
4. [Registry Connection Problems](#registry-connection-problems)
5. [Distribution Failures](#distribution-failures)
6. [mTLS and Authentication](#mtls-and-authentication)
7. [Performance Issues](#performance-issues)
8. [Rollback Problems](#rollback-problems)
9. [Debugging Techniques](#debugging-techniques)
10. [Getting Help](#getting-help)

---

## Quick Diagnostics

### Health Check Commands

```bash
# Check CLI version and dependencies
eunomia --version

# Validate policies in a directory
eunomia validate policies/ --verbose

# Test policy suite
eunomia test policies/ --verbose

# Check deployment status
eunomia status --endpoints http://localhost:8080

# Verify bundle integrity
eunomia verify bundle.tar.gz --public-key key.pub
```

### Common Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EUNOMIA_LOG_LEVEL` | Logging verbosity | `info` |
| `EUNOMIA_SIGNING_KEY` | Ed25519 private key (base64) | None |
| `EUNOMIA_REGISTRY_URL` | OCI registry URL | None |
| `EUNOMIA_REGISTRY_TOKEN` | Registry auth token | None |
| `RUST_BACKTRACE` | Enable backtraces | `0` |

---

## Policy Validation Errors

### Error: "default allow := false not found"

**Symptom:**
```
error[security/default-deny]: Policy must have 'default allow := false'
  --> policies/my-service/authz.rego:1:1
```

**Cause:** All authorization policies must explicitly default to deny.

**Solution:**
```rego
package my_service.authz

# Add this at the top of your policy
default allow := false

allow if {
    # your rules here
}
```

### Error: "Unknown operation_id referenced"

**Symptom:**
```
warning[semantic/unknown-operation]: Unknown operation_id 'deleteAllUsers'
  --> policies/users-service/authz.rego:45:5
   |
45 |     input.operation_id == "deleteAllUsers"
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Valid operations: getUser, createUser, updateUser, deleteUser
```

**Cause:** The policy references an operation that doesn't exist in the service contract.

**Solution:**
1. Check the service contract for valid operation IDs
2. Fix typos in the operation name
3. If the operation is new, ensure the contract is updated first

### Error: "Hardcoded secret detected"

**Symptom:**
```
error[security/no-hardcoded-secrets]: Hardcoded secret detected
  --> policies/api/authz.rego:12:5
   |
12 |     input.headers["x-api-key"] == "sk_live_abc123"
```

**Cause:** Policy contains hardcoded credentials.

**Solution:**
```rego
# BAD - hardcoded secret
allow if {
    input.headers["x-api-key"] == "sk_live_abc123"
}

# GOOD - reference external data
allow if {
    valid_api_key(input.headers["x-api-key"])
}

valid_api_key(key) if {
    key in data.valid_api_keys
}
```

### Error: "Syntax error in Rego file"

**Symptom:**
```
error: failed to parse policy
  --> policies/service/authz.rego:25:10
   |
25 |     allow if
   |          ^^ expected '{'
```

**Cause:** Invalid Rego syntax.

**Solution:**
1. Check for missing braces `{}`
2. Ensure `if` keyword is followed by a block
3. Validate imports are at the top of the file
4. Use `import future.keywords.if` for modern syntax

---

## Bundle Compilation Issues

### Error: "No policies found in directory"

**Symptom:**
```
error: no policies found in 'policies/'
```

**Cause:** Directory doesn't contain `.rego` files or path is incorrect.

**Solution:**
```bash
# Check directory contents
ls -la policies/

# Ensure .rego files exist
find policies/ -name "*.rego"

# Use correct path
eunomia build --policy-dir ./policies/users-service
```

### Error: "Bundle checksum mismatch"

**Symptom:**
```
error: bundle checksum mismatch
  expected: sha256:abc123...
  actual:   sha256:def456...
```

**Cause:** Bundle was modified after signing or corrupted during transfer.

**Solution:**
1. Re-download the bundle
2. Re-sign the bundle: `eunomia sign bundle.tar.gz --key-file private.key`
3. Verify network transfer integrity

### Error: "Invalid bundle format"

**Symptom:**
```
error: invalid bundle format: missing .manifest file
```

**Cause:** Bundle is corrupted or not in OPA bundle format.

**Solution:**
```bash
# Verify bundle structure
tar -tzf bundle.tar.gz | head -20

# Expected output should include:
# .manifest
# policies/.../*.rego
```

---

## Registry Connection Problems

### Error: "Failed to connect to registry"

**Symptom:**
```
error: failed to connect to registry at https://registry.example.com
  caused by: connection refused
```

**Cause:** Registry is unreachable or URL is incorrect.

**Solution:**
```bash
# Test connectivity
curl -v https://registry.example.com/v2/

# Check DNS resolution
nslookup registry.example.com

# Verify firewall rules
nc -zv registry.example.com 443
```

### Error: "Authentication failed"

**Symptom:**
```
error: registry authentication failed: 401 Unauthorized
```

**Cause:** Invalid or expired credentials.

**Solution:**
```bash
# Using token authentication
eunomia publish bundle.tar.gz \
  --registry https://registry.example.com \
  --token "$REGISTRY_TOKEN"

# Using basic auth
eunomia publish bundle.tar.gz \
  --registry https://registry.example.com \
  --username admin \
  --password "$REGISTRY_PASSWORD"

# Check token expiration
echo $REGISTRY_TOKEN | jwt decode -
```

### Error: "TLS certificate verification failed"

**Symptom:**
```
error: TLS certificate verification failed
  caused by: certificate has expired
```

**Cause:** Invalid or expired TLS certificate.

**Solution:**
```bash
# Check certificate
openssl s_client -connect registry.example.com:443 -servername registry.example.com

# For development, skip verification (NOT for production)
eunomia publish bundle.tar.gz --insecure

# Update CA certificates
sudo update-ca-certificates
```

---

## Distribution Failures

### Error: "No instances discovered"

**Symptom:**
```
error: no Archimedes instances discovered for service 'users-service'
```

**Cause:** No instances registered or discovery mechanism not configured.

**Solution:**
```bash
# Use static endpoints
eunomia push bundle.tar.gz \
  --endpoints http://archimedes-1:8080,http://archimedes-2:8080

# Check DNS discovery
dig _archimedes._tcp.users-service.svc.cluster.local SRV

# Verify Kubernetes service exists
kubectl get svc users-service -n production
```

### Error: "Push failed: connection refused"

**Symptom:**
```
error: failed to push to http://archimedes-1:8080
  caused by: connection refused
```

**Cause:** Archimedes instance is down or not accepting connections.

**Solution:**
```bash
# Check instance health
curl http://archimedes-1:8080/health

# Verify port is open
nc -zv archimedes-1 8080

# Check instance logs
kubectl logs -l app=archimedes -n production
```

### Error: "Push timeout exceeded"

**Symptom:**
```
error: push to http://archimedes-1:8080 timed out after 30s
```

**Cause:** Network latency or large bundle size.

**Solution:**
```bash
# Increase timeout
eunomia push bundle.tar.gz --timeout 60s

# Check bundle size
ls -lh bundle.tar.gz

# Enable compression
eunomia push bundle.tar.gz --compress
```

### Error: "Partial deployment failure"

**Symptom:**
```
error: deployment partially failed
  successful: 3/5 instances
  failed: archimedes-4, archimedes-5
```

**Cause:** Some instances are unhealthy or unreachable.

**Solution:**
```bash
# Check failed instance status
eunomia status --endpoints http://archimedes-4:8080,http://archimedes-5:8080

# Retry failed instances only
eunomia push bundle.tar.gz \
  --endpoints http://archimedes-4:8080,http://archimedes-5:8080

# Use rolling deployment to limit blast radius
eunomia push bundle.tar.gz --strategy rolling --batch-size 2
```

---

## mTLS and Authentication

### Error: "mTLS handshake failed"

**Symptom:**
```
error: mTLS handshake failed
  caused by: certificate not trusted
```

**Cause:** Client certificate not signed by trusted CA.

**Solution:**
```bash
# Verify certificate chain
openssl verify -CAfile ca.crt client.crt

# Check certificate details
openssl x509 -in client.crt -text -noout

# Ensure correct files are specified
eunomia push bundle.tar.gz \
  --ca-cert ca.crt \
  --client-cert client.crt \
  --client-key client.key
```

### Error: "SPIFFE identity not allowed"

**Symptom:**
```
error: SPIFFE identity not in allowlist
  identity: spiffe://example.com/eunomia/control-plane
  allowed: spiffe://example.com/eunomia/distributor
```

**Cause:** SPIFFE ID doesn't match Archimedes allowlist.

**Solution:**
1. Check Archimedes configuration for allowed SPIFFE IDs
2. Ensure control plane uses correct SPIFFE identity
3. Update allowlist if identity is legitimate

### Error: "Certificate expired"

**Symptom:**
```
error: client certificate has expired
  expired: 2026-01-01T00:00:00Z
  current: 2026-01-08T12:00:00Z
```

**Cause:** Client certificate has expired.

**Solution:**
1. Renew certificate from your CA
2. Update mounted secrets in Kubernetes
3. Restart Eunomia with new certificate

---

## Performance Issues

### Slow Policy Evaluation

**Symptom:** Policy tests or validation taking too long.

**Diagnosis:**
```bash
# Run with timing
time eunomia test policies/ --verbose

# Profile specific test
RUST_LOG=eunomia_test=debug eunomia test policies/
```

**Solutions:**
1. Reduce policy complexity
2. Use `with` keyword for mock data instead of large data files
3. Avoid recursive rules without bounds
4. Use indexing for large datasets

### High Memory Usage

**Symptom:** Eunomia consuming excessive memory.

**Diagnosis:**
```bash
# Monitor memory
watch -n 1 'ps aux | grep eunomia'
```

**Solutions:**
1. Reduce bundle cache size
2. Process policies in smaller batches
3. Avoid loading large data files into memory

### Slow Bundle Distribution

**Symptom:** Push operations taking too long.

**Solutions:**
```bash
# Enable compression
eunomia push bundle.tar.gz --compress

# Increase parallelism
eunomia push bundle.tar.gz --max-concurrent 10

# Use canary deployment for faster validation
eunomia push bundle.tar.gz --strategy canary --canary-percent 10
```

---

## Rollback Problems

### Error: "No previous version available"

**Symptom:**
```
error: cannot rollback: no previous version available for service 'users-service'
```

**Cause:** This is the first deployment or history was cleared.

**Solution:**
```bash
# Check version history
eunomia status --service users-service --verbose

# Deploy a known-good version explicitly
eunomia push known-good-bundle.tar.gz --force
```

### Error: "Rollback failed: health check timeout"

**Symptom:**
```
error: rollback failed: health check did not pass within 60s
```

**Cause:** Previous version also has issues.

**Solution:**
```bash
# Force immediate rollback
eunomia rollback --service users-service --force

# Rollback to specific version
eunomia rollback --service users-service --version 1.0.0

# Check what version was rolled back to
eunomia status --service users-service
```

### Error: "Automatic rollback triggered"

**Symptom:**
```
warning: automatic rollback triggered due to health check failures
  failed instances: 3/5 (threshold: 50%)
```

**Cause:** New policy caused health check failures.

**Solution:**
1. Review policy changes that caused failures
2. Check Archimedes logs for authorization errors
3. Test policy more thoroughly before deployment
4. Consider using canary deployments

---

## Debugging Techniques

### Enable Verbose Logging

```bash
# CLI verbose mode
eunomia test policies/ -vvv

# Environment variable
RUST_LOG=debug eunomia push bundle.tar.gz

# Specific module logging
RUST_LOG=eunomia_distributor=trace,eunomia_core=debug eunomia push bundle.tar.gz
```

### Inspect Bundle Contents

```bash
# List bundle contents
tar -tzf bundle.tar.gz

# Extract and inspect manifest
tar -xzf bundle.tar.gz -O .manifest | jq .

# View policy content
tar -xzf bundle.tar.gz -O policies/users-service/authz.rego
```

### Test Policy Evaluation

```bash
# Create test input file
cat > test-input.json << 'EOF'
{
  "caller": {
    "type": "user",
    "user_id": "user-123",
    "roles": ["admin"]
  },
  "service": "users-service",
  "operation_id": "deleteUser",
  "method": "DELETE",
  "path": "/users/456"
}
EOF

# Evaluate policy with test input
eunomia eval policies/users-service/authz.rego \
  --input test-input.json \
  --query "data.users_service.authz.allow"
```

### Network Debugging

```bash
# Test gRPC connectivity
grpcurl -plaintext localhost:8080 list

# Check TLS certificate
openssl s_client -connect archimedes:8080 -servername archimedes

# Capture traffic (development only)
tcpdump -i any port 8080 -w capture.pcap
```

### Policy Debugging with Tracing

```rego
# Add print statements for debugging
allow if {
    print("Checking user:", input.caller.user_id)
    print("Roles:", input.caller.roles)
    is_admin
    print("User is admin, allowing")
}

is_admin if {
    print("Checking admin role in:", input.caller.roles)
    "admin" in input.caller.roles
}
```

---

## Getting Help

### Collect Diagnostic Information

Before reporting an issue, collect:

```bash
# System information
uname -a
eunomia --version
rustc --version

# Reproduce with debug logging
RUST_LOG=debug RUST_BACKTRACE=1 eunomia <command> 2>&1 | tee debug.log

# Policy directory structure
find policies/ -type f -name "*.rego" | head -20

# Bundle information (if applicable)
tar -tzf bundle.tar.gz
```

### Support Resources

- **GitHub Issues**: [github.com/A-Somniatore/eunomia/issues](https://github.com/A-Somniatore/eunomia/issues)
- **Documentation**: [docs/](./docs/)
- **Design Document**: [docs/design.md](./design.md)
- **Specification**: [docs/spec.md](./spec.md)

### Filing a Bug Report

Include:
1. Eunomia version (`eunomia --version`)
2. Operating system and version
3. Steps to reproduce
4. Expected vs actual behavior
5. Debug logs (`RUST_LOG=debug`)
6. Minimal policy example (if applicable)

---

## Common Error Codes

| Code | Description | Resolution |
|------|-------------|------------|
| `E001` | Policy syntax error | Fix Rego syntax |
| `E002` | Validation failed | Address lint warnings |
| `E003` | Bundle creation failed | Check policy directory |
| `E004` | Signing failed | Verify key file |
| `E005` | Registry connection failed | Check network/auth |
| `E006` | Push failed | Check instance health |
| `E007` | Rollback failed | Verify version history |
| `E008` | mTLS handshake failed | Check certificates |
| `E009` | Health check failed | Review policy changes |
| `E010` | Timeout exceeded | Increase timeout/retry |

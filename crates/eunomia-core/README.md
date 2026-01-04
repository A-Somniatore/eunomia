# Eunomia Core

Core types and traits for the Eunomia authorization policy platform.

## Overview

This crate provides the foundational data structures used throughout Eunomia:

- **Policy** - Represents a Rego policy file with metadata
- **Bundle** - Compiled policy bundle for distribution to Archimedes
- **AuthorizationDecision** - The result of evaluating a policy
- **PolicyInput** - Input schema for authorization requests
- **CallerIdentity** - Caller identity types (SPIFFE, User, ApiKey, Anonymous)

## Usage

```rust
use eunomia_core::{PolicyInput, CallerIdentity};

let input = PolicyInput::builder()
    .caller(CallerIdentity::user("user-123", vec!["viewer".to_string()]))
    .service("users-service")
    .operation_id("getUser")
    .method("GET")
    .path("/users/user-123")
    .build();
```

## License

MIT OR Apache-2.0

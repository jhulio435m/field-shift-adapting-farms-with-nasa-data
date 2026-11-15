# ADR 0001: Rust modular monolith

Status: accepted.

Use Rust for a single deployable service organized by hexagonal boundaries. Axum serves the HTTP adapter; SQLx implements the PostgreSQL adapter. Contracts, rather than backend language, provide interoperability with customer systems. This is sufficient for the expected integration workload and avoids operating distributed services before there is evidence they are needed.

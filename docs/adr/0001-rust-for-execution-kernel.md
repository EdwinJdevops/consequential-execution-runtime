# ADR 0001: Rust for the execution kernel
Status: Accepted for M0

## Context
CER needs explicit state modeling, exhaustive matching, safe concurrency foundations, and a small trusted implementation surface.

## Decision
Use Rust for the kernel and adapter contract. This is revisitable if implementation cost prevents semantic validation.

## Alternatives
Go offers simpler operations and concurrency but weaker type-level state modeling. Python accelerates experiments but is not selected for the safety-critical kernel.

## Consequences
Rust expertise and compile times are costs. Python/TypeScript SDKs may be added later without moving hard safety authority into those SDKs.

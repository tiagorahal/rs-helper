## README.md

**rs-helper** is a lightweight **Rust** backend that aggregates and normalizes data from RuneScape and related wiki endpoints. Many official endpoints do not expose browser-friendly CORS headers; this project provides **CORS-safe** routes for use by web frontends and other clients.

**Status:** early planning / pre-alpha

## Goals (initial scope)

* **CORS-safe proxy/gateway** for RuneScape & wiki data
* **Response normalization** (consistent shapes, types, and error handling)
* **Caching** and basic **rate limiting** to protect upstreams
* **Feature flags** for enabling specific data sources incrementally
* **Front-end friendly pagination & filtering** on top of upstream endpoints

## Non-Goals (for now)

* Public hosting or multi-tenant service
* UI/visualization (may add an optional admin page later)
* Heavy data warehousing or long-term persistence

## Why

* Official APIs often don’t include the CORS headers needed for direct browser calls.
* A thin backend avoids CORS issues and centralizes retries, backoff, and schema normalization.
* A Chrome extension can sometimes bypass CORS, but a backend is generally more portable and reliable.

## High-Level Design

* **Rust service** that forwards, validates, and reshapes responses from game/wiki endpoints
* Central **error model** and **observability hooks** (logging/metrics)
* Pluggable **cache** layer with sensible TTLs
* Guardrails: input validation, timeouts, upstream circuit breakers

## Data Sources (planned)

* Game services (player stats, hiscores, etc.)
* Wiki endpoints (item metadata, search, pages)
* Optional: price/market data where available

> Exact sources will be added gradually and feature-flagged.

## Reliability & Safety

* Respect upstream **rate limits** and terms
* Do not store sensitive data
* Clear **user-agent** and contact metadata when required by upstreams

## Legal

* **Unofficial project.** Not affiliated with Jagex or RuneScape.
* Follow respective API and wiki usage policies.

## Roadmap (short)

* Define minimal route set and normalized schemas
* Add cache + rate limiting
* Implement retry/backoff policies
* Publish example response contracts
* Optional admin diagnostics page (later)

## Contributing

* Open an issue describing the endpoint you need and proposed response shape.
* Keep PRs small and focused on one upstream/source at a time.

## License

**MIT** — free for commercial use; attribution appreciated.

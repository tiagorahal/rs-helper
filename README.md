
## Introduction

Welcome to **rs-helper** — a tiny, fast 🦀 **Rust** gateway that fetches and **normalizes** RuneScape data while keeping your front-end happy with **CORS-safe** routes.
No more wrestling with mixed response shapes or browser CORS errors — just clean JSON and a minimal UI to browse items quickly.

**Status:** pre-alpha (early days, sharp edges, lots of enthusiasm)

---

## Features

* 🔓 **CORS-friendly API**: call from the browser without hacks.
* 🧼 **Normalized responses**: stable shapes, typed fields, clean errors.
* 🧭 **RS3 + OSRS**: choose your game data source.
* 🔤 **Letter (“alpha”) filter**: mirrors the official GE partition (a–z, or `#` for numeric).
* 💬 **Tiny UI** (server-rendered): search + category + letter, powered by Askama + htmx.
* 🔐 **No OpenSSL required**: `reqwest` + **rustls** under the hood.

---

## Endpoints (current)

* `GET /v1/ge/info`
* `GET /v1/ge/categories`
* `GET /v1/ge/items?category=&alpha=&page=&game=rs3|osrs`
* `GET /v1/ge/items/{id}?game=rs3|osrs`
* `GET /v1/ge/graph/{id}?game=rs3|osrs`

**Query tips**

| param      | values          | notes                                       |
| ---------- | --------------- | ------------------------------------------- |
| `game`     | `rs3` \| `osrs` | defaults to `rs3`                           |
| `category` | integer         | RS3 uses 0..43; OSRS uses `1` (“All items”) |
| `alpha`    | `a`…`z` or `#`  | `#` = items that start with a number        |
| `page`     | `1..N`          | upstream paging                             |

**Error envelope**

```json
{ "code": "upstream_error", "message": "Upstream request failed: ..." }
```

---

## Quick Start

### Prerequisites

* 🦀 **Rust 1.82+** (toolchain recommended)
* No system OpenSSL needed

### Run

```bash
cargo build
cargo run
```

Open the UI: `http://127.0.0.1:3000/`
Or hit JSON directly:

```bash
curl 'http://127.0.0.1:3000/v1/ge/items?category=9&alpha=c&page=1'
```

---

## How the UI Works

* **Search**: substring match on item name (server-side filter).
* **Category**: pick a GE category.
* **Letter (alpha)**: official first-letter partition; `#` covers numeric-leading names.
* Clean, server-rendered HTML (Askama) + partial updates (htmx). No custom JS needed.

---

## Why rs-helper?

* Many official endpoints **don’t ship CORS headers** → browsers complain.
* A thin Rust service is **portable, fast, and reliable**.
* Central place for **normalization**, **timeouts**, **retries**, and future **caching**.

---

## Design Notes

* Web: **Axum** + **tower-http** (CORS & tracing)
* Templates: **Askama** (+ **htmx** for partials)
* HTTP client: **reqwest** with **rustls** (no OpenSSL)
* Normalization: prices like `"17.0m"` → integers; times → ISO-8601

---

## Roadmap (short)

* HiScores Lite (RS3 & OSRS)
* RuneMetrics (profile, monthly XP, quests)
* Cache (TTL) + basic rate limiting
* Better error taxonomy & docs
* Optional admin/diagnostics page

---

## Contributing

* Open an issue describing the endpoint you need and the desired response shape.
* Keep PRs focused and incremental (one upstream/source at a time).

---

## Legal

**Unofficial project.** Not affiliated with Jagex or RuneScape.
Please follow the relevant API and wiki usage policies.

---

## License

**MIT** — use it, fork it, ship it. Attribution appreciated! ✨

# Gemini Cursor Proxy (Rust on Cloudflare Workers)

A production-grade, high-performance, OpenAI-compatible API Gateway built with **Rust** running at the edge on **Cloudflare Workers** (`workers-rs`). It acts as a resilient proxy between **Cursor IDE** and the **Google Gemini API** (powered by `gemini-3.8-flash` with native thinking/reasoning).

Designed from the ground up to be completely safe for public GitHub hosting, featuring multi-account scheduling with quota domain awareness, first-byte safe retries, and zero-buffering Server-Sent Events (SSE) streaming.

---

## Architecture Overview

```
Cursor IDE (Client)
      │
      │ HTTPS (POST /v1/chat/completions, GET /v1/models)
      │ Header: Authorization: Bearer <PROXY_TOKEN>
      ▼
Cloudflare Worker (Edge / Rust `workers-rs`)
      │
      ├── 1. Security & Auth Guard
      │      - Validates Bearer token using constant-time comparison
      │
      ├── 2. Body Parser & Normalizer
      │      - Enforces max_body_bytes (10MB limit)
      │      - Resolves model aliases to `gemini-3.8-flash`
      │      - Injects native `reasoning_effort: "high"`
      │
      ├── 3. Account & Quota Scheduler (LRU + Quota Domain)
      │      - Filters out cooling-down accounts
      │      - Isolates quota domains (shared projects)
      │      - Selects healthy candidate using Least-Recently-Used
      │
      ├── 4. Upstream Gateway (Google Gemini OpenAI Endpoint)
      │      - URL locked to `https://generativelanguage.googleapis.com/v1beta/openai`
      │      - Injects resolved GEMINI_KEY from Cloudflare Secrets
      │      - Strips sensitive headers
      │
      ├── 5. First-Byte Aware Retry Loop
      │      - Retries retryable errors (429, 5xx, network) before first byte
      │      - Strictly prevents replay if `first_byte_sent == true`
      │
      └── 6. End-to-End SSE Streaming
             - Direct streaming to Cursor without buffering into memory
```

---

## Why Rust & Why Cloudflare Workers?

1. **Ultra-low Latency & Memory Footprint**: Rust compiles to WebAssembly (`wasm32-unknown-unknown`), running inside Cloudflare V8 isolates with near-zero cold starts and tiny memory usage (<15MB), staying well beneath the 128MB limit.
2. **Global Edge Distribution**: Requests from Cursor hit the nearest Cloudflare PoP (Point of Presence) globally, terminating TLS and proxying directly to Google's backbone network.
3. **Memory Safety & Concurrency**: Type-safe concurrency via `std::sync::RwLock` eliminates race conditions and null-pointer exceptions without a garbage collection runtime.

---

## Model & Native Thinking Configuration

- **Primary Model**: `gemini-3.8-flash` (GA September 2026, optimized for autonomous coding, architecture, and multi-file tasks).
- **Native Reasoning**: Mapped directly via `reasoning_effort: "high"` on Google's OpenAI-compatible endpoint. No artificial prompting tricks (e.g. "think step by step") are injected.
- **Model Alias Policy**:
  - `auto` ➔ `gemini-3.8-flash`
  - `gpt-4o` ➔ `gemini-3.8-flash`
  - `gpt-4.1` ➔ `gemini-3.8-flash`
  - `gemini-3.8-flash` ➔ `gemini-3.8-flash`

---

## Secret Separation Architecture (Public Repo Safe)

This repository is designed to be hosted publicly on GitHub without any risk of credential leakage. Three strict tiers of secrets are maintained:

| Secret Tier | Location | Contents | Access / Scope |
|-------------|----------|----------|----------------|
| **Tier A: Public Repo** | GitHub Public Repo | Source code, `wrangler.toml` (vars only), `.env.example`, CI/CD workflows | Visible to everyone |
| **Tier B: CI/CD Secrets** | GitHub Actions Secrets | `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` | Used solely for deploying worker |
| **Tier C: Runtime Secrets** | Cloudflare Worker Secrets | `PROXY_TOKEN`, `GEMINI_KEY_01` .. `GEMINI_KEY_05` | Only accessible inside Worker runtime |

> [!IMPORTANT]
> Google Gemini API keys and Proxy tokens are NEVER stored in GitHub Actions, NEVER injected into build binaries, and NEVER committed to git.

---

## Setup & Deployment Guide

### 1. Configure Cloudflare Secrets

Run the following commands using the Cloudflare Wrangler CLI (or configure them in the Cloudflare Dashboard under **Workers & Pages > gemini-cursor-proxy > Settings > Variables > Secrets**):

#### Option A: Bulk Keys Pool (Recommended for 10 - 1,000+ API Keys)
Instead of creating dozens or hundreds of individual variables, put all your keys into a single secret:
```bash
# 1. Set your custom proxy authentication token
npx wrangler secret put PROXY_TOKEN

# 2. Put 10, 100, or 1,000+ Gemini API keys into GEMINI_KEYS_POOL
# Supported formats: newline-separated, comma-separated, or JSON array
npx wrangler secret put GEMINI_KEYS_POOL
```

#### Option B: Individual API Keys (Up to 20 keys with auto-discovery)
```bash
# 1. Set your custom proxy authentication token
npx wrangler secret put PROXY_TOKEN

# 2. Set individual Google Gemini API keys (automatically auto-detected by Worker)
npx wrangler secret put GEMINI_KEY_01
npx wrangler secret put GEMINI_KEY_02
npx wrangler secret put GEMINI_KEY_03
# ... up to GEMINI_KEY_20 without any code or config edits
```

### 2. Configure GitHub Secrets for CI/CD Deployment

In your GitHub repository, go to **Settings > Secrets and variables > Actions** and add:
1. `CLOUDFLARE_API_TOKEN`: Cloudflare API Token with `Edit Cloudflare Workers` permission.
2. `CLOUDFLARE_ACCOUNT_ID`: Your Cloudflare Account ID (found on the right sidebar of the Cloudflare Workers dashboard).

### 3. Deploy via GitHub Actions

Because your local machine may not have the Rust WebAssembly toolchain installed, deployment runs automatically via GitHub Actions:

```bash
git add .
git commit -m "feat: setup gemini cursor proxy"
git push origin main
```

The GitHub Actions workflow `.github/workflows/deploy.yml` will automatically:
1. Validate formatting (`cargo fmt`)
2. Run test suites (`cargo test`)
3. Compile the Rust WebAssembly binary using `worker-build`
4. Deploy to your Cloudflare account

---

## Configuring Cursor IDE

Open Cursor IDE and navigate to **Settings > Cursor Settings > Models**:

1. **OpenAI API Key**: Enter your `PROXY_TOKEN` (the secret you set in Cloudflare, **NOT** your Gemini key).
2. **Override OpenAI Base URL**: Set to your Cloudflare Worker URL:
   ```
   https://gemini-cursor-proxy.<your-subdomain>.workers.dev/v1
   ```
   *(Or your custom domain e.g. `https://api.yourdomain.com/v1`)*
3. **Model**: Set to `gemini-3.8-flash` (or leave as `gpt-4o` / `auto` since the proxy automatically normalizes aliases).

---

## Verifying the Proxy

### 1. Test Model List (`GET /v1/models`)

```bash
curl https://gemini-cursor-proxy.<your-subdomain>.workers.dev/v1/models \
  -H "Authorization: Bearer YOUR_PROXY_TOKEN"
```

Expected response:
```json
{
  "object": "list",
  "data": [
    {
      "id": "gemini-3.8-flash",
      "object": "model",
      "created": 1726000000,
      "owned_by": "google"
    }
  ]
}
```

### 2. Test Non-Streaming Chat Completion (`POST /v1/chat/completions`)

```bash
curl https://gemini-cursor-proxy.<your-subdomain>.workers.dev/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_PROXY_TOKEN" \
  -d '{
    "model": "gemini-3.8-flash",
    "messages": [
      {"role": "user", "content": "Explain rust ownership in one sentence."}
    ],
    "stream": false
  }'
```

### 3. Test Streaming Chat Completion (SSE)

```bash
curl -N https://gemini-cursor-proxy.<your-subdomain>.workers.dev/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_PROXY_TOKEN" \
  -d '{
    "model": "gemini-3.8-flash",
    "messages": [
      {"role": "user", "content": "Write a quick rust test."}
    ],
    "stream": true
  }'
```

---

## Adding, Disabling, or Modifying Accounts

### Adding Account `P06` without Changing Rust Code:

1. Add the secret to Cloudflare:
   ```bash
   npx wrangler secret put GEMINI_KEY_06
   ```
2. In `wrangler.toml` (or Cloudflare Environment Variable `ACCOUNTS_CONFIG_TOML`), add the account declaration:
   ```toml
   [[accounts]]
   id = "p06"
   secret_name = "GEMINI_KEY_06"
   quota_domain = "project-f"
   enabled = true
   ```
3. Commit and push to GitHub. The scheduler will automatically pick up `p06` without any modifications to the Rust router or core scheduler logic.

### Disabling an Account:
Simply set `enabled = false` for that account in the configuration.

---

## First-Byte Semantics & Safe Streaming

The gateway implements a strict state machine to prevent response corruption in Cursor:

```
[Request Started] ──► [Upstream Headers Received] ──► [First Byte Sent] ──► [Streaming Active]
       │                          │                            │
       ▼                          ▼                            ▼
  Retry Allowed              Retry Allowed             RETRY STRICTLY FORBIDDEN
  (Switch Account)           (Switch Account)          (Safely close stream on abort)
```

- **Pre-first-byte**: If Gemini returns a 429, 500, 502, 503, or connection drop, the gateway automatically marks that account/domain in cooldown and retries with another eligible account up to 3 attempts.
- **Post-first-byte**: Once the first byte has been dispatched to Cursor, the gateway will **NEVER** replay or retry the request. Replaying a request mid-stream would send duplicated JSON chunks and corrupt Cursor's editor state.

---

## Quota Domain Awareness

A single Google Cloud project may have multiple API keys, but they share the same rate-limit quota bucket.

In `gemini-cursor-proxy`, accounts declare a `quota_domain`:
- If `p01` and `p02` both belong to `project-a`:
- When `p01` receives a `429 Too Many Requests`, the scheduler cools down the **entire `project-a` domain**.
- The next request will skip `p02` immediately and route to `project-b` (e.g. `p03`), preventing wasted attempts against an already-throttled quota bucket.

---

## Smart Router & Task Classification (Active)

The proxy features an integrated, high-speed **Smart Router** driven by **`gemini-3.5-flash-lite`**:
- **EASY Tasks** (typos, formatting, simple renames, small questions): Routed to `gemini-3.5-flash-lite` without extra reasoning overhead for lightning-fast latency and maximum quota efficiency.
- **NORMAL Tasks** (standard feature implementation, bug fixes): Routed to `gemini-3.8-flash` with medium reasoning effort.
- **HARD Tasks** (architecture design, multi-file refactoring, autonomous agentic workflows): Routed to `gemini-3.8-flash` with native `reasoning_effort = "high"`.
- **Zero-risk Fallback**: If classification is ambiguous or fails, the router automatically defaults safely to `gemini-3.8-flash (thinking high)`.
- Can be configured or toggled via `SMART_ROUTER_ENABLED = "true"` / `"false"` in `wrangler.toml`.

---

## Troubleshooting

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| `401 Unauthorized` | Invalid or missing Bearer token in Cursor | Ensure Cursor's OpenAI API Key matches `PROXY_TOKEN`. |
| `503 All upstream accounts exhausted` | All configured Gemini keys hit 429 or are in cooldown | Add more Gemini projects or wait for cooldown period (default 30s) to expire. |
| `502 Upstream Error` | Cloudflare Secret for an account is missing | Check that `GEMINI_KEY_01` .. `GEMINI_KEY_05` are set in Cloudflare Secrets. |
| Stream drops mid-flight | Network timeout or Cloudflare wall-clock limit reached | Cursor will safely receive the completed chunk; no duplicate stream is replayed. |

---

## Rollback Procedure

If an unintended regression occurs, roll back instantly using Cloudflare's built-in deployment history:
1. In Cloudflare Dashboard, go to **Workers & Pages > gemini-cursor-proxy > Deployments**.
2. Select the previous stable deployment and click **Rollback**.
3. Alternatively, run `git revert HEAD` and push to `main` to let GitHub Actions deploy the previous commit.

# Gemini OpenAI Gateway (Rust on Cloudflare Workers)

A production-grade, ultra-low-latency, universal **OpenAI-compatible API Gateway** built in **Rust** running globally at the edge on **Cloudflare Workers** (`workers-rs`).

It acts as a resilient proxy connecting any AI client (**Cursor, Cline, Roo Code, Continue.dev, Aider, Windsurf, OpenAI SDK, LangChain**) to **Google Gemini API** (`gemini-2.5-flash`, `gemini-2.5-pro`, `gemini-2.0-flash`, and `gemini-2.0-flash-lite` with an automatic Smart Router).

Designed to be hosted publicly on GitHub with zero risk of secret leakage, multi-account rotation supporting 100–1,000+ API keys in a single secret, quota-domain isolation, first-byte safe retries, and zero-buffering SSE streaming.

---

## Supported Tools & IDEs

| Tool / Client | Support Status | Protocol / Endpoint |
|---|:---:|---|
| **Cursor IDE** | ✅ Native | `https://<YOUR-WORKER>/v1` |
| **Cline / Roo Code** (VS Code) | ✅ Native | OpenAI-compatible provider |
| **Continue.dev** (VS Code / JetBrains) | ✅ Native | OpenAI provider |
| **Aider** (CLI Pair Programmer) | ✅ Native | `OPENAI_API_BASE` |
| **Windsurf / Codeium** | ✅ Native | OpenAI Custom Endpoint |
| **OpenAI Python & Node.js SDK** | ✅ Native | `base_url` override |
| **LangChain / LlamaIndex / LiteLLM** | ✅ Native | Standard OpenAI Base URL |
| **cURL / HTTP Clients** | ✅ Native | Standard SSE & JSON REST |

---

## System Architecture

```
Client (Cursor, Cline, Continue, Aider, OpenAI SDK)
      │
      │ HTTPS (POST /v1/chat/completions, POST /v1/responses, GET /v1/models)
      │ Header: Authorization: Bearer <PROXY_TOKEN>
      ▼
Cloudflare Worker (Edge / Rust `workers-rs`)
      │
      ├── 1. Security & Auth Guard
      │      - Validates Bearer token using constant-time comparison
      │
      ├── 2. Smart Router & Task Classifier (Active)
      │      - Automatically classifies user task using `gemini-3.5-flash-lite`
      │      - EASY ➔ `gemini-3.5-flash-lite` (lightning fast, quota efficient)
      │      - NORMAL ➔ `gemini-2.5-flash` (balanced coding & reasoning)
      │      - HARD ➔ `gemini-2.5-pro` (complex architecture & deep reasoning)
      │
      ├── 3. Account & Quota Scheduler (LRU + Quota Domain)
      │      - Rotates keys across accounts/projects
      │      - Isolates quota domains (if 1 key hits 429, cools down entire project)
      │      - Supports 100 - 1,000+ keys via `GEMINI_KEYS_POOL`
      │
      ├── 4. Upstream Gateway (Google Gemini OpenAI Endpoint)
      │      - URL locked to `https://generativelanguage.googleapis.com/v1beta/openai`
      │      - Resolves API key securely from Cloudflare Secrets
      │      - Strips sensitive headers
      │
      ├── 5. First-Byte Aware Retry Loop
      │      - Retries retryable errors (429, 5xx, network drop) before first byte
      │      - Strictly prevents replay if `first_byte_sent == true`
      │
      └── 6. End-to-End SSE Streaming
             - Direct streaming to client without buffering in memory
```

---

## Model & Smart Routing Policy

- **Primary Model**: `gemini-2.5-flash` (Google's flagship fast model, optimized for coding, debugging, reasoning).
- **Pro Model**: `gemini-2.5-pro` (State-of-the-art coding and complex architecture reasoning).
- **Lite Model**: `gemini-3.5-flash-lite` (Ultra-fast, cost & quota efficient).
- **Workhorse Model**: `gemini-2.0-flash` (Next-gen fast multimodal).
- **Supported Model Aliases** (automatically normalized):
  - `auto` ➔ `gemini-2.5-flash`
  - `gpt-4o`, `gpt-4.1`, `o3-mini`, `deepseek-chat` ➔ `gemini-2.5-flash`
  - `gpt-4o-mini` ➔ `gemini-3.5-flash-lite`
  - `o1`, `claude-3-5-sonnet`, `claude-3-7-sonnet`, `deepseek-reasoner` ➔ `gemini-2.5-pro`
  - Direct pass-through: `gemini-3.5-flash-lite`, `gemini-2.5-flash`, `gemini-2.5-pro`, `gemini-2.0-flash`, `gemini-1.5-flash`, `gemini-1.5-pro`

---

## Secret Separation (Public Repo Safe)

| Secret Tier | Location | Contents | Access / Scope |
|---|---|---|---|
| **Tier A: Public Repo** | GitHub Public Repo | Source code, `wrangler.toml`, `.env.example`, CI/CD workflows | Publicly visible |
| **Tier B: CI/CD Secrets** | GitHub Actions Secrets | `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` | Used solely for deploying worker |
| **Tier C: Runtime Secrets** | Cloudflare Worker Secrets | `PROXY_TOKEN`, `GEMINI_KEYS_POOL` / `GEMINI_KEY_01`..`20` | Only inside Worker runtime |

---

## Setup & Deployment Guide

### 1. Configure Cloudflare Secrets

In your Cloudflare Dashboard under **Workers & Pages > gemini-openai-gateway > Settings > Variables and Secrets**:

#### Option A: Bulk Keys Pool (Recommended for 10 - 1,000+ Keys in 1 Secret)
Add a secret named **`GEMINI_KEYS_POOL`** containing all your keys (newline-separated, comma-separated, or JSON):
```text
AIzaSyKey1...
AIzaSyKey2...
AIzaSyKey1000...
```
Also add **`PROXY_TOKEN`** (your secret password for client tools, e.g. `sk-my-gateway-token`).

#### Option B: Individual Keys (Up to 20 keys with auto-discovery)
- `PROXY_TOKEN`: Your secret client password.
- `GEMINI_KEY_01` .. `GEMINI_KEY_20`: Automatically detected by the worker.

### 2. Configure GitHub Secrets for CI/CD Deployment

In GitHub repository: **Settings > Secrets and variables > Actions**:
1. `CLOUDFLARE_API_TOKEN`: Cloudflare API Token (with *Edit Cloudflare Workers* template).
2. `CLOUDFLARE_ACCOUNT_ID`: Your Cloudflare Account ID.

### 3. Deploy via GitHub Actions

Pushing to `main` automatically formats, tests, compiles Wasm, and deploys:
```bash
git push origin main
```

---

## Client Setup Guides

### 1. Cursor IDE
1. Open **Cursor Settings > Models**.
2. Under **OpenAI API**:
   - **OpenAI API Key**: `<YOUR_PROXY_TOKEN>`
   - **Override OpenAI Base URL**: `https://gemini-openai-gateway.<subdomain>.workers.dev/v1`
3. Add model: `gemini-2.5-flash` (or use `gpt-4o`).

### 2. Cline / Roo Code (VS Code Extension)
1. Open Cline / Roo Code Settings.
2. Select API Provider: **OpenAI Compatible**.
3. **Base URL**: `https://gemini-openai-gateway.<subdomain>.workers.dev/v1`
4. **API Key**: `<YOUR_PROXY_TOKEN>`
5. **Model ID**: `gemini-2.5-flash` (or `auto`).

### 3. Continue.dev (`config.json`)
```json
{
  "models": [
    {
      "title": "Gemini 3.8 Flash (via Gateway)",
      "provider": "openai",
      "model": "gemini-2.5-flash",
      "apiBase": "https://gemini-openai-gateway.<subdomain>.workers.dev/v1",
      "apiKey": "YOUR_PROXY_TOKEN"
    }
  ]
}
```

### 4. Aider (Terminal Pair Programmer)
```bash
export OPENAI_API_BASE=https://gemini-openai-gateway.<subdomain>.workers.dev/v1
export OPENAI_API_KEY=YOUR_PROXY_TOKEN
aider --model gemini-2.5-flash
```

### 5. OpenAI Python SDK
```python
from openai import OpenAI

client = OpenAI(
    api_key="YOUR_PROXY_TOKEN",
    base_url="https://gemini-openai-gateway.<subdomain>.workers.dev/v1",
)

response = client.chat.completions.create(
    model="gemini-2.5-flash",
    messages=[{"role": "user", "content": "Write a high-performance Rust actor"}],
    stream=True,
)

for chunk in response:
    content = chunk.choices[0].delta.content
    if content:
        print(content, end="", flush=True)
```

---

## Verification via cURL

```bash
# Test Models endpoint:
curl https://gemini-openai-gateway.<subdomain>.workers.dev/v1/models \
  -H "Authorization: Bearer YOUR_PROXY_TOKEN"

# Test Streaming Chat Completions:
curl -N https://gemini-openai-gateway.<subdomain>.workers.dev/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_PROXY_TOKEN" \
  -d '{"model":"gemini-2.5-flash","messages":[{"role":"user","content":"Hi!"}],"stream":true}'
```

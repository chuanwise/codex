# Codex with unlimited HTTP status retries

Maintained branch: `codex-unbounded-retry`.

Shared API requests retry every non-2xx HTTP error indefinitely, including 401,
403, 404, 429, and 5xx. This includes unary and streaming HTTP API requests that
use `codex-client::run_with_retry`. The policy is always enabled in this fork;
`request_max_retries` and `stream_max_retries` cannot exhaust this HTTP loop.
HTTP status errors that reach the Responses stream retry layer, including
WebSocket handshake status errors, also retry indefinitely. HTTP clients unrelated
to model API requests retain upstream behavior.

The local backoff starts at 5 seconds and doubles to a 60-second maximum.
`Retry-After` can extend a wait, but each wait remains capped at 60 seconds.
Requests remain cancellable. Permanent status errors keep retrying until cancelled.

Pushes to the maintained branch, and manual runs of **Fork release**, test the
retry implementation, build five platform packages, smoke-test them, and publish
all packages to a new GitHub Release. The workflow requires no personal tokens,
upstream signing credentials, paid runners, or npm publishing access. GitHub's
normal Actions availability and usage limits still apply.

Download a release archive for your OS and architecture, extract the entire
archive, and run `bin/codex` (`bin/codex.exe` on Windows). Preserve the adjacent
`codex-resources`, package manifest, and helper binaries. Linux builds use glibc
and target Ubuntu 24.04 or compatible newer distributions. macOS builds are unsigned.
Each archive has a SHA-256 file and a source commit file.

Run retry regression tests with:

```bash
just test -p codex-api --test unbounded_http_retry --locked
```

Upstream base for the first fork change:
`b1e72963c3b71a9265a551e54beff078384efed9`.

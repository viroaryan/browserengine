# 🛡️ Security Policy — Nirvana Browser Engine

## Memory-Safety Architecture

Nirvana Browser Engine is designed from the ground up in **100% Pure Rust** to eliminate entire classes of C/C++ browser vulnerabilities (Use-After-Free, buffer overflows, uninitialized reads):

- **`#![forbid(unsafe_code)]`:** Enforced across all 12 core subsystems (`url`, `dom`, `html`, `css`, `style`, `layout`, `font`, `gfx`, `js`, `net`, `adblock`, `core`).
- **JNI Panic & Pointer Boundary (`nirvana_android`):** Null pointer validation + `std::panic::catch_unwind` prevents native Rust panics from terminating the Android JVM process.
- **Resource & DoS Caps:** Guarded HTML recursion depth (`512`), viewport clamp (`1..16384`), image dimension clamp (`<= 8192px`), network body bounds, and sandboxed JS loop/recursion budgets (`boa_engine`).
- **Network Privacy & Isolation:** Strict Same-Origin Policy (`SOP`), Content Security Policy (`CSP`), Mixed-Content blocking, `HttpOnly` RFC 6265 cookie isolation, and native socket-level tracking blocker (`nirvana_adblock`).

---

## 🚨 Reporting a Vulnerability

> [!IMPORTANT]
> **Please DO NOT open public GitHub issues for exploitable security vulnerabilities** (memory corruption across JNI, sandbox escapes in `nirvana_js`, SOP/CORS bypasses, or XSS/cookie leaks).

Instead, contact the maintainer **@viroaryan** privately or enable GitHub Private Security Advisories on `viroaryan/browserengine` with:
1. Affected crate(s) or Android component (`crates/nirvana_*` or `android/`)
2. Minimal reproducible HTML / CSS / JS / URL test case
3. Observed stack trace, behavior, and potential security impact

*Evaluation & auditing of this codebase for security research is strictly governed by the Proprietary Viewing Terms in [LICENSE](LICENSE).*

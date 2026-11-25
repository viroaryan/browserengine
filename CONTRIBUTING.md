# Contributing to Nirvana Browser Engine

> [!CAUTION]
> **Proprietary Licensing & IP Assignment Notice**
> Nirvana Browser Engine is currently **Proprietary Source-Available (`ALL RIGHTS RESERVED`)** under [LICENSE](LICENSE). Before contributing code, opening a Pull Request, or submitting patches, please read the terms below carefully.

---

## 📜 Mandatory Legal & Ownership Terms

1. **No Open-Source License Grant:** Viewing this repository does **not** grant an MIT, Apache-2.0, GPL, or permissive reuse license. Do not fork and redistribute derivatives without written permission from **@viroaryan**.
2. **Contributor IP Assignment:** By opening a Pull Request or submitting code/documentation/assets to this repository, you confirm that your work is wholly original and you irrevocably assign all copyright and intellectual property rights in your submission to **Aryan (@viroaryan) & Nirvana Browser Project**.

---

## ⚙️ Sovereign Engineering Standards

All accepted code changes must pass every quality and safety bar:

| Standard | Requirement |
| :--- | :--- |
| **Zero `unsafe` in Core Crates** | All 12 core crates strictly enforce `#![forbid(unsafe_code)]`. Only `crates/nirvana_android` allows JNI FFI wrapped in `catch_unwind`. |
| **Zero C/C++ Engine Dependencies** | Never introduce Chromium, WebKit, Gecko, Skia C++, or C/C++ build tool dependencies into the engine crates. |
| **Panic Immunity** | Real-world malformed HTML, broken CSS (`min > max`), invalid UTF-8/images, or adversarial JS must return safe defaults or errors—never panic. |
| **Full Verification Required** | Run `cargo test --workspace --no-fail-fast` and Android `gradle testDebugUnitTest assembleDebug` cleanly before opening a PR. |

---

## 🔄 Pull Request Workflow

1. Discuss non-trivial changes first via an [Architecture / Feature Request](.github/ISSUE_TEMPLATE/feature_request.yml) or [Site Compatibility Issue](.github/ISSUE_TEMPLATE/site_compat.yml).
2. Keep each change clean, single-responsibility, and documented with concise docstrings on public APIs.
3. Verify formatting (`cargo fmt --check`) and tests (`cargo test --workspace`) cleanly before submitting via our [Pull Request Template](.github/PULL_REQUEST_TEMPLATE.md).

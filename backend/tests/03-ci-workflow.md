# 03 — CI workflow

> **B4 doc set, part 3.** The backend GitHub Actions workflow: exact file
> content, turn-on order, the frontend job to add later, and branch-protection
> recommendations. Owner: B4 — you create the actual file at
> `.github/workflows/backend-ci.yml` at the repo root (the YAML below is the
> full content; nothing else in this doc needs to exist as a file).

---

## 1. The workflow file — `.github/workflows/backend-ci.yml`

Create the directory `.github/workflows/` at the **repo root**
(`/home/z/my-project/Rechess/.github/workflows/`), not under `backend/`.
Exact content:

```yaml
# Backend quality gates — owner: B4 (backend/tests/03-ci-workflow.md).
# Mirrors the local merge-gate checklist in backend/tests/01-merge-gates.md:
# fmt -> clippy -> test. Kept in one job so failures report in gate order.
name: backend-ci

on:
  push:
    branches: [main]
    paths: ['backend/**']
  pull_request:
    branches: [main]
    paths: ['backend/**']

defaults:
  run:
    working-directory: backend

jobs:
  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # "stable" matches the pinned channel in backend/rust-toolchain.toml
      # (1.100.0). If the pin ever moves off stable, change this to
      # dtolnay/rust-toolchain@1.100.0 so CI never drifts from the pin.
      # The toolchain profile is "minimal", so the components are added here.
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend

      # Gate 1 — formatting. RED on the current tree until the one-time
      # mechanical `cargo fmt --all` PR lands (01-merge-gates.md §4): land
      # that PR FIRST, in the same PR-set as this workflow, or every backend
      # PR fails on arrival.
      - name: cargo fmt --check
        run: cargo fmt --all -- --check

      # Gate 2 — static analysis, warnings denied. Passes today (exit 0).
      # The 4 "unused dependency" warnings (rand, shakmaty, tower-http, uuid)
      # are expected: deliberate Day-1 pins for B1/B2/B3, cargo *manifest*
      # warnings that -D warnings does not promote (01-merge-gates.md §5).
      # They disappear as the modules land. If a future toolchain starts
      # failing on them, add the documented allow to backend/Cargo.toml:
      #   [lints.cargo]
      #   unused_dependencies = "allow"
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings

      # Gate 3 — unit + integration tests (15 tests / 3 suites today; grows
      # with game:: / rooms:: / network:: and the integration suites).
      - name: cargo test
        run: cargo test

      # Gate 4 — the release artifact builds (the Dockerfile's builder step
      # depends on this exact command succeeding).
      - name: cargo build --release
        run: cargo build --release
```

Design notes (why it looks like this):

- **`paths: ['backend/**']`** keeps backend gates off pure-frontend PRs, so
  F1–F4 are not blocked by a rustc problem they cannot fix — and vice versa
  for the frontend job (§3).
- **Toolchain**: `dtolnay/rust-toolchain@stable` installs the current stable
  (1.100.0 is pinned in `backend/rust-toolchain.toml`); any `rustup`-mediated
  command inside `backend/` then resolves via the pin. The alternative is no
  action at all: plain `rustup` on ubuntu-latest reads
  `rust-toolchain.toml` and installs the pin automatically — but the
  dtolnay action gives the explicit `components:` input the minimal profile
  needs. If the team ever pins a non-stable channel, switch to
  `dtolnay/rust-toolchain@1.100.0` with the same components line.
- **`Swatinem/rust-cache@v2`** with `workspaces: backend` keys the cache on
  `backend/Cargo.lock` (which is committed — that is what makes CI builds
  reproducible, per the master-plan version policy). Do not add
  `cache: all`-style shortcuts that mix debug and release artifacts.
- **Steps run in gate order** (fmt → clippy → test → release build) so the
  check name tells you which gate failed; `--all-targets` on clippy covers
  `tests/` integration binaries too, which is what the local checklist runs.
- **No Rust unit tests are ever skipped in CI**: `cargo test` runs all
  suites. If a test must be temporarily disabled, that requires team approval
  and a tracked issue (your brief §6) — CI has no escape hatch for it.

## 2. Turn-on order (important)

1. Land the one-time mechanical formatting PR (`cargo fmt --all`, see
   [01-merge-gates.md](01-merge-gates.md) §4). Until then the fmt step fails
   on every PR.
2. Create `.github/workflows/backend-ci.yml` with the content above.
3. Watch the first run on a PR: all four steps green (they are green locally
   today: test 15/15, clippy exit 0, release build ok — fmt needs step 1).
4. Then protect `main` (§4).

## 3. Frontend job (add later, same repo, separate workflow)

The frontend gate set is four commands (`frontend/RULES.md` §5 — UI is
additionally verified by manual checklists; the wire contract is proven by
`cargo test protocol_contract.rs` on the backend, by `npm run typecheck`
compiling `protocol.fixtures.ts`, and by vitest in `protocol.test.ts`). The
job runs all four frontend gates on node 22. Create it as
`.github/workflows/frontend-ci.yml` when F3/F4 start merging:

```yaml
name: frontend-ci

on:
  push:
    branches: [main]
    paths: ['frontend/**']
  pull_request:
    branches: [main]
    paths: ['frontend/**']

defaults:
  run:
    working-directory: frontend

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: frontend/package-lock.json
      - run: npm ci
      - name: typecheck        # tsc --noEmit — also compiles protocol fixtures
        run: npm run typecheck
      - name: lint
        run: npm run lint
      - name: test             # vitest — wire-contract and store unit tests
        run: npm run test
      - name: build            # production build (what B4 ships)
        run: npm run build
```

Coordinate with F3/F4 so a missing script name in `frontend/package.json`
fails loudly (setup-node runs `npm ci` against the committed lockfile). The
goal from your brief: frontend typecheck/lint/test/build failures are
**visible in the shared repository**, even as a separate job — never let
frontend breakage hide behind a green backend check.

## 4. Branch protection recommendations (repo settings)

Configure once the workflow has one green run on `main`:

- **Require pull request before merging** to `main`, with at least 1 approval
  — this is the technical backing for the merge-authority workflow
  ([README.md](README.md) §2): module owners (B1/B2/B3, F1–F4) do not push to
  `main` directly; PRs only.
- **Required status checks** (search and select):
  `backend-ci / backend` and later `frontend-ci / frontend`.
  Require branches to be up to date before merging.
- **Restrict who can push** to `main` to B4 (merge authority) — with required
  PRs on, admins should not bypass either; keep "do not allow bypassing"
  enabled.
- **Do not** enable "auto-dismiss stale approvals" lightly: a re-review after
  every fmt fixup is noise in an 8-person team; instead require re-review
  only when the PR changes `backend/src/protocol/`, `docs/protocol-v1.md` or
  `frontend/src/types/protocol.ts` (the same-PR rule, enforced socially by
  you as reviewer — [01-merge-gates.md](01-merge-gates.md) §8).

## 5. Operating rules for CI (binding)

- CI and the local checklist in [01-merge-gates.md](01-merge-gates.md) must
  always run the **same commands**. If you change a gate, you change both,
  in one PR.
- Never make main green by disabling a failing test without team approval and
  a tracked issue (your brief §6). A temporarily-skipped test shows as
  `ignored` in the step log — you review those weekly.
- CI failures are the owning module owner's to fix; you attribute them via
  the per-module filters ([01-merge-gates.md](01-merge-gates.md) §3) and
  route the issue, rather than patching another owner's module yourself.

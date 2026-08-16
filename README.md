# Implore

A small Catholic app for the people you quietly carry in prayer.

Add prayer intentions with optional details, tags, color, schedule, and an optional saint association. Shared logic lives in Rust ([Crux](https://redbadger.github.io/crux/)); the iOS shell is SwiftUI.

## Status

iOS app with local persistence and optional private account sync:

- **Today** — intentions due today (daily / weekly / monthly / novena cadence), mark as prayed, temporal-cycle liturgical day heading (US calendar, computed in Rust), and named observances from the remote catalog
- **Calendar** — browse ±1 year from today; liturgical day, observances, and intentions due on the selected day (mark prayed only for today)
- **Intentions** — add / edit / remove / archive; optional saint companion from the same catalog (`companion: true`)
- **Settings** — daily reminder digest, appearance, language (en / fr / es), optional email/password account + sync

Auth and sync state machines live in the Crux core (`crux_http` + `crux_kv`). A small Axum API in `server/` stores private sync documents (SQLite).

## Layout

```
shared/                 # Rust Crux core (events, model, view model, liturgical, account/sync) + BoltFFI
server/                 # Local Axum sync API (auth + GET/PUT /sync)
apple/                  # SwiftUI iOS app (XcodeGen + generated bindings)
content/observances/    # Unified calendar + companions (en, fr, es)
```

## Prerequisites

- Rust (see `rust-version` in root `Cargo.toml`)
- [Xcode](https://developer.apple.com/xcode/)
- [XcodeGen](https://github.com/yonaskolb/XcodeGen)
- [BoltFFI](https://github.com/redbadger/boltffi) CLI (`boltffi`)
- [just](https://github.com/casey/just) (optional, for the Apple recipes)

## Shared core

```bash
cargo test -p shared
```

Regenerate Swift types after changing `Event` / `ViewModel` / `Prayer` (or from `apple/`, use `just typegen`):

```bash
cargo run -p shared --features codegen,facet_typegen --bin codegen -- \
  --language swift --output-dir apple/generated
```

## Sync API (local)

```bash
cargo run -p server
```

Listens on `http://0.0.0.0:3000` by default (`IMPLORE_BIND`, `IMPLORE_DB` env vars optional). The iOS Simulator reaches it at `http://127.0.0.1:3000` (the core’s `API_BASE_URL`).

```bash
cargo test -p server
```

## iOS app

From `apple/`:

```bash
just generate   # typegen + boltffi pack + xcodegen
open ImploreApp.xcodeproj
```

Or build from the command line:

```bash
just build
```

Use the **ImploreApp-iOS** scheme. Generated packages under `apple/generated/` are gitignored; run `just generate` after a fresh clone.

Start the sync server before using Account in Settings.

## Content catalog

Locale JSON in `content/observances/` (`en.json`, `fr.json`, `es.json`). One list drives Today, Calendar, and the saint picker:

| Field | Role |
|-------|------|
| `id`, `name`, `date` (`MM-DD`), `rank`, `summary` | Today / Calendar named feast / memorial |
| `companion: true` | Intention picker (+ patronage) |
| `companion: false` | Calendar-only (e.g. Transfiguration, Assumption) |

Movable temporal-cycle days (Easter, Pentecost, Ordinary Time weeks, …) come from Rust (`shared/src/liturgical.rs`), not this JSON.

At runtime the iOS app downloads `observances/{locale}.json` from S3 and caches it under Application Support. On first launch or when offline, the app uses the cached copy; if a locale is missing, it falls back to English.

### Publishing

From `apple/`:

```bash
just publish-observances
```

Requires the [AWS CLI](https://aws.amazon.com/cli/) with write access to the bucket. Base URL:

`https://atgeo-intercede-app-090552655796-us-east-2-an.s3.us-east-2.amazonaws.com/`

## License

Copyright © 2026 Georges Kmeid. All rights reserved.

This repository is public for viewing only. No license is granted to use, copy, modify, or distribute the source code or content without explicit permission.

# Implore

A small Catholic app for the people you quietly carry in prayer.

Add prayer intentions with optional details, tags, and an optional saint association. Shared logic lives in Rust ([Crux](https://redbadger.github.io/crux/)); the iOS shell is SwiftUI.

## Status

Early scaffold: add/remove/archive intentions on iOS with local persistence and Active/Archived/All filters; optional saint picker backed by a remote catalog. iCloud sync and daily rotation are not built yet.

## Layout

```
shared/          # Rust Crux core (events, model, view model) + BoltFFI
apple/           # SwiftUI iOS app (XcodeGen + generated bindings)
content/saints/  # Saints catalog source JSON (en, fr, es)
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

Regenerate Swift types after changing `Event` / `ViewModel` / `Prayer`:

```bash
cargo run -p shared --features codegen --bin codegen -- \
  --language swift --output-dir apple/generated
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

## Saints catalog

The saints list is maintained as locale JSON files in `content/saints/` (for example `en.json`, `fr.json`, `es.json`). Each file contains a versioned catalog with saint id, name, feast day, patronage, and summary.

At runtime the iOS app downloads the catalog from a public S3 bucket (`saints/{locale}.json`) and caches it locally under Application Support. On first launch or when offline, the app uses the cached copy; if a locale is missing, it falls back to English.

To publish catalog changes, edit the JSON in `content/saints/` and upload to S3 from `apple/`:

```bash
just publish-saints
```

This requires the [AWS CLI](https://aws.amazon.com/cli/) configured with credentials that can write to the bucket. The app reads from:

`https://atgeo-intercede-app-090552655796-us-east-2-an.s3.us-east-2.amazonaws.com/saints/{locale}.json`

## License

Apache-2.0

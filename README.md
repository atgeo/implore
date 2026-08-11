# Implore

A small Catholic app for the people you quietly carry in prayer.

Add prayer intentions with optional details and tags. Shared logic lives in Rust ([Crux](https://redbadger.github.io/crux/)); the iOS shell is SwiftUI.

## Status

Early scaffold: add/remove/archive intentions on iOS with local persistence and Active/Archived/All filters. iCloud sync, associations, and daily rotation are not built yet.

## Layout

```
shared/          # Rust Crux core (events, model, view model) + BoltFFI
apple/           # SwiftUI iOS app (XcodeGen + generated bindings)
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

## License

Apache-2.0

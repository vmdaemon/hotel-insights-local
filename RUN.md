# How to Run Hotel Insights (Local)
## Prerequisites
- Node.js (v18+ recommended)
- npm
- Rust toolchain (rustup/cargo)
- Tauri system dependencies
macOS setup:
```bash
xcode-select --install
```

## Run desktop app (GUI)
```bash
cd /Users/vamadha/hotel-insights-local
npm install
```

If macOS libc++ headers error appears:
```bash
SDKROOT="$(xcrun --show-sdk-path)"
export CXXFLAGS="-isystem $SDKROOT/usr/include/c++/v1 -isysroot $SDKROOT"
export CPPFLAGS="$CXXFLAGS"
```

Start app:
```bash
npm run tauri dev
```

## First-time app flow
1. Create admin user
2. Login
3. Pick bookings CSV
4. Preview
5. Import
6. Explore dashboard

## Build production app
```bash
cd /Users/vamadha/hotel-insights-local
npm run build
npm run tauri build
```

## CLI import (optional)
```bash
cd /Users/vamadha/hotel-insights-local/src-tauri
cargo run --bin hotel-insights-cli -- import "/absolute/path/to/hotel_bookings.csv"
```

With custom DB file:
```bash
cargo run --bin hotel-insights-cli -- import "/absolute/path/to/hotel_bookings.csv" --db-file "/absolute/path/to/analytics.duckdb"
```

# Hotel Insights (Local) — Dev Notes

## Run the desktop app

```bash
cd hotel-insights-local
npm install

SDKROOT="$(xcrun --show-sdk-path)"
export CXXFLAGS="-isystem $SDKROOT/usr/include/c++/v1 -isysroot $SDKROOT"
export CPPFLAGS="$CXXFLAGS"

npm run tauri dev
```

The `CXXFLAGS/CPPFLAGS` are required on some macOS setups so the bundled DuckDB build can find libc++ headers.

## Run the importer end-to-end (no GUI)

```bash
cd hotel-insights-local/src-tauri

SDKROOT="$(xcrun --show-sdk-path)"
export CXXFLAGS="-isystem $SDKROOT/usr/include/c++/v1 -isysroot $SDKROOT"
export CPPFLAGS="$CXXFLAGS"

cargo run --bin hotel-insights-cli -- import "/absolute/path/to/hotel_bookings.csv"
```

This will:
- Create/open the per-app DuckDB database
- Auto-create properties from the CSV `hotel` column
- Append observations
- Block exact duplicate file re-imports per property (by SHA-256 file hash)

The default DB file location for the CLI is:

```
~/Library/Application Support/com.madhav.hotelinsights/hotelinsights/analytics.duckdb
```

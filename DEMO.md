# Demo Guide — Hotel Insights (Local)

This guide walks through a complete demo of **Hotel Insights (Local)** and includes **example outputs** you can use while presenting.

## What you’re demoing

**Hotel Insights (Local)** is a local-first desktop analytics app for hotel booking data.

In this demo you will:

1. Run the Tauri desktop app
2. Bootstrap an admin user (first run)
3. Import a bookings CSV (with preview)
4. Explore KPIs and breakdowns
5. (Optional) run the importer + debug outputs via CLI

---

## Prerequisites

- Node.js + npm
- Rust toolchain (cargo)
- Tauri prerequisites for your OS
- A bookings CSV file with required headers

### CSV requirements

The importer expects these required columns:

- `hotel`
- `is_canceled`
- `lead_time`
- `arrival_date_year`
- `arrival_date_month`
- `arrival_date_day_of_month`
- `stays_in_weekend_nights`
- `stays_in_week_nights`
- `adults`
- `children`
- `babies`

It will also use additional optional columns when present (examples: `market_segment`, `distribution_channel`, `country`, `adr`, etc.).

---

## Demo flow (GUI)

### 1) Start the desktop app

From project root:

```bash
cd /Users/vamadha/hotel-insights-local
npm install
npm run tauri dev
```

#### macOS note (libc++ headers / DuckDB builds)

On some macOS setups, you may need these exports:

```bash
SDKROOT="$(xcrun --show-sdk-path)"
export CXXFLAGS="-isystem $SDKROOT/usr/include/c++/v1 -isysroot $SDKROOT"
export CPPFLAGS="$CXXFLAGS"

npm run tauri dev
```

**Expected behavior**

- The app opens with the title **Hotel Insights**.
- On first launch (no users in DB): you see **Create Admin**.
- On subsequent launches: you see **Login**.

---

### 2) Bootstrap admin (first run only)

In the app:

1. Enter a username and password
2. Confirm password
3. Click **Create**

**Expected result**

- Admin user is created
- You are logged in immediately
- App transitions to the Home/Dashboard screen

---

### 3) Import bookings CSV

In the app:

1. Click **Pick bookings CSV**
2. Select a `.csv` file
3. Confirm preview loads
4. Click **Import**

**Expected UI output**

- A preview summary (rows/columns) and a header list
- Import results including a computed file hash
- Per-property results, because the importer supports **multi-property** files by grouping on the CSV `hotel` column

#### Example output (what you should see conceptually)

The app shows something like:

```
Imported file hash: 7b5c...f2a1

Results:
  Resort Hotel: OK — imported 312 / rejected 5
  City Hotel:   OK — imported 420 / rejected 3
```

#### Duplicate import protection

If you import the **exact same file again**, the importer blocks duplicates per property (by SHA-256 hash).

**Expected output**

```
Resort Hotel: BLOCKED — imported 0 / rejected 1
This exact file was already imported for this property at 2026-03-10 11:21:45
```

---

### 4) Explore the Overview dashboard

The dashboard supports:

- selecting properties (checkboxes)
- setting a date range (Start/End)
- refreshing metrics

**Expected KPIs (examples)**

- Arrivals (non-canceled)
- Cancellation rate
- Room-nights
- ADR avg
- Estimated revenue
- Lead time avg/median
- Room type change rate
- Repeat guest rate
- Special requests avg

**Expected breakdown tables (examples)**

- Arrivals by month (non-canceled)
- Cancellations by month
- Market segment mix (top)
- Distribution channel mix (top)
- Country mix (top)

> Note: values depend on your dataset.

---

## Demo flow (CLI) — optional but great for “proof” output

The repo includes a CLI binary: `hotel-insights-cli`.

### 1) Import via CLI

```bash
cd /Users/vamadha/hotel-insights-local/src-tauri

cargo run --bin hotel-insights-cli -- import "/absolute/path/to/hotel_bookings.csv"
```

**Example JSON output (shape)**

```json
{
  "file_hash": "7b5c...f2a1",
  "properties": [
    {
      "property_name": "Resort Hotel",
      "property_id": 1,
      "import_id": 12,
      "blocked_duplicate": false,
      "blocked_message": null,
      "rows_imported": 312,
      "rows_rejected": 5
    }
  ]
}
```

### 2) Print debug breakdown outputs

```bash
cargo run --bin hotel-insights-cli -- debug-breakdowns
```

**Example JSON output (shape)**

```json
{
  "properties": [{ "id": 1, "name": "Resort Hotel" }],
  "overview": {
    "bookings_total": 12345,
    "bookings_canceled": 4512,
    "cancellation_rate": 0.3654,
    "room_nights": 28400,
    "avg_los": 3.12,
    "adr_avg": 104.87,
    "est_revenue": 2978200.45,
    "min_arrival_date": "2015-07-01",
    "max_arrival_date": "2017-08-31"
  },
  "cancellations_by_month": [{ "month": "2016-01", "bookings_total": 380, "bookings_canceled": 120, "cancellation_rate": 0.3158 }],
  "market_segment_mix": [{ "key": "Online TA", "count": 4200, "share": 0.39 }]
}
```

---

## Suggested presenter script (2–3 minutes)

1. “This is a local-first hotel analytics desktop app built with Tauri + Preact + DuckDB.”
2. “On first launch, we bootstrap an admin account. After that, users log in.”
3. “Now I import a bookings CSV — the app previews the file before import.”
4. “Import stores data locally, groups by property, and blocks duplicate re-imports.”
5. “Dashboard shows KPIs and mix breakdowns across properties and time windows.”
6. “Everything runs locally — no external DB required.”

---

## Troubleshooting (demo day)

- **Login fails**: ensure the admin user exists and credentials match.
- **Import fails**: verify the CSV has the required headers (exact names).
- **Duplicate blocked**: use a different export file (or a different dataset) to show a fresh import.
- **macOS build errors**: re-run with `SDKROOT/CXXFLAGS/CPPFLAGS` exports.

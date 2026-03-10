# Hotel Insights (Local)
A local-first desktop analytics app for hotel booking data.
`hotel-insights-local` is built with **Tauri + Preact + TypeScript + DuckDB**.  
It lets you import booking CSV files, store data locally, and explore operational KPIs such as arrivals, cancellations, ADR, estimated revenue, and mix breakdowns.
## Key features
- Local desktop app (Tauri)
- CSV preview + import
- Auto property creation from CSV `hotel`
- Duplicate-file protection (SHA-256 hash per property)
- First-run admin bootstrap + login
- KPI dashboard and categorical breakdowns
## Tech stack
- Frontend: Preact + TypeScript + Vite
- Desktop: Tauri v2
- Backend: Rust
- Local DB: DuckDB
## CSV required columns
- hotel
- is_canceled
- lead_time
- arrival_date_year
- arrival_date_month
- arrival_date_day_of_month
- stays_in_weekend_nights
- stays_in_week_nights
- adults
- children
- babies
## Default DB location (macOS)
`~/Library/Application Support/com.madhav.hotelinsights/hotelinsights/analytics.duckdb`
## Run instructions
See `RUN.md`.

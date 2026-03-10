use std::path::{Path, PathBuf};

fn default_db_file() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("com.madhav.hotelinsights")
        .join("hotelinsights")
        .join("analytics.duckdb"))
}

fn usage() -> &'static str {
    "Usage:\n  hotel-insights-cli import <csv_path> [--db-file <db_file>]\n  hotel-insights-cli debug-breakdowns [--db-file <db_file>]\n"
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Err(usage().to_string());
    };

    match cmd.as_str() {
        "import" => {
            let Some(csv_path) = args.next() else {
                return Err(usage().to_string());
            };

            let mut db_file: Option<String> = None;
            while let Some(arg) = args.next() {
                if arg == "--db-file" {
                    db_file = args.next();
                } else {
                    return Err(format!("unknown arg: {arg}\n\n{}", usage()));
                }
            }

            let db_file = db_file
                .map(PathBuf::from)
                .unwrap_or(default_db_file()?);

            let result = hotel_insights_local_lib::import_bookings::import_bookings_csv(
                &db_file,
                &PathBuf::from(csv_path),
            )?;

            // Print machine-readable output.
            println!(
                "{}",
                serde_json::to_string_pretty(&result)
                    .map_err(|e| format!("failed to serialize result: {e}"))?
            );
            Ok(())
        }
        "debug-breakdowns" => {
            let mut db_file: Option<String> = None;
            while let Some(arg) = args.next() {
                if arg == "--db-file" {
                    db_file = args.next();
                } else {
                    return Err(format!("unknown arg: {arg}\n\n{}", usage()));
                }
            }

            let db_file = db_file
                .map(PathBuf::from)
                .unwrap_or(default_db_file()?);

            let props = hotel_insights_local_lib::queries::list_properties(&db_file)?;
            let ids: Vec<i64> = props.iter().map(|p| p.id).collect();

            let overview = hotel_insights_local_lib::queries::overview_metrics(&db_file, &ids, None, None)?;
            let canc = hotel_insights_local_lib::queries::cancellations_by_month(
                &db_file,
                &ids,
                overview.min_arrival_date.as_deref(),
                overview.max_arrival_date.as_deref(),
            )?;
            let seg = hotel_insights_local_lib::queries::market_segment_mix(
                &db_file,
                &ids,
                overview.min_arrival_date.as_deref(),
                overview.max_arrival_date.as_deref(),
                8,
            )?;

            let out = serde_json::json!({
              "properties": props,
              "overview": overview,
              "cancellations_by_month": canc,
              "market_segment_mix": seg
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out)
                    .map_err(|e| format!("failed to serialize result: {e}"))?
            );
            Ok(())
        }
        _ => Err(usage().to_string()),
    }
}

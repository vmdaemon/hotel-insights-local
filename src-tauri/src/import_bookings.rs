use std::{collections::HashMap, path::Path};

use chrono::{Duration, NaiveDate};
use sha2::{Digest, Sha256};

use crate::db;

#[derive(serde::Serialize)]
pub struct ImportPropertyResult {
    pub property_name: String,
    pub property_id: i64,
    pub import_id: Option<i64>,
    pub blocked_duplicate: bool,
    pub blocked_message: Option<String>,
    pub rows_imported: u64,
    pub rows_rejected: u64,
}

#[derive(serde::Serialize)]
pub struct ImportBookingsResult {
    pub file_hash: String,
    pub properties: Vec<ImportPropertyResult>,
}

fn month_to_number(month: &str) -> Option<u32> {
    match month.trim().to_lowercase().as_str() {
        "january" => Some(1),
        "february" => Some(2),
        "march" => Some(3),
        "april" => Some(4),
        "may" => Some(5),
        "june" => Some(6),
        "july" => Some(7),
        "august" => Some(8),
        "september" => Some(9),
        "october" => Some(10),
        "november" => Some(11),
        "december" => Some(12),
        _ => None,
    }
}

fn compute_file_hash(file_path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(file_path).map_err(|e| format!("failed to read file: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

fn ensure_property_id(conn: &duckdb::Connection, name: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO properties (name) VALUES (?) ON CONFLICT(name) DO NOTHING",
        [name],
    )
    .map_err(|e| format!("failed to create property: {e}"))?;

    let id: i64 = conn
        .query_row(
            "SELECT id FROM properties WHERE name = ? LIMIT 1",
            [name],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to read property id: {e}"))?;
    Ok(id)
}

fn existing_import_timestamp(conn: &duckdb::Connection, property_id: i64, file_hash: &str) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT CAST(imported_at AS VARCHAR) FROM imports WHERE property_id = ? AND file_hash = ? LIMIT 1",
        duckdb::params![property_id, file_hash],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(ts) => Ok(Some(ts)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("failed to query existing import: {e}")),
    }
}

fn create_import_row(conn: &duckdb::Connection, property_id: i64, file_name: &str, file_hash: &str) -> Result<i64, String> {
    let id: i64 = conn
        .query_row(
            "INSERT INTO imports (property_id, file_name, file_hash) VALUES (?, ?, ?) RETURNING id",
            duckdb::params![property_id, file_name, file_hash],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to create import row: {e}"))?;

    Ok(id)
}

fn compute_booking_key(fields: &[(&str, String)]) -> String {
    let mut hasher = Sha256::new();
    for (k, v) in fields {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

pub fn import_bookings_csv(db_file: &Path, file_path: &Path) -> Result<ImportBookingsResult, String> {
    db::init_db(db_file)?;
    let mut conn = db::open_db(db_file)?;

    let file_hash = compute_file_hash(file_path)?;
    let file_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("bookings.csv")
        .to_string();

    let file = std::fs::File::open(file_path).map_err(|e| format!("failed to open csv: {e}"))?;
    let mut reader = csv::ReaderBuilder::new().has_headers(true).from_reader(file);
    let headers = reader
        .headers()
        .map_err(|e| format!("failed to read csv headers: {e}"))?
        .clone();

    let idx = |name: &str| -> Result<usize, String> {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| format!("missing required column: {name}"))
    };

    let hotel_i = idx("hotel")?;
    let is_canceled_i = idx("is_canceled")?;
    let lead_time_i = idx("lead_time")?;
    let year_i = idx("arrival_date_year")?;
    let month_i = idx("arrival_date_month")?;
    let day_i = idx("arrival_date_day_of_month")?;
    let wknd_i = idx("stays_in_weekend_nights")?;
    let wk_i = idx("stays_in_week_nights")?;
    let adults_i = idx("adults")?;
    let children_i = idx("children")?;
    let babies_i = idx("babies")?;
    let is_repeated_guest_i = headers.iter().position(|h| h == "is_repeated_guest");
    let total_of_special_requests_i = headers.iter().position(|h| h == "total_of_special_requests");
    let required_car_parking_spaces_i = headers.iter().position(|h| h == "required_car_parking_spaces");
    let market_segment_i = headers.iter().position(|h| h == "market_segment");
    let distribution_channel_i = headers.iter().position(|h| h == "distribution_channel");
    let country_i = headers.iter().position(|h| h == "country");
    let adr_i = headers.iter().position(|h| h == "adr");
    let reserved_room_type_i = headers.iter().position(|h| h == "reserved_room_type");
    let assigned_room_type_i = headers.iter().position(|h| h == "assigned_room_type");
    let deposit_type_i = headers.iter().position(|h| h == "deposit_type");
    let customer_type_i = headers.iter().position(|h| h == "customer_type");
    let reservation_status_i = headers.iter().position(|h| h == "reservation_status");
    let reservation_status_date_i = headers.iter().position(|h| h == "reservation_status_date");

    // We support multi-property files by creating one import row per distinct hotel value.
    #[derive(Default)]
    struct Working {
        property_id: i64,
        import_id: Option<i64>,
        blocked_duplicate: bool,
        blocked_message: Option<String>,
        rows_imported: u64,
        rows_rejected: u64,
    }

    let tx = conn.transaction().map_err(|e| format!("failed to start tx: {e}"))?;
    let mut by_property: HashMap<String, Working> = HashMap::new();

    let mut stmt = tx
        .prepare(
            r#"
INSERT INTO booking_observations (
  import_id,
  property_id,
  booking_key,
  arrival_date,
  departure_date,
  nights,
  is_canceled,
  lead_time,
  adr,
  est_revenue,

  is_repeated_guest,
  total_of_special_requests,
  required_car_parking_spaces,

  market_segment,
  distribution_channel,
  country,
  reserved_room_type,
  assigned_room_type,
  deposit_type,
  customer_type,
  reservation_status,
  reservation_status_date
)
VALUES (
  ?,
  ?,
  ?,
  CAST(? AS DATE),
  CAST(? AS DATE),
  ?,
  ?,
  ?,
  ?,
  ?,

  ?,
  ?,
  ?,

  ?,
  ?,
  ?,
  ?,
  ?,
  ?,
  ?,
  ?,
  CAST(? AS DATE)
)
"#,
        )
        .map_err(|e| format!("failed to prepare insert: {e}"))?;

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => return Err(format!("failed to read csv row: {e}")),
        };

        let property_name = record.get(hotel_i).unwrap_or("").trim().to_string();
        if property_name.is_empty() {
            // No property; cannot import.
            continue;
        }

        let working = by_property.entry(property_name.clone()).or_insert_with(|| {
            let property_id = ensure_property_id(&tx, &property_name).unwrap_or(0);
            Working {
                property_id,
                ..Default::default()
            }
        });

        // If we failed to create property, reject all rows.
        if working.property_id == 0 {
            working.rows_rejected += 1;
            continue;
        }

        if working.blocked_duplicate {
            // We already decided to block this property.
            working.rows_rejected += 1;
            continue;
        }

        // Create the imports row on first row for this property.
        if working.import_id.is_none() {
            if let Some(ts) = existing_import_timestamp(&tx, working.property_id, &file_hash)? {
                working.blocked_duplicate = true;
                working.blocked_message = Some(format!(
                    "This exact file was already imported for this property at {ts}"
                ));
                working.rows_rejected += 1;
                continue;
            }

            let import_id = create_import_row(&tx, working.property_id, &file_name, &file_hash)?;
            working.import_id = Some(import_id);
        }

        let import_id = match working.import_id {
            Some(id) => id,
            None => {
                working.rows_rejected += 1;
                continue;
            }
        };

        // Parse required fields.
        let year: i32 = match record.get(year_i).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                working.rows_rejected += 1;
                continue;
            }
        };
        let month_str = record.get(month_i).unwrap_or("");
        let month: u32 = match month_to_number(month_str) {
            Some(v) => v,
            None => {
                working.rows_rejected += 1;
                continue;
            }
        };
        let day: u32 = match record.get(day_i).unwrap_or("").trim().parse() {
            Ok(v) => v,
            Err(_) => {
                working.rows_rejected += 1;
                continue;
            }
        };

        let arrival_date = match NaiveDate::from_ymd_opt(year, month, day) {
            Some(d) => d,
            None => {
                working.rows_rejected += 1;
                continue;
            }
        }; 

        let wknd: i64 = record.get(wknd_i).unwrap_or("0").trim().parse().unwrap_or(0);
        let wk: i64 = record.get(wk_i).unwrap_or("0").trim().parse().unwrap_or(0);
        let nights_i64 = wknd + wk;
        if nights_i64 < 0 {
            working.rows_rejected += 1;
            continue;
        }
        let nights: i32 = nights_i64 as i32;
        let departure_date = arrival_date + Duration::days(nights_i64);

        let is_canceled: bool = record.get(is_canceled_i).unwrap_or("0").trim() == "1";
        let lead_time: i32 = record.get(lead_time_i).unwrap_or("").trim().parse().unwrap_or(0);

        let adr: Option<f64> = adr_i
            .and_then(|i| record.get(i))
            .and_then(|s| s.trim().parse::<f64>().ok());
        let est_revenue: Option<f64> = adr.map(|a| if is_canceled { 0.0 } else { a * (nights_i64 as f64) });

        let is_repeated_guest: Option<bool> = is_repeated_guest_i
            .and_then(|i| record.get(i))
            .and_then(|s| match s.trim() {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            });
        let total_of_special_requests: Option<i32> = total_of_special_requests_i
            .and_then(|i| record.get(i))
            .and_then(|s| s.trim().parse::<i32>().ok());
        let required_car_parking_spaces: Option<i32> = required_car_parking_spaces_i
            .and_then(|i| record.get(i))
            .and_then(|s| s.trim().parse::<i32>().ok());

        let market_segment = market_segment_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let distribution_channel = distribution_channel_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let country = country_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let reserved_room_type = reserved_room_type_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let assigned_room_type = assigned_room_type_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let deposit_type = deposit_type_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let customer_type = customer_type_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let reservation_status = reservation_status_i.and_then(|i| record.get(i)).map(|s| s.to_string());
        let reservation_status_date: Option<NaiveDate> = reservation_status_date_i.and_then(|i| record.get(i)).and_then(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                // Often YYYY-MM-DD
                NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
            }
        });

        // Include fields that are stable across snapshot exports.
        let adults = record.get(adults_i).unwrap_or("").trim().to_string();
        let children = record.get(children_i).unwrap_or("").trim().to_string();
        let babies = record.get(babies_i).unwrap_or("").trim().to_string();
        let adr_key = adr.map(|v| format!("{v:.6}")).unwrap_or_else(|| "".to_string());

        let booking_key = compute_booking_key(&[
            ("hotel", property_name.clone()),
            ("arrival_date", arrival_date.format("%Y-%m-%d").to_string()),
            ("nights", nights.to_string()),
            ("lead_time", lead_time.to_string()),
            ("adults", adults),
            ("children", children),
            ("babies", babies),
            ("adr", adr_key),
            (
                "market_segment",
                market_segment.clone().unwrap_or_default(),
            ),
            (
                "distribution_channel",
                distribution_channel.clone().unwrap_or_default(),
            ),
        ]);

        let arrival_str = arrival_date.format("%Y-%m-%d").to_string();
        let departure_str = departure_date.format("%Y-%m-%d").to_string();
        let status_date_str: Option<String> = reservation_status_date
            .map(|d| d.format("%Y-%m-%d").to_string());

        // Insert; if we fail, reject the row.
        let insert_result = stmt.execute(duckdb::params![
            import_id,
            working.property_id,
            booking_key,
            arrival_str,
            departure_str,
            nights,
            is_canceled,
            lead_time,
            adr,
            est_revenue,

            is_repeated_guest,
            total_of_special_requests,
            required_car_parking_spaces,

            market_segment,
            distribution_channel,
            country,
            reserved_room_type,
            assigned_room_type,
            deposit_type,
            customer_type,
            reservation_status,
            status_date_str,
        ]);

        match insert_result {
            Ok(_) => working.rows_imported += 1,
            Err(_) => working.rows_rejected += 1,
        }
    }

    // Update imports row counts.
    for w in by_property.values() {
        if let Some(import_id) = w.import_id {
            tx.execute(
                "UPDATE imports SET row_count = ?, rejected_count = ? WHERE id = ?",
                duckdb::params![w.rows_imported as i64, w.rows_rejected as i64, import_id],
            )
            .map_err(|e| format!("failed to update imports counts: {e}"))?;
        }
    }

    tx.commit().map_err(|e| format!("failed to commit import: {e}"))?;

    let mut properties = Vec::new();
    for (name, w) in by_property {
        properties.push(ImportPropertyResult {
            property_name: name,
            property_id: w.property_id,
            import_id: w.import_id,
            blocked_duplicate: w.blocked_duplicate,
            blocked_message: w.blocked_message,
            rows_imported: w.rows_imported,
            rows_rejected: w.rows_rejected,
        });
    }
    properties.sort_by(|a, b| a.property_name.cmp(&b.property_name));

    Ok(ImportBookingsResult { file_hash, properties })
}

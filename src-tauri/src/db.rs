use std::{fs, path::{Path, PathBuf}};

use duckdb::Connection;

pub struct DbPaths {
    pub db_file: PathBuf,
}

pub fn resolve_db_paths(app_data_dir: &Path) -> DbPaths {
    // Keep this stable across app renames.
    let db_file = app_data_dir.join("hotelinsights").join("analytics.duckdb");
    DbPaths { db_file }
}

pub fn init_db(db_file: &Path) -> Result<(), String> {
    if let Some(parent) = db_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create db dir: {e}"))?;
    }

    let conn = Connection::open(db_file).map_err(|e| format!("failed to open db: {e}"))?;

    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT INTO meta (key, value)
  SELECT 'schema_version', '1'
  WHERE NOT EXISTS (SELECT 1 FROM meta WHERE key = 'schema_version');

-- DuckDB doesn't support SQLite-style rowids; use sequences for IDs.
CREATE SEQUENCE IF NOT EXISTS users_id_seq;
CREATE SEQUENCE IF NOT EXISTS properties_id_seq;
CREATE SEQUENCE IF NOT EXISTS imports_id_seq;
CREATE SEQUENCE IF NOT EXISTS booking_observations_id_seq;

CREATE TABLE IF NOT EXISTS users (
  id BIGINT PRIMARY KEY DEFAULT nextval('users_id_seq'),
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS properties (
  id BIGINT PRIMARY KEY DEFAULT nextval('properties_id_seq'),
  name TEXT NOT NULL UNIQUE,
  timezone TEXT,
  created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS imports (
  id BIGINT PRIMARY KEY DEFAULT nextval('imports_id_seq'),
  property_id BIGINT NOT NULL,
  file_name TEXT NOT NULL,
  file_hash TEXT NOT NULL,
  imported_at TIMESTAMP NOT NULL DEFAULT now(),
  row_count BIGINT NOT NULL DEFAULT 0,
  rejected_count BIGINT NOT NULL DEFAULT 0,
  blocked_reason TEXT,
  UNIQUE(property_id, file_hash)
);

CREATE TABLE IF NOT EXISTS booking_observations (
  id BIGINT PRIMARY KEY DEFAULT nextval('booking_observations_id_seq'),
  import_id BIGINT NOT NULL,
  property_id BIGINT NOT NULL,
  booking_key TEXT NOT NULL,

  arrival_date DATE NOT NULL,
  departure_date DATE NOT NULL,
  nights INTEGER NOT NULL,
  is_canceled BOOLEAN NOT NULL,

  lead_time INTEGER,
  adr DOUBLE,
  est_revenue DOUBLE,

  is_repeated_guest BOOLEAN,
  total_of_special_requests INTEGER,
  required_car_parking_spaces INTEGER,

  market_segment TEXT,
  distribution_channel TEXT,
  country TEXT,

  reserved_room_type TEXT,
  assigned_room_type TEXT,
  deposit_type TEXT,
  customer_type TEXT,
  reservation_status TEXT,
  reservation_status_date DATE
);

-- Lightweight migrations (additive)
ALTER TABLE booking_observations ADD COLUMN IF NOT EXISTS is_repeated_guest BOOLEAN;
ALTER TABLE booking_observations ADD COLUMN IF NOT EXISTS total_of_special_requests INTEGER;
ALTER TABLE booking_observations ADD COLUMN IF NOT EXISTS required_car_parking_spaces INTEGER;

-- Stable schema view (avoid SELECT * so ALTER TABLE won't break it)
DROP VIEW IF EXISTS bookings_latest;
CREATE VIEW bookings_latest AS
  SELECT
    bo.id,
    bo.import_id,
    bo.property_id,
    bo.booking_key,
    bo.arrival_date,
    bo.departure_date,
    bo.nights,
    bo.is_canceled,
    bo.lead_time,
    bo.adr,
    bo.est_revenue,
    bo.is_repeated_guest,
    bo.total_of_special_requests,
    bo.required_car_parking_spaces,
    bo.market_segment,
    bo.distribution_channel,
    bo.country,
    bo.reserved_room_type,
    bo.assigned_room_type,
    bo.deposit_type,
    bo.customer_type,
    bo.reservation_status,
    bo.reservation_status_date
  FROM booking_observations bo
  QUALIFY ROW_NUMBER() OVER (
    PARTITION BY bo.property_id, bo.booking_key
    ORDER BY bo.id DESC
  ) = 1;
"#,
    )
    .map_err(|e| format!("failed to initialize schema: {e}"))?;

    Ok(())
}

pub fn open_db(db_file: &Path) -> Result<Connection, String> {
    Connection::open(db_file).map_err(|e| format!("failed to open db: {e}"))
}

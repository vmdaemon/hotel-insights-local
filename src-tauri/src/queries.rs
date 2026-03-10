use std::path::Path;

use crate::db;
use duckdb::{params_from_iter, types::Value};

#[derive(serde::Serialize)]
pub struct Property {
    pub id: i64,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct OverviewMetrics {
    pub bookings_total: i64,
    pub bookings_canceled: i64,
    pub cancellation_rate: f64,

    pub room_nights: i64,
    pub avg_los: f64,
    pub adr_avg: Option<f64>,
    pub est_revenue: Option<f64>,

    pub min_arrival_date: Option<String>,
    pub max_arrival_date: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ArrivalsByMonthRow {
    pub month: String,
    pub arrivals: i64,
}

#[derive(serde::Serialize)]
pub struct DailyArrivalsRow {
    pub date: String,
    pub arrivals: i64,
}

#[derive(serde::Serialize)]
pub struct CancellationByMonthRow {
    pub month: String,
    pub bookings_total: i64,
    pub bookings_canceled: i64,
    pub cancellation_rate: f64,
}

#[derive(serde::Serialize)]
pub struct CategoricalBreakdownRow {
    pub key: String,
    pub count: i64,
    pub share: f64,
}

#[derive(serde::Serialize)]
pub struct KpiCard {
    pub key: String,
    pub label: String,
    pub value: Option<f64>,
    pub previous_value: Option<f64>,
    pub delta: Option<f64>,
    pub delta_pct: Option<f64>,
}

#[derive(serde::Serialize)]
pub struct KpiDashboard {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub previous_start_date: Option<String>,
    pub previous_end_date: Option<String>,
    pub cards: Vec<KpiCard>,
}

pub fn list_properties(db_file: &Path) -> Result<Vec<Property>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    let mut stmt = conn
        .prepare("SELECT id, name FROM properties ORDER BY name")
        .map_err(|e| format!("failed to prepare properties query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Property {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| format!("failed to query properties: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("failed to read property row: {e}"))?);
    }
    Ok(out)
}

fn build_in_clause(prefix: &str, count: usize) -> String {
    // e.g. "property_id IN (?, ?, ?)"
    let placeholders = std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{prefix} IN ({placeholders})")
}

#[derive(Default)]
struct KpiAgg {
    arrivals: f64,
    cancellation_rate: f64,
    room_nights: f64,
    adr_avg: Option<f64>,
    est_revenue: Option<f64>,
    lead_time_avg: Option<f64>,
    lead_time_median: Option<f64>,
    upgrade_rate: Option<f64>,
    repeat_guest_rate: Option<f64>,
    special_requests_avg: Option<f64>,
}

fn kpi_agg(
    conn: &duckdb::Connection,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<KpiAgg, String> {
    if property_ids.is_empty() {
        return Ok(KpiAgg::default());
    }

    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();
    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
SELECT
  COUNT(*) FILTER (WHERE NOT is_canceled) AS arrivals,
  COALESCE(AVG(CASE WHEN is_canceled THEN 1.0 ELSE 0.0 END), 0.0) AS cancellation_rate,
  COALESCE(SUM(CASE WHEN NOT is_canceled THEN nights ELSE 0 END), 0) AS room_nights,
  AVG(CASE WHEN NOT is_canceled THEN adr ELSE NULL END) AS adr_avg,
  COALESCE(SUM(CASE WHEN NOT is_canceled THEN est_revenue ELSE 0 END), 0.0) AS est_revenue,
  AVG(CASE WHEN NOT is_canceled THEN lead_time ELSE NULL END) AS lead_time_avg,
  median(CASE WHEN NOT is_canceled THEN lead_time ELSE NULL END) AS lead_time_median,
  AVG(CASE WHEN NOT is_canceled AND assigned_room_type IS NOT NULL AND reserved_room_type IS NOT NULL AND assigned_room_type != reserved_room_type THEN 1.0
           WHEN NOT is_canceled AND assigned_room_type IS NOT NULL AND reserved_room_type IS NOT NULL THEN 0.0
           ELSE NULL END) AS upgrade_rate,
  AVG(CASE WHEN NOT is_canceled THEN CAST(is_repeated_guest AS DOUBLE) ELSE NULL END) AS repeat_guest_rate,
  AVG(CASE WHEN NOT is_canceled THEN CAST(total_of_special_requests AS DOUBLE) ELSE NULL END) AS special_requests_avg
FROM bookings_latest
WHERE {where_sql}
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare KPI agg: {e}"))?;

    stmt.query_row(params_from_iter(params.iter()), |row| {
        Ok(KpiAgg {
            arrivals: row.get::<_, i64>(0)? as f64,
            cancellation_rate: row.get::<_, f64>(1)?,
            room_nights: row.get::<_, i64>(2)? as f64,
            adr_avg: row.get::<_, Option<f64>>(3)?,
            est_revenue: row.get::<_, Option<f64>>(4)?,
            lead_time_avg: row.get::<_, Option<f64>>(5)?,
            lead_time_median: row.get::<_, Option<f64>>(6)?,
            upgrade_rate: row.get::<_, Option<f64>>(7)?,
            repeat_guest_rate: row.get::<_, Option<f64>>(8)?,
            special_requests_avg: row.get::<_, Option<f64>>(9)?,
        })
    })
    .map_err(|e| format!("failed to execute KPI agg: {e}"))
}

fn make_card(key: &str, label: &str, value: Option<f64>, previous: Option<f64>) -> KpiCard {
    let (delta, delta_pct) = match (value, previous) {
        (Some(v), Some(p)) => {
            let d = v - p;
            let pct = if p.abs() < f64::EPSILON { None } else { Some(d / p) };
            (Some(d), pct)
        }
        _ => (None, None),
    };

    KpiCard {
        key: key.to_string(),
        label: label.to_string(),
        value,
        previous_value: previous,
        delta,
        delta_pct,
    }
}

pub fn kpi_dashboard(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
    compare_previous_period: bool,
) -> Result<KpiDashboard, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    // Determine effective date range.
    let bounds = overview_metrics(db_file, property_ids, start_date, end_date)?;
    let start = bounds.min_arrival_date.clone();
    let end = bounds.max_arrival_date.clone();

    let current = kpi_agg(&conn, property_ids, start.as_deref(), end.as_deref())?;

    let mut prev_agg: Option<KpiAgg> = None;
    let mut prev_start: Option<String> = None;
    let mut prev_end: Option<String> = None;

    if compare_previous_period {
        if let (Some(s), Some(e)) = (start.as_deref(), end.as_deref()) {
            // previous period = same length immediately before start
            let s_date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|e| format!("invalid start_date: {e}"))?;
            let e_date = chrono::NaiveDate::parse_from_str(e, "%Y-%m-%d")
                .map_err(|e| format!("invalid end_date: {e}"))?;
            let len_days = (e_date - s_date).num_days() + 1;
            let prev_e_date = s_date - chrono::Duration::days(1);
            let prev_s_date = prev_e_date - chrono::Duration::days(len_days - 1);
            prev_start = Some(prev_s_date.format("%Y-%m-%d").to_string());
            prev_end = Some(prev_e_date.format("%Y-%m-%d").to_string());
            prev_agg = Some(kpi_agg(
                &conn,
                property_ids,
                prev_start.as_deref(),
                prev_end.as_deref(),
            )?);
        }
    }

    let prev = prev_agg;

    let cards = vec![
        make_card(
            "arrivals",
            "Arrivals (non-canceled)",
            Some(current.arrivals),
            prev.as_ref().map(|p| p.arrivals),
        ),
        make_card(
            "cancellation_rate",
            "Cancellation rate",
            Some(current.cancellation_rate),
            prev.as_ref().map(|p| p.cancellation_rate),
        ),
        make_card(
            "room_nights",
            "Room-nights",
            Some(current.room_nights),
            prev.as_ref().map(|p| p.room_nights),
        ),
        make_card(
            "adr_avg",
            "ADR avg",
            current.adr_avg,
            prev.as_ref().and_then(|p| p.adr_avg),
        ),
        make_card(
            "est_revenue",
            "Est revenue",
            current.est_revenue,
            prev.as_ref().and_then(|p| p.est_revenue),
        ),
        make_card(
            "lead_time_avg",
            "Lead time avg (days)",
            current.lead_time_avg,
            prev.as_ref().and_then(|p| p.lead_time_avg),
        ),
        make_card(
            "lead_time_median",
            "Lead time median (days)",
            current.lead_time_median,
            prev.as_ref().and_then(|p| p.lead_time_median),
        ),
        make_card(
            "upgrade_rate",
            "Room type change rate",
            current.upgrade_rate,
            prev.as_ref().and_then(|p| p.upgrade_rate),
        ),
        make_card(
            "repeat_guest_rate",
            "Repeat guest rate",
            current.repeat_guest_rate,
            prev.as_ref().and_then(|p| p.repeat_guest_rate),
        ),
        make_card(
            "special_requests_avg",
            "Special requests avg",
            current.special_requests_avg,
            prev.as_ref().and_then(|p| p.special_requests_avg),
        ),
    ];

    Ok(KpiDashboard {
        start_date: start,
        end_date: end,
        previous_start_date: prev_start,
        previous_end_date: prev_end,
        cards,
    })
}

pub fn overview_metrics(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<OverviewMetrics, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    if property_ids.is_empty() {
        return Ok(OverviewMetrics {
            bookings_total: 0,
            bookings_canceled: 0,
            cancellation_rate: 0.0,
            room_nights: 0,
            avg_los: 0.0,
            adr_avg: None,
            est_revenue: None,
            min_arrival_date: None,
            max_arrival_date: None,
        });
    }

    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();

    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
SELECT
  COUNT(*) AS bookings_total,
  SUM(CASE WHEN is_canceled THEN 1 ELSE 0 END) AS bookings_canceled,

  SUM(CASE WHEN NOT is_canceled THEN nights ELSE 0 END) AS room_nights,
  AVG(CASE WHEN NOT is_canceled THEN nights ELSE NULL END) AS avg_los,
  AVG(CASE WHEN NOT is_canceled THEN adr ELSE NULL END) AS adr_avg,
  SUM(CASE WHEN NOT is_canceled THEN est_revenue ELSE NULL END) AS est_revenue,

  CAST(MIN(arrival_date) AS VARCHAR) AS min_arrival_date,
  CAST(MAX(arrival_date) AS VARCHAR) AS max_arrival_date
FROM bookings_latest
WHERE {where_sql}
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare overview query: {e}"))?;

    let row = stmt
        .query_row(params_from_iter(params.iter()), |row| {
            let bookings_total: i64 = row.get(0)?;
            let bookings_canceled: i64 = row.get(1)?;

            let room_nights: i64 = row.get(2)?;
            let avg_los: Option<f64> = row.get(3)?;
            let adr_avg: Option<f64> = row.get(4)?;
            let est_revenue: Option<f64> = row.get(5)?;

            let min_arrival_date: Option<String> = row.get(6)?;
            let max_arrival_date: Option<String> = row.get(7)?;

            let cancellation_rate = if bookings_total > 0 {
                bookings_canceled as f64 / bookings_total as f64
            } else {
                0.0
            };

            Ok(OverviewMetrics {
                bookings_total,
                bookings_canceled,
                cancellation_rate,
                room_nights,
                avg_los: avg_los.unwrap_or(0.0),
                adr_avg,
                est_revenue,
                min_arrival_date,
                max_arrival_date,
            })
        })
        .map_err(|e| format!("failed to execute overview query: {e}"))?;

    Ok(row)
}

pub fn arrivals_by_month(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<ArrivalsByMonthRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    if property_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));
    where_clauses.push("NOT is_canceled".to_string());

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();

    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
SELECT
  strftime(arrival_date, '%Y-%m') AS month,
  COUNT(*) AS arrivals
FROM bookings_latest
WHERE {where_sql}
GROUP BY 1
ORDER BY 1
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare arrivals query: {e}"))?;

    let rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(ArrivalsByMonthRow {
                month: row.get(0)?,
                arrivals: row.get(1)?,
            })
        })
        .map_err(|e| format!("failed to execute arrivals query: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("failed to read arrivals row: {e}"))?);
    }
    Ok(out)
}

pub fn daily_arrivals(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<DailyArrivalsRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;
    if property_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));
    where_clauses.push("NOT is_canceled".to_string());

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();
    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
SELECT
  CAST(arrival_date AS VARCHAR) AS date,
  COUNT(*) AS arrivals
FROM bookings_latest
WHERE {where_sql}
GROUP BY 1
ORDER BY 1
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare daily arrivals query: {e}"))?;

    let rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(DailyArrivalsRow {
                date: row.get(0)?,
                arrivals: row.get(1)?,
            })
        })
        .map_err(|e| format!("failed to execute daily arrivals query: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("failed to read daily arrivals row: {e}"))?);
    }
    Ok(out)
}

pub fn cancellations_by_month(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<CancellationByMonthRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;
    if property_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();
    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
SELECT
  strftime(arrival_date, '%Y-%m') AS month,
  COUNT(*) AS bookings_total,
  SUM(CASE WHEN is_canceled THEN 1 ELSE 0 END) AS bookings_canceled
FROM bookings_latest
WHERE {where_sql}
GROUP BY 1
ORDER BY 1
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare cancellations query: {e}"))?;

    let rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            let bookings_total: i64 = row.get(1)?;
            let bookings_canceled: i64 = row.get(2)?;
            let cancellation_rate = if bookings_total > 0 {
                bookings_canceled as f64 / bookings_total as f64
            } else {
                0.0
            };
            Ok(CancellationByMonthRow {
                month: row.get(0)?,
                bookings_total,
                bookings_canceled,
                cancellation_rate,
            })
        })
        .map_err(|e| format!("failed to execute cancellations query: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("failed to read cancellations row: {e}"))?);
    }
    Ok(out)
}

fn categorical_breakdown(
    conn: &duckdb::Connection,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
    column: &str,
    limit: usize,
) -> Result<Vec<CategoricalBreakdownRow>, String> {
    let mut where_clauses: Vec<String> = Vec::new();
    where_clauses.push(build_in_clause("property_id", property_ids.len()));
    where_clauses.push("NOT is_canceled".to_string());
    where_clauses.push(format!("{column} IS NOT NULL"));
    where_clauses.push(format!("{column} != ''"));

    let mut params: Vec<Value> = property_ids.iter().map(|id| Value::BigInt(*id)).collect();
    if let Some(sd) = start_date {
        where_clauses.push("arrival_date >= CAST(? AS DATE)".to_string());
        params.push(Value::Text(sd.to_string()));
    }
    if let Some(ed) = end_date {
        where_clauses.push("arrival_date <= CAST(? AS DATE)".to_string());
        params.push(Value::Text(ed.to_string()));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"
WITH base AS (
  SELECT {column} AS key
  FROM bookings_latest
  WHERE {where_sql}
), totals AS (
  SELECT COUNT(*)::DOUBLE AS total FROM base
)
SELECT
  key,
  COUNT(*) AS count,
  (COUNT(*)::DOUBLE / (SELECT total FROM totals)) AS share
FROM base
GROUP BY 1
ORDER BY 2 DESC
LIMIT {limit}
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare breakdown query: {e}"))?;

    let rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(CategoricalBreakdownRow {
                key: row.get(0)?,
                count: row.get(1)?,
                share: row.get(2)?,
            })
        })
        .map_err(|e| format!("failed to execute breakdown query: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("failed to read breakdown row: {e}"))?);
    }
    Ok(out)
}

pub fn market_segment_mix(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: usize,
) -> Result<Vec<CategoricalBreakdownRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;
    if property_ids.is_empty() {
        return Ok(vec![]);
    }
    categorical_breakdown(
        &conn,
        property_ids,
        start_date,
        end_date,
        "market_segment",
        limit,
    )
}

pub fn distribution_channel_mix(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: usize,
) -> Result<Vec<CategoricalBreakdownRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;
    if property_ids.is_empty() {
        return Ok(vec![]);
    }
    categorical_breakdown(
        &conn,
        property_ids,
        start_date,
        end_date,
        "distribution_channel",
        limit,
    )
}

pub fn country_mix(
    db_file: &Path,
    property_ids: &[i64],
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: usize,
) -> Result<Vec<CategoricalBreakdownRow>, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;
    if property_ids.is_empty() {
        return Ok(vec![]);
    }
    categorical_breakdown(&conn, property_ids, start_date, end_date, "country", limit)
}

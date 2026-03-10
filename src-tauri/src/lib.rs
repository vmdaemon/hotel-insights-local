pub mod db;
mod auth;
mod csv_preview;
pub mod import_bookings;
pub mod queries;

use tauri::Manager;
use std::sync::Mutex;

#[derive(serde::Serialize)]
struct DbInitResult {
    db_path: String,
}

#[derive(Default)]
struct SessionState(Mutex<Option<String>>);

#[tauri::command]
fn db_init(app: tauri::AppHandle) -> Result<DbInitResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

    let paths = db::resolve_db_paths(&app_data_dir);
    db::init_db(&paths.db_file)?;

    Ok(DbInitResult {
        db_path: paths.db_file.to_string_lossy().to_string(),
    })
}

fn resolve_db_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(db::resolve_db_paths(&app_data_dir).db_file)
}

#[tauri::command]
fn auth_status(app: tauri::AppHandle, session: tauri::State<'_, SessionState>) -> Result<auth::AuthStatus, String> {
    let db_file = resolve_db_file(&app)?;
    let logged_in_user = session.0.lock().map_err(|_| "session lock poisoned".to_string())?.clone();
    auth::auth_status(&db_file, logged_in_user)
}

#[tauri::command]
fn auth_bootstrap_create_admin(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    username: String,
    password: String,
) -> Result<(), String> {
    let db_file = resolve_db_file(&app)?;
    auth::bootstrap_create_admin(&db_file, &username, &password)?;
    *session.0.lock().map_err(|_| "session lock poisoned".to_string())? = Some(username);
    Ok(())
}

#[tauri::command]
fn auth_login(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    username: String,
    password: String,
) -> Result<(), String> {
    let db_file = resolve_db_file(&app)?;
    auth::login(&db_file, &username, &password)?;
    *session.0.lock().map_err(|_| "session lock poisoned".to_string())? = Some(username);
    Ok(())
}

#[tauri::command]
fn auth_logout(session: tauri::State<'_, SessionState>) -> Result<(), String> {
    *session.0.lock().map_err(|_| "session lock poisoned".to_string())? = None;
    Ok(())
}

#[tauri::command]
fn preview_bookings_csv(file_path: String, max_rows: Option<u32>) -> Result<csv_preview::CsvPreview, String> {
    let max_rows = max_rows.unwrap_or(50) as usize;
    csv_preview::preview_csv(std::path::Path::new(&file_path), max_rows)
}

#[tauri::command]
fn import_bookings_csv(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    file_path: String,
) -> Result<import_bookings::ImportBookingsResult, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();

    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }

    let db_file = resolve_db_file(&app)?;
    import_bookings::import_bookings_csv(&db_file, std::path::Path::new(&file_path))
}

#[tauri::command]
fn list_properties(app: tauri::AppHandle, session: tauri::State<'_, SessionState>) -> Result<Vec<queries::Property>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::list_properties(&db_file)
}

#[tauri::command]
fn overview_metrics(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<queries::OverviewMetrics, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::overview_metrics(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
    )
}

#[tauri::command]
fn arrivals_by_month(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<queries::ArrivalsByMonthRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::arrivals_by_month(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
    )
}

#[tauri::command]
fn kpi_dashboard(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    compare_previous_period: Option<bool>,
) -> Result<queries::KpiDashboard, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::kpi_dashboard(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
        compare_previous_period.unwrap_or(true),
    )
}

#[tauri::command]
fn daily_arrivals(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<queries::DailyArrivalsRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::daily_arrivals(&db_file, &property_ids, start_date.as_deref(), end_date.as_deref())
}

#[tauri::command]
fn cancellations_by_month(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<queries::CancellationByMonthRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::cancellations_by_month(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
    )
}

#[tauri::command]
fn market_segment_mix(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<queries::CategoricalBreakdownRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::market_segment_mix(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
        limit.unwrap_or(10) as usize,
    )
}

#[tauri::command]
fn distribution_channel_mix(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<queries::CategoricalBreakdownRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::distribution_channel_mix(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
        limit.unwrap_or(10) as usize,
    )
}

#[tauri::command]
fn country_mix(
    app: tauri::AppHandle,
    session: tauri::State<'_, SessionState>,
    property_ids: Vec<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<queries::CategoricalBreakdownRow>, String> {
    let logged_in_user = session
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?
        .clone();
    if logged_in_user.is_none() {
        return Err("not authenticated".to_string());
    }
    let db_file = resolve_db_file(&app)?;
    queries::country_mix(
        &db_file,
        &property_ids,
        start_date.as_deref(),
        end_date.as_deref(),
        limit.unwrap_or(10) as usize,
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SessionState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            db_init,
            auth_status,
            auth_bootstrap_create_admin,
            auth_login,
            auth_logout,
            preview_bookings_csv,
            import_bookings_csv,
            list_properties,
            overview_metrics,
            arrivals_by_month,
            kpi_dashboard,
            daily_arrivals,
            cancellations_by_month,
            market_segment_mix,
            distribution_channel_mix,
            country_mix
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

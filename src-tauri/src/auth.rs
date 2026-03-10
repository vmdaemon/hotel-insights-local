use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use rand_core::OsRng;

use crate::db;

#[derive(serde::Serialize)]
pub struct AuthStatus {
    pub has_admin: bool,
    pub logged_in_user: Option<String>,
}

pub fn auth_status(db_file: &std::path::Path, logged_in_user: Option<String>) -> Result<AuthStatus, String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    let has_admin: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
        .map_err(|e| format!("failed to query users: {e}"))?;

    Ok(AuthStatus {
        has_admin,
        logged_in_user,
    })
}

pub fn bootstrap_create_admin(db_file: &std::path::Path, username: &str, password: &str) -> Result<(), String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    let has_admin: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
        .map_err(|e| format!("failed to query users: {e}"))?;
    if has_admin {
        return Err("admin already exists".to_string());
    }

    if username.trim().is_empty() {
        return Err("username required".to_string());
    }
    if password.is_empty() {
        return Err("password required".to_string());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("failed to hash password: {e}"))?
        .to_string();

    conn.execute(
        "INSERT INTO users (username, password_hash) VALUES (?, ?)",
        [username, &password_hash],
    )
    .map_err(|e| format!("failed to create admin: {e}"))?;

    Ok(())
}

pub fn login(db_file: &std::path::Path, username: &str, password: &str) -> Result<(), String> {
    db::init_db(db_file)?;
    let conn = db::open_db(db_file)?;

    let stored_hash: String = conn
        .query_row(
            "SELECT password_hash FROM users WHERE username = ? LIMIT 1",
            [username],
            |row| row.get(0),
        )
        .map_err(|_| "invalid username or password".to_string())?;

    let parsed_hash = PasswordHash::new(&stored_hash)
        .map_err(|_| "invalid username or password".to_string())?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "invalid username or password".to_string())?;

    Ok(())
}

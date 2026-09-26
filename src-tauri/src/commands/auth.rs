use sqlx::PgPool;
use tauri::State;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserSession {
    pub user_id: i32,
    pub role_name: String,
    pub first_name: String,
    pub last_name: String,
}

pub struct ActiveSessions(pub Mutex<HashMap<String, UserSession>>);

#[tauri::command]
pub async fn login(
    pin: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<serde_json::Value, String> {

    let user = sqlx::query(
        "
        SELECT
            u.user_id,
            u.role_id,
            u.first_name,
            u.last_name,
            r.role_name
        FROM app_user u
        JOIN role r ON u.role_id = r.role_id
        WHERE u.password_hash = crypt($1, u.password_hash)
        AND u.active = true
        "
    )
        .bind(pin)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    match user {
        Some(row) => {
            let first_name: String = row.get::<Option<String>, _>("first_name").unwrap_or_default();
            let last_name: String = row.get::<Option<String>, _>("last_name").unwrap_or_default();
            let role_name: String = row.get("role_name");
            let user_id: i32 = row.get("user_id");
            let role_id: i32 = row.get("role_id");

            let token = Uuid::new_v4().to_string();

            let mut lock = sessions.0.lock().map_err(|_| "Грешка со меморијата на сесиите".to_string())?;
            lock.insert(
                token.clone(),
                UserSession {
                    user_id,
                    role_name: role_name.clone(),
                    first_name: first_name.clone(),
                    last_name: last_name.clone(),
                },
            );

            let full_name = format!("{} {}", first_name, last_name).trim().to_string();
            let display_name = if full_name.is_empty() { "Корисник".to_string() } else { full_name };

            Ok(serde_json::json!({
                "token": token,
                "user_id": user_id,
                "role_id": role_id,
                "role_name": role_name,
                "first_name": first_name,
                "last_name": last_name,
                "name": display_name
            }))
        }

        None => Err("Погрешен PIN".to_string())
    }
}

pub async fn require_admin_session(
    token: &str,
    sessions: &ActiveSessions,
) -> Result<UserSession, String> {
    let lock = sessions.0.lock().map_err(|_| "Грешка со сесиите".to_string())?;

    if let Some(session) = lock.get(token) {
        let up = session.role_name.to_uppercase();
        if up.contains("АДМИН") || up.contains("ADMIN") {
            return Ok(session.clone());
        }
        return Err("Немате дозвола за оваа акција (Потребен е Админ)".to_string());
    }

    Err("Невалидна или истечена сесија. Најавете се повторно.".to_string())
}

pub async fn require_manager_session(
    token: &str,
    sessions: &ActiveSessions,
) -> Result<UserSession, String> {
    let lock = sessions.0.lock().map_err(|_| "Грешка со сесиите".to_string())?;

    if let Some(session) = lock.get(token) {
        let up = session.role_name.to_uppercase();
        if up.contains("АДМИН") || up.contains("ADMIN")
            || up.contains("МЕНАЏЕР") || up.contains("MANAGER") {
            return Ok(session.clone());
        }
        return Err("Немате дозвола за оваа акција (Потребен е Менаџер или Админ)".to_string());
    }

    Err("Невалидна или истечена сесија. Најавете се повторно.".to_string())
}
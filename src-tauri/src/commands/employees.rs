use sqlx::{PgPool, Row};
use tauri::State;

use crate::commands::auth::{require_manager_session, ActiveSessions};

#[tauri::command]
pub async fn get_employees(
    token: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<serde_json::Value>, String> {
    require_manager_session(&token, sessions.inner()).await?;

    let rows = sqlx::query(
        "
        SELECT
            u.user_id,
            u.role_id,
            u.first_name,
            u.last_name,
            u.active,
            r.role_name
        FROM app_user u
        JOIN role r ON u.role_id = r.role_id
        ORDER BY u.user_id
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "id": row.get::<i32, _>("user_id"),
            "role_id": row.get::<i32, _>("role_id"),
            "first_name": row.get::<String, _>("first_name"),
            "last_name": row.get::<String, _>("last_name"),
            "active": row.get::<bool, _>("active"),
            "role_name": row.get::<String, _>("role_name"),
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_roles(
    pool: State<'_, PgPool>
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query("SELECT role_id, role_name FROM role")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "role_id": row.get::<i32, _>("role_id"),
            "role_name": row.get::<String, _>("role_name")
        }));
    }
    Ok(result)
}

#[tauri::command]
pub async fn add_employee(
    token: String,
    first_name: String,
    last_name: String,
    pin: String,
    role_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    if first_name.trim().is_empty() || last_name.trim().is_empty() || pin.trim().is_empty() {
        return Err("Сите полиња се задолжителни".to_string());
    }

    sqlx::query(
        "
        INSERT INTO app_user (first_name, last_name, password_hash, role_id, active)
        VALUES ($1, $2, crypt($3, gen_salt('bf')), $4, true)
        "
    )
        .bind(first_name)
        .bind(last_name)
        .bind(pin)
        .bind(role_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_employee_role(
    token: String,
    user_id: i32,
    role_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    sqlx::query(
        "
        UPDATE app_user
        SET role_id = $1
        WHERE user_id = $2
        "
    )
        .bind(role_id)
        .bind(user_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn reset_employee_pin(
    token: String,
    user_id: i32,
    new_pin: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    if new_pin.trim().is_empty() {
        return Err("PIN не смее да биде празен".to_string());
    }

    sqlx::query(
        "
        UPDATE app_user
        SET password_hash = crypt($1, gen_salt('bf'))
        WHERE user_id = $2
        "
    )
        .bind(new_pin)
        .bind(user_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_employee(
    token: String,
    user_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    sqlx::query(
        "
        UPDATE app_user
        SET active = NOT active
        WHERE user_id = $1
        "
    )
        .bind(user_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
#[tauri::command]
pub async fn delete_employee(
    token: String,
    user_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    let result = sqlx::query("DELETE FROM app_user WHERE user_id = $1")
        .bind(user_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Вработениот не е пронајден".to_string());
    }

    Ok(())
}
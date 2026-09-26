use sqlx::PgPool;
use tauri::State;

use crate::commands::auth::{require_manager_session, ActiveSessions};
use crate::models::category::Category;

#[tauri::command]
pub async fn get_categories(
    pool: State<'_, PgPool>
) -> Result<Vec<Category>, String> {
    let categories = sqlx::query_as::<_, Category>(
        "
        SELECT
            category_id,
            name,
            description
        FROM category
        WHERE active = true
        ORDER BY name
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

#[tauri::command]
pub async fn create_category(
    token: String,
    name: String,
    description: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    if name.trim().is_empty() {
        return Err("Името е задолжително".to_string());
    }

    sqlx::query(
        "
        INSERT INTO category (name, description)
        VALUES ($1, $2)
        "
    )
        .bind(name)
        .bind(description)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_category(
    token: String,
    category_id: i32,
    name: String,
    description: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    sqlx::query(
        "
        UPDATE category
        SET name = $1,
            description = $2
        WHERE category_id = $3
        "
    )
        .bind(name)
        .bind(description)
        .bind(category_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_category(
    token: String,
    category_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    sqlx::query(
        "
        UPDATE category
        SET active = false
        WHERE category_id = $1
        "
    )
        .bind(category_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
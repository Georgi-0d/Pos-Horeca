use sqlx::PgPool;
use tauri::State;

use crate::commands::auth::{require_manager_session, ActiveSessions};

#[tauri::command]
pub async fn add_product(
    token: String,
    category_id: i32,
    name: String,
    description: String,
    price: f64,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {

    require_manager_session(&token, sessions.inner()).await?;

    if name.trim().is_empty() {
        return Err("Името е задолжително".to_string());
    }

    if price < 0.0 {
        return Err("Цената не смее да биде негативна".to_string());
    }

    sqlx::query(
        "
        INSERT INTO product
        (
            category_id,
            name,
            description,
            price
        )
        VALUES
        ($1,$2,$3,$4)
        "
    )
        .bind(category_id)
        .bind(name)
        .bind(description)
        .bind(price)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_product(
    token: String,
    product_id: i32,
    category_id: i32,
    name: String,
    description: String,
    price: f64,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {

    require_manager_session(&token, sessions.inner()).await?;

    if name.trim().is_empty() {
        return Err("Името е задолжително".to_string());
    }

    if price < 0.0 {
        return Err("Цената не смее да биде негативна".to_string());
    }

    sqlx::query(
        "
        UPDATE product
        SET
            category_id = $1,
            name = $2,
            description = $3,
            price = $4
        WHERE product_id = $5
        "
    )
        .bind(category_id)
        .bind(name)
        .bind(description)
        .bind(price)
        .bind(product_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_product(
    token: String,
    product_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {

    require_manager_session(&token, sessions.inner()).await?;

    sqlx::query(
        "
        UPDATE product
        SET active=false
        WHERE product_id=$1
        "
    )
        .bind(product_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_product_price(
    token: String,
    product_id: i32,
    price: f64,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {

    require_manager_session(&token, sessions.inner()).await?;

    if price < 0.0 {
        return Err("Цената не смее да биде негативна".to_string());
    }

    sqlx::query(
        "
        UPDATE product
        SET price=$1
        WHERE product_id=$2
        "
    )
        .bind(price)
        .bind(product_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
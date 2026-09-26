use sqlx::{PgPool, Row};
use tauri::State;
use rust_decimal::Decimal;

use crate::commands::auth::{require_manager_session, ActiveSessions};

#[tauri::command]
pub async fn today_sales(
    pool: State<'_, PgPool>
) -> Result<Decimal, String> {
    let total: Decimal = sqlx::query_scalar(
        "
        SELECT COALESCE(SUM(amount), 0)
        FROM payment
        WHERE payment_date::date = CURRENT_DATE
        "
    )
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(total)
}

#[tauri::command]
pub async fn today_orders(
    pool: State<'_, PgPool>
) -> Result<i64, String> {
    let count: i64 = sqlx::query_scalar(
        "
        SELECT COUNT(*)
        FROM orders
        WHERE created_at::date = CURRENT_DATE
        "
    )
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub async fn best_product(
    pool: State<'_, PgPool>
) -> Result<String, String> {
    let product: Option<String> = sqlx::query_scalar(
        "
        SELECT p.name
        FROM order_item oi
        JOIN product p ON p.product_id = oi.product_id
        JOIN orders o ON o.order_id = oi.order_id
        WHERE o.status = 'ПЛАТЕНА'
        GROUP BY p.name
        ORDER BY SUM(oi.quantity) DESC
        LIMIT 1
        "
    )
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(product.unwrap_or_else(|| "-".to_string()))
}

#[tauri::command]
pub async fn z_report(
    token: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<serde_json::Value, String> {
    let session = require_manager_session(&token, sessions.inner()).await?;
    let admin_id = session.user_id;

    let mut tx = pool.begin()
        .await
        .map_err(|e| e.to_string())?;

    let last_close: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
        "SELECT MAX(closed_at) FROM shift_close"
    )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let since = last_close.unwrap_or_else(|| {
        chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    });

    let row = sqlx::query(
        "
        SELECT
            COALESCE(SUM(amount), 0) as total,
            COUNT(*) as order_count
        FROM payment
        WHERE payment_date > $1
        "
    )
        .bind(since)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let total: Decimal = row.get("total");
    let order_count: i64 = row.get("order_count");

    sqlx::query(
        "
        INSERT INTO shift_close (closed_by, total, order_count)
        VALUES ($1, $2, $3)
        "
    )
        .bind(admin_id)
        .bind(total)
        .bind(order_count as i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit()
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "total": total.to_string(),
        "order_count": order_count,
        "since": since.to_string()
    }))
}

#[tauri::command]
pub async fn employee_performance(
    token: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<serde_json::Value>, String> {
    require_manager_session(&token, sessions.inner()).await?;

    let rows = sqlx::query(
        "
        SELECT
            u.first_name,
            u.last_name,
            COALESCE(SUM(pay.amount), 0) as total,
            COUNT(pay.payment_id) as order_count
        FROM app_user u
        LEFT JOIN orders o ON o.user_id = u.user_id
        LEFT JOIN payment pay ON pay.order_id = o.order_id
        GROUP BY u.user_id, u.first_name, u.last_name
        ORDER BY total DESC
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "name": format!("{} {}", row.get::<String, _>("first_name"), row.get::<String, _>("last_name")),
            "total": row.get::<Decimal, _>("total").to_string(),
            "order_count": row.get::<i64, _>("order_count")
        }));
    }
    Ok(result)
}

#[tauri::command]
pub async fn search_orders(
    token: String,
    query: String,
    start_date: Option<String>,
    end_date: Option<String>,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<serde_json::Value>, String> {
    require_manager_session(&token, sessions.inner()).await?;

    if query.trim().is_empty() && start_date.is_none() && end_date.is_none() {
        return Ok(Vec::new());
    }

    let pattern = format!("%{}%", query.trim());

    let rows = sqlx::query(
        "
        SELECT
            i.invoice_number,
            i.issued_at,
            pay.amount,
            u.first_name,
            u.last_name
        FROM invoice i
        JOIN payment pay ON i.payment_id = pay.payment_id
        JOIN orders o ON pay.order_id = o.order_id
        JOIN app_user u ON o.user_id = u.user_id
        WHERE ($1 = '%%' OR i.invoice_number ILIKE $1 OR (u.first_name || ' ' || u.last_name) ILIKE $1)
          AND ($2::text IS NULL OR $2 = '' OR i.issued_at >= $2::timestamp)
          AND ($3::text IS NULL OR $3 = '' OR i.issued_at <= $3::timestamp)
        ORDER BY i.issued_at DESC
        LIMIT 100
        "
    )
        .bind(pattern)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "invoice": row.get::<String, _>("invoice_number"),
            "date": row.get::<chrono::DateTime<chrono::Utc>, _>("issued_at").to_string(),
            "amount": row.get::<Decimal, _>("amount").to_string(),
            "employee": format!("{} {}", row.get::<String, _>("first_name"), row.get::<String, _>("last_name"))
        }));
    }

    Ok(result)
}
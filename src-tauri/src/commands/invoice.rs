use sqlx::{PgPool, Row};
use tauri::State;
use rust_decimal::Decimal;
use crate::commands::auth::ActiveSessions;

#[tauri::command]
pub async fn create_invoice(
    token: String,
    payment_id: i32,
    invoice_number: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна сесија.".to_string());
        }
    }

    sqlx::query(
        "
        INSERT INTO invoice (payment_id, invoice_number)
        VALUES ($1, $2)
        "
    )
        .bind(payment_id)
        .bind(invoice_number)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_invoice(
    token: String,
    invoice_number: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<serde_json::Value, String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна сесија за пристап до фактури.".to_string());
        }
    }

    let settings_row = sqlx::query(
        "SELECT restaurant_name, currency, vat_rate FROM app_settings LIMIT 1"
    )
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut restaurant_name = "Ресторан".to_string();
    let mut currency = "MKD".to_string();
    let mut vat_rate_str = "0".to_string();

    if let Some(row) = settings_row {
        restaurant_name = row.get::<String, _>("restaurant_name");
        currency = row.get::<String, _>("currency");
        vat_rate_str = row.get::<Decimal, _>("vat_rate").to_string();
    }

    let rows = sqlx::query(
        r#"
        SELECT
            i.invoice_number,
            i.issued_at,
            pay.method,
            pay.amount,
            p.name as product_name,
            oi.quantity,
            oi.unit_price
        FROM invoice i
        JOIN payment pay ON i.payment_id = pay.payment_id
        JOIN orders o ON pay.order_id = o.order_id
        JOIN order_item oi ON o.order_id = oi.order_id
        JOIN product p ON oi.product_id = p.product_id
        WHERE i.invoice_number = $1
        ORDER BY oi.item_number
        "#
    )
        .bind(&invoice_number)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if rows.is_empty() {
        return Err("Фактурата не е пронајдена".to_string());
    }

    let issued_at: chrono::DateTime<chrono::Utc> = rows[0].get("issued_at");
    let issued_at_str = issued_at.to_rfc3339();
    let method = rows[0].get::<String, _>("method");
    let total_amount = rows[0].get::<Decimal, _>("amount").to_string();

    let mut items = Vec::new();
    for row in rows {
        let p_name = row.get::<String, _>("product_name");
        let qty = row.get::<i32, _>("quantity");
        let u_price = row.get::<Decimal, _>("unit_price").to_string();

        items.push(serde_json::json!({
            "name": p_name,
            "quantity": qty,
            "unit_price": u_price
        }));
    }

    Ok(serde_json::json!({
        "restaurant_name": restaurant_name,
        "currency": currency,
        "vat_rate": vat_rate_str,
        "invoice_number": invoice_number,
        "issued_at": issued_at_str,
        "method": method,
        "total_amount": total_amount,
        "items": items
    }))
}
use sqlx::{PgPool, Row};
use tauri::State;
use std::str::FromStr;
use crate::commands::auth::{require_manager_session, ActiveSessions};

#[tauri::command]
pub async fn get_settings(
    pool: State<'_, PgPool>
) -> Result<serde_json::Value, String> {
    let row = sqlx::query(
        r#"
        SELECT restaurant_name, vat_rate, currency, printer_config
        FROM app_settings
        WHERE id = 1
        "#
    )
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let vat_rate: rust_decimal::Decimal = row.get("vat_rate");

    Ok(serde_json::json!({
        "restaurant_name": row.get::<String, _>("restaurant_name"),
        "vat_rate": vat_rate.to_string(),
        "currency": row.get::<String, _>("currency"),
        "printer_config": row.get::<String, _>("printer_config")
    }))
}

#[tauri::command]
pub async fn update_settings(
    token: String,
    restaurant_name: String,
    vat_rate: f64,
    currency: String,
    printer_config: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await? ;

    if restaurant_name.trim().is_empty() {
        return Err("Името на ресторанот не смее да биде празно".to_string());
    }

    if vat_rate < 0.0 {
        return Err("ДДВ стапката не смее да биде негативна".to_string());
    }

    let vat_decimal = rust_decimal::Decimal::from_str(&vat_rate.to_string())
        .map_err(|e| format!("Грешка со конверзија на ДДВ: {}", e))?;

    sqlx::query(
        r#"
        UPDATE app_settings
        SET restaurant_name = $1,
            vat_rate = $2,
            currency = $3,
            printer_config = $4
        WHERE id = 1
        "#
    )
        .bind(restaurant_name)
        .bind(vat_decimal)
        .bind(currency)
        .bind(printer_config)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
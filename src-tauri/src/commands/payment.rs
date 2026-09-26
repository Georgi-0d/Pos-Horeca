use sqlx::PgPool;
use tauri::State;
use crate::commands::auth::ActiveSessions;

#[tauri::command]
pub async fn create_payment(
    token: String,
    order_id: i32,
    amount: rust_decimal::Decimal,
    method: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Критична грешка: Неовластена наплата без сесија!".to_string());
        }
    }

    sqlx::query(
        "
        INSERT INTO payment (order_id, amount, method)
        VALUES ($1, $2, $3)
        "
    )
        .bind(order_id)
        .bind(amount)
        .bind(method)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
use sqlx::PgPool;
use tauri::State;
use crate::models::table::RestaurantTable;
use crate::commands::auth::ActiveSessions;

pub const STATUS_FREE: &str = "СЛОБОДНА";
pub const STATUS_OCCUPIED: &str = "ЗАФАТЕНА";
pub const STATUS_RESERVED: &str = "РЕЗЕРВИРАНА";

#[tauri::command]
pub async fn get_tables(
    token: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<RestaurantTable>, String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна или истечена сесија.".to_string());
        }
    }

    let tables = sqlx::query_as::<_, RestaurantTable>(
        "
        SELECT table_id, table_number, capacity, status
        FROM restaurant_table
        ORDER BY table_id ASC
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(tables)
}

#[tauri::command]
pub async fn reserve_table(
    token: String,
    table_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна или истечена сесија.".to_string());
        }
    }

    set_table_status(table_id, STATUS_RESERVED, pool.inner()).await
}

#[tauri::command]
pub async fn cancel_reservation(
    token: String,
    table_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна или истечена сесија.".to_string());
        }
    }

    set_table_status(table_id, STATUS_FREE, pool.inner()).await
}

pub async fn set_table_status<'e, E>(
    table_id: i32,
    status: &str,
    executor: E,
) -> Result<(), String>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "
        UPDATE restaurant_table
        SET status = $1
        WHERE table_id = $2
        "
    )
        .bind(status)
        .bind(table_id)
        .execute(executor)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
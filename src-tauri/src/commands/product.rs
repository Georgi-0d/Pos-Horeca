use sqlx::PgPool;
use tauri::State;
use crate::models::product::Product;
use crate::commands::auth::ActiveSessions;

#[tauri::command]
pub async fn get_products(
    token: String,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<Product>, String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Неавторизиран пристап. Невалидна сесија.".to_string());
        }
    }

    let products = sqlx::query_as::<_, Product>(
        "
        SELECT product_id, category_id, name, description, price
        FROM product
        WHERE active = true
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(products)
}

#[tauri::command]
pub async fn get_products_by_category(
    token: String,
    category_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<Product>, String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка со сесијата".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Неавторизиран пристап. Невалидна сесија.".to_string());
        }
    }

    let products = sqlx::query_as::<_, Product>(
        "
        SELECT product_id, category_id, name, description, price
        FROM product
        WHERE category_id = $1
        AND active = true
        ORDER BY name
        "
    )
        .bind(category_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(products)
}
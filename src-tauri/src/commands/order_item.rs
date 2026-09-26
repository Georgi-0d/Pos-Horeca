use sqlx::PgPool;
use tauri::State;
use crate::models::order_item::OrderItem;
use crate::commands::auth::ActiveSessions;

#[tauri::command]
pub async fn add_order_item(
    token: String,
    order_id: i32,
    product_id: i32,
    quantity: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна сесија. Најавете се повторно.".to_string());
        }
    }

    if quantity <= 0 {
        return Err("Количината мора да биде поголема од 0".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let existing: Option<i32> = sqlx::query_scalar(
        "
        SELECT item_number
        FROM order_item
        WHERE order_id = $1
        AND product_id = $2
        "
    )
        .bind(order_id)
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(item_number) = existing {
        sqlx::query(
            "
            UPDATE order_item
            SET quantity = quantity + $3
            WHERE order_id = $1
            AND item_number = $2
            "
        )
            .bind(order_id)
            .bind(item_number)
            .bind(quantity)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        let next_item_number: i32 = sqlx::query_scalar(
            "
            SELECT COALESCE(MAX(item_number), 0) + 1
            FROM order_item
            WHERE order_id = $1
            "
        )
            .bind(order_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        let unit_price: rust_decimal::Decimal = sqlx::query_scalar(
            "SELECT price FROM product WHERE product_id = $1"
        )
            .bind(product_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query(
            "
            INSERT INTO order_item (order_id, item_number, product_id, quantity, unit_price)
            VALUES ($1, $2, $3, $4, $5)
            "
        )
            .bind(order_id)
            .bind(next_item_number)
            .bind(product_id)
            .bind(quantity)
            .bind(unit_price)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_order_items(
    token: String,
    order_id: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<Vec<OrderItem>, String> {
    {
        let lock = sessions.0.lock().map_err(|_| "Системска грешка".to_string())?;
        if !lock.contains_key(&token) {
            return Err("Невалидна сесија.".to_string());
        }
    }

    let items = sqlx::query_as::<_, OrderItem>(
        "
        SELECT
            oi.order_id,
            oi.item_number,
            oi.product_id,
            p.name as product_name,
            oi.quantity,
            oi.unit_price
        FROM order_item oi
        JOIN product p ON oi.product_id = p.product_id
        WHERE oi.order_id = $1
        ORDER BY oi.item_number
        "
    )
        .bind(order_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(items)
}
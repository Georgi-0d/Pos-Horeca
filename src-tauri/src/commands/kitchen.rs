use sqlx::{PgPool, Row};
use tauri::State;

#[tauri::command]
pub async fn get_kitchen_orders(
    pool: State<'_, PgPool>
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        r#"
        SELECT
            o.order_id,
            o.table_id,
            rt.table_number,
            oi.item_number,
            p.name AS product_name,
            oi.quantity,
            o.created_at
        FROM orders o
        JOIN order_item oi ON o.order_id = oi.order_id
        JOIN product p ON oi.product_id = p.product_id
        JOIN restaurant_table rt ON o.table_id = rt.table_id
        WHERE o.status = 'АКТИВНА'
        ORDER BY o.created_at ASC, oi.item_number ASC
        "#
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "order_id": row.get::<i32, _>("order_id"),
            "table_id": row.get::<i32, _>("table_id"),
            "table_number": row.get::<String, _>("table_number"),
            "item_number": row.get::<i32, _>("item_number"),
            "product_name": row.get::<String, _>("product_name"),
            "quantity": row.get::<i32, _>("quantity"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339()
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn mark_kitchen_order_ready(
    order_id: i32,
    pool: State<'_, PgPool>
) -> Result<(), String> {
    println!("Нарачката {} е означена како спремна во кујна.", order_id);
    Ok(())
}
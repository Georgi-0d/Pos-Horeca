use sqlx::{PgPool, Row};
use tauri::State;

use crate::commands::auth::{require_manager_session, ActiveSessions};

pub async fn update_stock(
    order_id: i32,
    conn: &mut sqlx::PgConnection
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT product_id, quantity FROM order_item WHERE order_id = $1"
    )
        .bind(order_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let p_id: i32 = row.get("product_id");
        let order_qty: i32 = row.get("quantity");

        let recipe_row = sqlx::query(
            "SELECT recipe_id FROM recipe WHERE product_id = $1"
        )
            .bind(p_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(r) = recipe_row {
            let r_id: i32 = r.get("recipe_id");

            let ings = sqlx::query(
                "SELECT ingredient_id, quantity_needed FROM recipe_item WHERE recipe_id = $1"
            )
                .bind(r_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;

            for ing in ings {
                let ing_id: i32 = ing.get("ingredient_id");
                let qty_needed: rust_decimal::Decimal = ing.get("quantity_needed");

                sqlx::query(
                    "UPDATE ingredient SET current_stock = current_stock - ($1 * $2) WHERE ingredient_id = $3"
                )
                    .bind(qty_needed)
                    .bind(order_qty as f64)
                    .bind(ing_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        } else {
            sqlx::query(
                "INSERT INTO inventory (product_id, quantity_change, operation_type) VALUES ($1, -$2, 'ПРОДАЖБА')"
            )
                .bind(p_id)
                .bind(order_qty)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_inventory(
    pool: State<'_, PgPool>
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "
        SELECT
            p.product_id,
            p.name,
            COALESCE(SUM(i.quantity_change), 0) as quantity,
            p.min_stock
        FROM product p
        LEFT JOIN inventory i ON p.product_id = i.product_id
        WHERE p.active = true
        GROUP BY p.product_id, p.name, p.min_stock
        ORDER BY p.name
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "product_id": row.get::<i32, _>("product_id"),
            "name": row.get::<String, _>("name"),
            "quantity": row.get::<i64, _>("quantity"),
            "min_stock": row.get::<i32, _>("min_stock")
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_low_stock(
    pool: State<'_, PgPool>
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "
        SELECT
            p.product_id,
            p.name,
            COALESCE(SUM(i.quantity_change), 0) as quantity,
            p.min_stock
        FROM product p
        LEFT JOIN inventory i ON p.product_id = i.product_id
        WHERE p.active = true
        GROUP BY p.product_id, p.name, p.min_stock
        HAVING COALESCE(SUM(i.quantity_change), 0) <= p.min_stock
        ORDER BY p.name
        "
    )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(serde_json::json!({
            "product_id": row.get::<i32, _>("product_id"),
            "name": row.get::<String, _>("name"),
            "quantity": row.get::<i64, _>("quantity"),
            "min_stock": row.get::<i32, _>("min_stock")
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn restock_product(
    token: String,
    product_id: i32,
    quantity: i32,
    pool: State<'_, PgPool>,
    sessions: State<'_, ActiveSessions>,
) -> Result<(), String> {
    require_manager_session(&token, sessions.inner()).await?;

    if quantity <= 0 {
        return Err("Количината мора да биде поголема од 0".to_string());
    }

    sqlx::query(
        "
    INSERT INTO inventory
    (
        product_id,
        quantity_change,
        operation_type
    )
    VALUES
    ($1, $2, 'НАДОПОЛНУВАЊЕ_ЗАЛИХИ')
    "
    )
        .bind(product_id)
        .bind(quantity)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
use sqlx::PgPool;
use tauri::State;
use rust_decimal::Decimal;

use crate::commands::inventory::update_stock;
use crate::commands::table::{set_table_status, STATUS_FREE, STATUS_OCCUPIED};

#[tauri::command]
pub async fn create_order(
    user_id: i32,
    table_id: i32,
    pool: State<'_, PgPool>
) -> Result<i32, String> {

    let mut tx = pool.begin()
        .await
        .map_err(|e| e.to_string())?;

    let order_id: i32 = sqlx::query_scalar(
        "
        INSERT INTO orders
        (
            user_id,
            table_id,
            status
        )
        VALUES
        (
            $1,
            $2,
            'АКТИВНА'
        )
        RETURNING order_id
        "
    )
        .bind(user_id)
        .bind(table_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    set_table_status(table_id, STATUS_OCCUPIED, &mut *tx)
        .await?;

    tx.commit()
        .await
        .map_err(|e| e.to_string())?;

    Ok(order_id)
}

#[tauri::command]
pub async fn get_active_order(
    table_id: i32,
    pool: State<'_, PgPool>
) -> Result<Option<i32>, String> {

    let order_id = sqlx::query_scalar::<_, i32>(
        "
        SELECT order_id
        FROM orders
        WHERE table_id = $1
        AND status = 'АКТИВНА'
        LIMIT 1
        "
    )
        .bind(table_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(order_id)
}

#[tauri::command]
pub async fn get_order_total(
    order_id: i32,
    pool: State<'_, PgPool>
) -> Result<Decimal, String> {

    let total = sqlx::query_scalar(
        "
        SELECT COALESCE(SUM(quantity * unit_price), 0)
        FROM order_item
        WHERE order_id = $1
        "
    )
        .bind(order_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(total)
}

#[tauri::command]
pub async fn pay_order(
    order_id: i32,
    method: String,
    pool: State<'_, PgPool>
) -> Result<String, String> {

    let mut tx = pool.begin()
        .await
        .map_err(|e| e.to_string())?;

    let existing: Option<i32> = sqlx::query_scalar(
        "
        SELECT payment_id
        FROM payment
        WHERE order_id = $1
        "
    )
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Err("Нарачката е веќе платена".to_string());
    }

    let table_id: i32 = sqlx::query_scalar(
        "
        SELECT table_id
        FROM orders
        WHERE order_id = $1
        "
    )
        .bind(order_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let total: Decimal = sqlx::query_scalar(
        "
        SELECT COALESCE(SUM(quantity * unit_price), 0)
        FROM order_item
        WHERE order_id = $1
        "
    )
        .bind(order_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    if total == Decimal::ZERO {
        return Err("Нарачката е празна".to_string());
    }

    let payment_id: i32 = sqlx::query_scalar(
        "
        INSERT INTO payment
        (
            order_id,
            amount,
            method
        )
        VALUES
        (
            $1,
            $2,
            $3
        )
        RETURNING payment_id
        "
    )
        .bind(order_id)
        .bind(total)
        .bind(method)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "
        UPDATE orders
        SET status = 'ПЛАТЕНА'
        WHERE order_id = $1
        "
    )
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    update_stock(order_id, &mut tx)
        .await?;

    set_table_status(table_id, STATUS_FREE, &mut *tx)
        .await?;

    let invoice_number = format!("INV-{}", order_id);

    sqlx::query(
        "
        INSERT INTO invoice
        (
            payment_id,
            invoice_number
        )
        VALUES
        (
            $1,
            $2
        )
        "
    )
        .bind(payment_id)
        .bind(&invoice_number)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit()
        .await
        .map_err(|e| e.to_string())?;

    Ok(invoice_number)
}

#[tauri::command]
pub async fn update_order_quantity(
    order_id: i32,
    product_id: i32,
    quantity: i32,
    pool: State<'_, PgPool>
) -> Result<(), String> {

    if quantity <= 0 {
        return Err("Количината мора да биде поголема од 0".to_string());
    }

    sqlx::query(
        "
        UPDATE order_item
        SET quantity = $1
        WHERE order_id = $2
        AND product_id = $3
        "
    )
        .bind(quantity)
        .bind(order_id)
        .bind(product_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_order_item(
    order_id: i32,
    product_id: i32,
    pool: State<'_, PgPool>
) -> Result<(), String> {

    sqlx::query(
        "
        DELETE FROM order_item
        WHERE order_id = $1
        AND product_id = $2
        "
    )
        .bind(order_id)
        .bind(product_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
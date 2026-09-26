use serde::Serialize;
use sqlx::FromRow;
use rust_decimal::Decimal;


#[derive(Serialize, FromRow)]
pub struct OrderItem {
    pub order_id: i32,
    pub item_number: i32,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
}
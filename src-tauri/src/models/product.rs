use serde::Serialize;
use sqlx::FromRow;
use rust_decimal::Decimal;


#[derive(Serialize, FromRow)]
pub struct Product {
    pub product_id: i32,
    pub category_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,
}
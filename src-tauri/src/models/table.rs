use serde::Serialize;
use sqlx::FromRow;


#[derive(Serialize, FromRow)]
pub struct RestaurantTable {
    pub table_id: i32,
    pub table_number: String,
    pub capacity: i32,
    pub status: String,
}
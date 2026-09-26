use serde::Serialize;
use sqlx::FromRow;


#[derive(Serialize, FromRow)]
pub struct Category {
    pub category_id: i32,
    pub name: String,
    pub description: Option<String>,
}
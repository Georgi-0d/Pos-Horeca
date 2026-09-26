use serde::Serialize;
use sqlx::FromRow;
use rust_decimal::Decimal;

#[derive(Serialize, FromRow)]
pub struct Ingredient {
    pub ingredient_id: i32,
    pub name: String,
    pub unit: String,
    pub current_stock: Decimal,
}

#[derive(Serialize, FromRow)]
pub struct RecipeItem {
    pub ingredient_id: i32,
    pub quantity_needed: Decimal,
}
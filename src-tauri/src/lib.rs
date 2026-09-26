mod db;
mod commands;
mod models;
use db::create_pool;
use sqlx::PgPool;
use tauri::Manager;
use commands::auth::ActiveSessions;
use std::collections::HashMap;
use std::sync::Mutex;
use commands::auth::login;

use commands::product::{
    get_products,
    get_products_by_category
};

use commands::category::{
    get_categories,
    create_category,
    update_category,
    delete_category
};

use commands::table::{
    get_tables,
    reserve_table,
    cancel_reservation
};

use commands::order::{
    create_order,
    get_order_total,
    get_active_order,
    pay_order,
    update_order_quantity,
    delete_order_item
};

use commands::order_item::{
    add_order_item,
    get_order_items
};

use commands::payment::create_payment;

use commands::invoice::{
    create_invoice,
    get_invoice
};

use commands::inventory::{
    get_inventory,
    get_low_stock,
    restock_product
};

use commands::report::{
    today_sales,
    today_orders,
    best_product,
    z_report,
    employee_performance,
    search_orders
};

use commands::admin::{
    add_product,
    update_product,
    delete_product,
    update_product_price
};

use commands::employees::{
    get_employees,
    get_roles,
    add_employee,
    update_employee_role,
    reset_employee_pin,
    toggle_employee,
    delete_employee
};
use commands::window::{
    minimize_window,
    maximize_window,
    unmaximize_window,
    is_maximized,
    close_window
};
use commands::kitchen::{get_kitchen_orders, mark_kitchen_order_ready};
use commands::settings::{get_settings, update_settings};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(
            tauri::generate_handler![
                login,

                get_products,
                get_products_by_category,
                get_categories,
                create_category,
                update_category,
                delete_category,
                get_settings,
                update_settings,
                get_tables,
                reserve_table,
                cancel_reservation,
                delete_employee,
                create_order,
                get_order_total,
                get_active_order,
                pay_order,

                add_order_item,
                get_order_items,
                update_order_quantity,
                delete_order_item,

                create_payment,

                create_invoice,
                get_invoice,

                get_inventory,
                get_low_stock,
                restock_product,
                get_kitchen_orders,
                mark_kitchen_order_ready,
                today_sales,
                today_orders,
                best_product,
                z_report,
                employee_performance,
                search_orders,

                add_product,
                update_product,
                delete_product,
                update_product_price,

                get_employees,
                get_roles,
                add_employee,
                update_employee_role,
                reset_employee_pin,
                toggle_employee,
                minimize_window,
                maximize_window,
                unmaximize_window,
                is_maximized,
                close_window,

            ]
        )
        .setup(|app: &mut tauri::App| {
            let pool: PgPool = tauri::async_runtime::block_on(async {
                create_pool().await
            });
            app.manage(ActiveSessions(Mutex::new(HashMap::new())));
            app.manage(pool);

            println!("Database connected!");

            if std::env::args().any(|a| a == "--kitchen-window") {
                tauri::WebviewWindowBuilder::new(
                    app,
                    "kitchen",
                    tauri::WebviewUrl::App("index.html?mode=kitchen".into()),
                )
                    .title("Кујна")
                    .fullscreen(true)
                    .decorations(false)
                    .build()?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri");
}
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_pool() -> PgPool {
    let database_url =
        "postgres://db_202526z_va_prj_poshoreca_owner:4040c5ddf69@localhost:9999/db_202526z_va_prj_poshoreca";

    PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO project, public")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
}
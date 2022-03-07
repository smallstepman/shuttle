#[macro_use]
extern crate rocket;
use async_trait::async_trait;
use unveil_service::{declare_service, Deployment, Factory, Service};

use sqlx;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Default)]
struct App;

#[async_trait]
impl Service for App {
    async fn deploy(&self, factory: &mut dyn Factory) -> Deployment {
        /*let pool = factory.get_postgres_connection_pool().await.unwrap();

        sqlx::query("CREATE TABLE my_table (name STRING, favourite_number INTEGER) IN NOT EXISTS")
            .execute(&pool)
            .await
            .unwrap();*/

        rocket::build().mount("/hello", routes![index]).into()
    }
}

declare_service!(App, App::default);

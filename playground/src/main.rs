use std::time::Duration;

use rocket::{get, routes, tokio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(10)
        .build()
        .unwrap();
    rt.block_on(async {
        let rocket = rocket::build()
            .mount("/", routes![hello])
            .ignite()
            .await
            .unwrap();
        let s = rocket.shutdown();
        tokio::runtime::Handle::current().spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            s.notify();
        });
        let x = tokio::runtime::Handle::current().spawn(rocket.launch());

        let c = rocket::Config {
            port: 11111,
            ..Default::default()
        };
        let rocket = rocket::custom(c)
            .mount("/", routes![hello])
            .ignite()
            .await
            .unwrap();
        let q = tokio::runtime::Handle::current().spawn(rocket.launch());

        x.await.unwrap().unwrap();
        q.await.unwrap().unwrap()
    });
    std::thread::sleep(Duration::from_secs(3));
    Ok(())
}

#[get("/world")]
fn hello() -> &'static str {
    let c = rocket::Config {
        port: 22222,
        ..Default::default()
    };

    tokio::runtime::Handle::current().spawn(rocket::custom(c).launch());
    ""
}

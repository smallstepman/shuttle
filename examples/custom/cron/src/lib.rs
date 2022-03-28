use std::future::Future;

use chrono::Utc;
use saffron::Cron;
use shuttle_service::{
    IntoService,
    Service
};
use tokio::runtime::Runtime;
use tokio::time::sleep;

#[macro_use]
extern crate shuttle_service;

struct CronArgs<F>
where
    F: Future
{
    expression: String,
    job: fn() -> F
}

fn init() -> CronArgs<impl Future> {
    CronArgs {
        expression: "* * * * *".to_string(),
        job
    }
}

async fn job() {
    println!("working from my async");
}

impl<F> IntoService for CronArgs<F>
where
    F: Future + Send + Sync
{
    type Service = CronService<F>;

    fn into_service(self) -> Self::Service {
        CronService {
            cron: self
                .expression
                .parse()
                .expect("failed to parse cron expression"),
            job: self.job,
            runtime: Runtime::new().unwrap()
        }
    }
}

struct CronService<F>
where
    F: Future
{
    cron: Cron,
    job: fn() -> F,
    runtime: Runtime
}

impl<F> CronService<F>
where
    F: Future
{
    async fn start(&self) {
        for time in self.cron.clone().iter_after(Utc::now()).take(5) {
            let duration = time.signed_duration_since(Utc::now());
            let duration = duration.to_std().expect("time has already passed");

            sleep(duration).await;

            (self.job)().await;
        }
    }
}

impl<F> Service for CronService<F>
where
    F: Future + Send + Sync
{
    fn bind(&mut self, _addr: std::net::SocketAddr) -> Result<(), shuttle_service::error::Error> {
        self.runtime.block_on(self.start());

        println!("All done");

        Ok(())
    }
}

declare_service!(CronArgs<_>, init);

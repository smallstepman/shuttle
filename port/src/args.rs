use shuttle_common::Port;
use std::net::IpAddr;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "shuttle")]
pub struct Args {
    #[structopt(long, about = "Override the default root path for shuttle")]
    pub(crate) path: Option<PathBuf>,
    #[structopt(
        long,
        about = "Override the default port for the api",
        default_value = "8001"
    )]
    pub(crate) bind_port: Port,
    #[structopt(
        long,
        about = "Override the default bind address",
        default_value = "127.0.0.1"
    )]
    pub(crate) bind_addr: IpAddr,
}

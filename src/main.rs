use crate::util::DynResult;
use args::Action;
use env_logger::Builder;
use std::env;
use std::fs;
use std::process::Child;

#[macro_use]
mod macros;
#[macro_use]
extern crate log;
mod args;
mod bencher;
mod updater;
mod util;

static mut SERVER_CHILD: Option<Child> = None;

fn main() {
    Builder::new()
        .parse_filters(&env::var("SKYREPORT_LOG").unwrap_or_else(|_| "info".to_owned()))
        .init();
    runtime();
}

#[tokio::main]
async fn runtime() {
    let what_to_do = Action::from_env();
    let ret = async {
        cmderr!("git", "config", "--global", "pull.rebase", "true");
        fs::create_dir_all("preset")?;
        fs::create_dir_all("results")?;
        fs::create_dir_all("reports")?;
        match what_to_do {
            Action::NewBench(bench) => bencher::new(bench.commit(), bench.pull()).await?,
            Action::UpdateNext => updater::update_next()?,
            Action::UpdateRelease(release) => updater::update_release(&release)?,
        }
        Ok(())
    };
    let errored;
    let ret: DynResult<()> = ret.await;
    if let Err(e) = ret {
        log::error!("The task failed with: {}", e);
        errored = true;
    } else {
        errored = false;
    }
    unsafe {
        // we know everything has terminated; so no threads are accessing this
        info!("Terminating server ...");
        if let Some(Err(e)) = SERVER_CHILD.take().map(|mut v| v.kill()) {
            log::error!("Failed to terminate the server: {}", e);
        } else {
            info!("Terminated server");
        }
    }
    if let Err(e) = util::clear_source_dir() {
        log::error!("Failed to remove files from cloned dir with: `{}`", e);
    }
    if errored {
        err!("skyreport operation failed");
    }
}

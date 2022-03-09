/*
 * Created on Thu Sep 14 2021
 *
 * This file is a part of Skytable's "Perf" tool
 * Skytable's performance tool is used to analyze the performance of the
 * Skytable database.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use crate::util::DynResult;
use args::Action;
use env_logger::Builder;
use std::env;
use std::fs;

#[macro_use]
mod macros;
#[macro_use]
extern crate log;
mod args;
mod bencher;
mod updater;
mod util;

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
        error!("The task failed with: {}", e);
        errored = true;
    } else {
        errored = false;
    }
    if errored {
        err!("skyreport operation failed");
    }
}

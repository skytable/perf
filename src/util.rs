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

use std::env;
use std::fs;
use std::io::Write;

pub type DynResult<T> = Result<T, Box<dyn std::error::Error>>;
const REPO: &str = "https://github.com/skytable/skytable.git";
const RELEASE_DIR: &str = "target/release";
const VAR_LATEST_COMMIT: &str = "LATEST_COMMIT";

pub fn get_latest_commit() -> DynResult<String> {
    Ok(env::var(VAR_LATEST_COMMIT).map(|v| v.to_string().replace('"', ""))?)
}

/// This will clone the skytable/skytable repo, switch to the directory
/// and checkout the branch, returning errors if any do occur
pub fn clone_and_checkout(branch: &str) -> DynResult<()> {
    info!("Cloning repo ...");
    hspawnerr!("git", "clone", REPO);
    info!("Switching to repo directory ...");
    env::set_current_dir("skytable")?;
    info!("Checking out branch `{}`", branch);
    hspawnerr!("git", "checkout", branch);
    // now set the latest commit
    info!("Setting `{}` to the latest SHA on HEAD", VAR_LATEST_COMMIT);
    let latest_commit = cmd!("git", "log", "-n", "1", "--pretty=format:\"%H\"").output()?;
    if !latest_commit.stderr.is_empty() {
        return rerr!("Failed to get the commit hash");
    }
    env::set_var(
        VAR_LATEST_COMMIT,
        String::from_utf8_lossy(&latest_commit.stdout)
            .to_string()
            .replace('"', ""),
    );
    info!("Done cloning. Returning control ...");
    Ok(())
}

/// This will prepare a release build for skytable and switch to the `target/release`
/// directory.
///
/// **Important note:** This function expects to be in the source directory
pub fn build() -> DynResult<()> {
    info!("Starting build ... (this may take a while)");
    hspawnerr!(
        "cargo",
        "build",
        "-p",
        "skyd",
        "-p",
        "sky-bench",
        "--release"
    );
    info!("Switching to the release directory ... ");
    env::set_current_dir(RELEASE_DIR)?;
    info!("Done building. Returning control ...");
    Ok(())
}

/// This will start the server as a child process (sharing same stdout/stderr)
///
/// **Important note:** This function expects to be in the `target/release` directory
pub fn start_server_in_background() -> DynResult<()> {
    info!("Starting server in background");
    let child = cmd!("./skyd", "--noart").spawn()?;
    info!("Sleeping to wait for server to start up");
    sleep!(10);
    // now set global process ID
    unsafe {
        // we know this is single threaded, so we're good
        crate::SERVER_CHILD = Some(child);
    }
    info!("Finished sleeping. Returning control ...");
    Ok(())
}

/// This will run the benchmark with the defaults for `skyreport` and return the stdout
///
/// **Important note:** This function expects to be in the `target/release` directory
pub fn run_benchmark_and_get_stdout() -> DynResult<String> {
    info!("Beginning benchmark ...");
    let output = cmd!("./sky-bench", "-c50", "-q1000000", "-s4", "--json").output()?;
    let stderr = &output.stderr;
    if !stderr.is_empty() {
        return rerr!(format!(
            "sky-bench failed with error: `{}`",
            String::from_utf8_lossy(stderr)
        ));
    }
    if !output.status.success() {
        // double-check; did it fail?
        return rerr!("`sky-bench` returned a non-zero exit code");
    }
    let json_out = String::from_utf8_lossy(&output.stdout)
        .to_string()
        .trim()
        .to_string();
    trace!("JSON output from sky-bench: `{}`", json_out);
    Ok(json_out)
}

pub fn create_and_write_to_file(fname: &str, body: &[u8]) -> DynResult<()> {
    trace!("Creating file: `{}`", fname);
    let mut file = fs::File::create(fname)?;
    trace!("Writing to file ...");
    file.write_all(body)?;
    Ok(())
}

pub fn clear_source_dir() -> DynResult<()> {
    info!("Removing the generated files ...");
    // remove the "${PWD}/skytable" directory
    fs::remove_dir_all("skytable")?;
    info!("Removed source files and finished getting bench output. Returning control ...");
    Ok(())
}

use crate::bencher::Report;
use crate::util;
use crate::DynResult;
use serde::{Deserialize, Serialize};
use std::env;
pub type SkyBenchReport = Vec<SkyBenchReportSection>;

#[derive(Debug, Serialize, Deserialize)]
pub struct SkyBenchReportSection {
    name: String,
    pub stat: f32,
}

pub const FILE_LATEST_RELEASE: &str = "./preset/release.json";
const BRANCH_LATEST: &str = "next";
pub const FILE_NEXT: &str = "./preset/next.json";

#[derive(Debug, Deserialize, Serialize)]
pub struct ReportItem {
    pub commit: String,
    pub report: Report,
}

impl ReportItem {
    pub const fn new(commit: String, report: Report) -> Self {
        Self { commit, report }
    }
}

/// Updates the release preset result to the provided release.
/// **Be warned! You should supply the latest release**
pub fn update_release(release: &str) -> DynResult<()> {
    info!(
        "Updating results for latests release (assuming `{}` is latest)",
        release
    );
    let results: Report = Report::from_stdout(self::raw_result(release)?)?;
    let result_update = ReportItem::new(release.to_owned(), results);
    let result_update_str = serde_json::to_string_pretty(&result_update)?;
    util::create_and_write_to_file(FILE_LATEST_RELEASE, result_update_str.as_bytes())?;
    commit!(format!("Update results for release {}", release));
    Ok(())
}

/// Updates the next preset result to the current `HEAD` on `skytable/skytable`
/// (`next`)
pub fn update_next() -> DynResult<()> {
    info!("Updating results for next ...",);
    let results: Report = Report::from_stdout(self::raw_result(BRANCH_LATEST)?)?;
    let result_update = ReportItem::new(util::get_latest_commit()?, results);
    let result_update_str = serde_json::to_string_pretty(&result_update)?;
    util::create_and_write_to_file(FILE_NEXT, result_update_str.as_bytes())?;
    commit!("Update results for next");
    Ok(())
}

/// This returns the raw output from `sky-bench` for the provided `branch`
pub fn raw_result(branch: &str) -> DynResult<String> {
    // get the current directory
    let curdir = cd!();
    // first clone the release; this will switch to /skytable
    util::clone_and_checkout(branch)?;
    // build. this will switch to target/release
    util::build()?;
    // start the server
    util::start_server_in_background()?;
    // run the bench
    let benchret = util::run_benchmark_and_get_stdout()?;
    info!("Switching to the base directory ...");
    // now switch to the original dir
    cd!(curdir);
    Ok(benchret)
}

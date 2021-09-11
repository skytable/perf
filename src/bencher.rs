use crate::updater;
use crate::updater::ReportItem;
use crate::updater::SkyBenchReport;
use crate::updater::FILE_LATEST_RELEASE;
use crate::updater::FILE_NEXT;
use crate::util;
use crate::DynResult;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;

macro_rules! concat_string {
    ($($e:expr),*) => {{
        let mut s = String::new();
        $(s.push_str(&$e.to_string());)*
        s
    }};
}

const COMMIT_BASE_URL: &str = "https://github.com/skytable/skytable/commit";
const PR_BASE_URL: &str = "https://github.com/skytable/skytable/pull";
const FILE_URL: &str = "https://github.com/skytable/perf/blob/next/reports";

#[derive(Debug, Deserialize, Serialize, Clone)]
/// A report, which is a structure for JSON of the following form:
/// ```json
/// {
///     "get": 444555.12,
///     "set": 398276.52,
///     "update": 389244.75
/// }
/// ```
pub struct Report {
    get: f32,
    set: f32,
    update: f32,
}

impl Report {
    /// This parses the output from `sky-bench` which looks like:
    /// ```json
    /// [{"report":"NAME","stat":1234567.89}]
    /// ```
    ///
    /// It expects GET, SET and UPDATE to be in order
    pub fn from_stdout(stdout: impl AsRef<[u8]>) -> DynResult<Self> {
        let from_stdout: SkyBenchReport =
            serde_json::from_str(&String::from_utf8_lossy(stdout.as_ref()))?;
        Ok(Self {
            get: from_stdout[0].stat,
            set: from_stdout[1].stat,
            update: from_stdout[2].stat,
        })
    }
}

#[derive(Debug, Serialize)]
/// A raw report written to `results/*.json`. This is created when a bench operation
/// is run against a commit
pub struct RawReport {
    commit: String,
    pr: String,
    raw: Report,
    results: Vec<Comparison>,
}

#[derive(Debug, Serialize)]
/// A comparison for the [`RawReport`]
pub struct Comparison {
    against: String,
    result: Report,
}

impl Comparison {
    /// Create a new [`Comparison`] from the provided `against` and `result`
    pub const fn new(against: String, result: Report) -> Self {
        Self { against, result }
    }
}

/// Create a new bench for the provided commit and PR
pub async fn new(commit: &str, pr: u16) -> DynResult<()> {
    let crab = octocrab::Octocrab::builder()
        .personal_token(env::var("GH_TOKEN")?)
        .build()?;
    info!("New bench for commit: `{}` in PR#{}", commit, pr);

    // just use the CLI data command; no need for fancy libs
    let date = cmd!("date", "+%d%m%Y-%H%M%S").output()?;
    if !date.stderr.is_empty() {
        return rerr!(format!(
            "`date` returned an error: `{}`",
            String::from_utf8_lossy(&date.stderr)
        ));
    }
    let datestr = String::from_utf8_lossy(&date.stdout).to_string();
    let datestr = datestr.trim();

    // we have two files: the raw report; and the markdown report
    let json_filename = format!("./results/result-{}.json", datestr);
    let report_filename = format!("./reports/result-{}.md", datestr);
    // set var for the workflow to add the comment
    let url_to_report = format!(
        "{base_url}/result-{date}.md",
        base_url = FILE_URL,
        date = datestr
    );

    // get the base output from sky-bench
    let result = updater::raw_result(commit)?;
    let current_report: Report = Report::from_stdout(result.as_bytes())?;
    let result_bytes = serde_json::to_string_pretty(&result)?;

    // write the raw report
    let mut raw_report_file = fs::File::create(&json_filename)?;
    raw_report_file.write_all(result_bytes.as_bytes())?;

    /*
     now decode the files because we need to render the markdown file.
     (1) Decode `preset/next.json` to compare against `next`
     (2) Decode `preset/release.json` to compare against the last release
     (3) Decode `self::result` to get the current results
    */
    let last_release_report = fs::read_to_string(FILE_LATEST_RELEASE)?;
    let last_release_report: ReportItem = serde_json::from_str(&last_release_report)?;
    let last_head_report = fs::read_to_string(FILE_NEXT)?;
    let last_head_report: ReportItem = serde_json::from_str(&last_head_report)?;

    // compare against next
    let next_delta_get = delta(current_report.get, last_head_report.report.get);
    let next_delta_set = delta(current_report.set, last_head_report.report.set);
    let next_delta_update = delta(current_report.update, last_head_report.report.update);
    let delta_next = Report {
        get: next_delta_get,
        set: next_delta_set,
        update: next_delta_update,
    };

    // compare against last release
    let last_release_delta_get = delta(current_report.get, last_release_report.report.get);
    let last_release_delta_set = delta(current_report.set, last_release_report.report.set);
    let last_release_delta_update = delta(current_report.update, last_release_report.report.update);
    let delta_last_release = Report {
        get: last_release_delta_get,
        set: last_release_delta_set,
        update: last_release_delta_update,
    };

    // prepare the raw report
    let raw_report = RawReport {
        commit: concat_string!(commit),
        pr: concat_string!(pr),
        results: vec![
            Comparison::new(last_head_report.commit.clone(), delta_next.clone()),
            Comparison::new(
                last_release_report.commit.clone(),
                delta_last_release.clone(),
            ),
        ],
        raw: current_report.clone(),
    };

    // write the raw report
    util::create_and_write_to_file(
        &json_filename,
        serde_json::to_string_pretty(&raw_report)?.as_bytes(),
    )?;

    let list_vs_next_title = format!("v/s next ({commit})", commit = &last_head_report.commit);
    let list_vs_release_title = format!(
        "v/s release ({release})",
        release = last_release_report.commit
    );
    // write the markdown file
    info!("Writing report ...");
    let mut md = fs::File::create(report_filename)?;
    md.write_all("# Skyreport\n".as_bytes())?;
    md.write_all("## Meta\n".as_bytes())?;
    let commit_str = format!(
        "Commit: [{commit}]({base_url}/{commit})",
        commit = util::get_latest_commit()?,
        base_url = COMMIT_BASE_URL
    );
    let pr_str = format!(
        "Pull request: [{pr}]({pr_base_url}/{pr})",
        pr = pr,
        pr_base_url = PR_BASE_URL
    );
    md.write_all(render_list(vec![commit_str, pr_str]).as_bytes())?;

    // prepare the next list
    let next_get_result_string = format!("**GET**: {}", delta_next.get);
    let next_set_result_string = format!("**SET**: {}", delta_next.set);
    let next_update_result_string = format!("**UPDATE**: {}", delta_next.update);
    let next_list = render_nested_list(
        list_vs_next_title,
        vec![
            next_get_result_string,
            next_set_result_string,
            next_update_result_string,
        ],
    );

    // prepare the v/s tag list
    let release_get_result_string = format!("**GET**: {}", delta_last_release.get);
    let release_set_result_string = format!("**SET**: {}", delta_last_release.set);
    let release_update_result_string = format!("**UPDATE**: {}", delta_last_release.update);
    let release_list = render_nested_list(
        list_vs_release_title,
        vec![
            release_get_result_string,
            release_set_result_string,
            release_update_result_string,
        ],
    );

    // write summary
    md.write_all("## Summary\n".as_bytes())?;
    md.write_all(next_list.as_bytes())?;
    md.write_all(release_list.as_bytes())?;

    // write raw result
    md.write_all("## Raw Result\n".as_bytes())?;
    let current_get_result_string = format!("**GET**: {}", current_report.get);
    let current_set_result_string = format!("**SET**: {}", current_report.set);
    let current_update_result_string = format!("**UPDATE**: {}", current_report.update);
    let current_list = render_list(vec![
        current_get_result_string,
        current_set_result_string,
        current_update_result_string,
    ]);
    md.write_all(current_list.as_bytes())?;
    info!("Finished writing report!");
    commit!(
        format!("Added result for skytable/skytable#{} [skip ci]", pr),
        format!("Triggered by {trigger_commit}", trigger_commit = commit)
    );
    info!("Adding comment");
    crab.issues("skytable", "skytable")
        .create_comment(
            pr.into(),
            format!(
                "The benchmark has completed. Review [the benchmark here]({url})",
                url = url_to_report
            ),
        )
        .await?;
    info!("Added comment");
    Ok(())
}

pub fn delta(now: f32, prev: f32) -> f32 {
    ((now - prev) / prev) * 100_f32
}

/// Renders a markdown list from the provided string vector. For example,
/// `vec!["a", "b", "c"]` is turned into:
/// ```md
/// - a
/// - b
/// - c
/// ```
pub fn render_list(input: Vec<String>) -> String {
    let mut st = String::new();
    input.into_iter().for_each(|item| {
        st.push_str("- ");
        st.push_str(&item);
        st.push('\n');
    });
    st
}

/// Renders a nested markdown lists' single entry. For example,
/// for `title`: "Favorites", and `input`: `vec!["apples", "bananas", "carrots"]`,
/// the following markdown is produced:
/// ```md
/// - Favorites
///   - apples
///   - bananas
///   - carrots
/// ```
fn render_nested_list(title: String, input: Vec<String>) -> String {
    let mut st = "- ".to_owned();
    st.push_str(&title);
    st.push('\n');
    input.into_iter().for_each(|item| {
        st.push_str("  - ");
        st.push_str(&item);
        st.push('\n');
    });
    st
}

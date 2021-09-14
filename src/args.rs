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

macro_rules! nxiter {
    ($iter:expr, $emsg:expr) => {
        $iter.next().unwrap_or_else(|| err!($emsg))
    };
}

const ARG_UPDATE: &str = "update";
const ARG_BENCH: &str = "bench";
const ARG_UPDATE_NEXT: &str = "next";
const ARG_UPDATE_RELEASE: &str = "release";

#[derive(Debug, PartialEq)]
pub struct NewBench {
    commit: String,
    pull: u16,
}

impl NewBench {
    pub const fn new(commit: String, pull: u16) -> Self {
        Self { commit, pull }
    }
    pub const fn pull(&self) -> u16 {
        self.pull
    }
    pub const fn commit(&self) -> &String {
        &self.commit
    }
}

#[derive(Debug, PartialEq)]
pub enum Action {
    UpdateNext,
    UpdateRelease(String),
    NewBench(NewBench),
}

impl Action {
    pub fn from_args(args: Vec<String>) -> Self {
        let mut iter = args.into_iter().skip(1);
        match iter.next() {
            Some(arg) => {
                let action = match arg.as_ref() {
                    ARG_UPDATE => {
                        // we need to update
                        let update_what = nxiter!(iter, "Unknown update action!");
                        let update = match update_what.as_ref() {
                            ARG_UPDATE_NEXT => Action::UpdateNext,
                            ARG_UPDATE_RELEASE => {
                                // need a release
                                let tag = nxiter!(iter, "Please provide a tag!");
                                Action::UpdateRelease(tag)
                            }
                            _ => err!("Unknown update action"),
                        };
                        update
                    }
                    ARG_BENCH => {
                        // we need to bench
                        let bench_what_commit = nxiter!(iter, "Please provide the commit to bench");
                        let bench_which_pr: u16 =
                            match nxiter!(iter, "Please provider the PR ID").parse() {
                                Ok(pr) => pr,
                                _ => err!("Bad value for PR ID"),
                            };
                        Action::NewBench(NewBench::new(bench_what_commit, bench_which_pr))
                    }
                    _ => err!("Unknown action"),
                };
                if iter.next().is_some() {
                    // hmm, more args? nope, we didn't expect that
                    err!("Unexpected arguments");
                }
                action
            }
            None => err!("Please provide a second argument with the action to perform!"),
        }
    }
    pub fn from_env() -> Self {
        Self::from_args(env::args().collect())
    }
}

#[cfg(test)]
macro_rules! tvec {
    ($($v:expr),*) => {
        vec![$($v.into()),*]
    };
}

#[test]
fn test_update_release() {
    let args = tvec!["skyreport", "update", "release", "v0.7.0"];
    assert_eq!(
        Action::from_args(args),
        Action::UpdateRelease("v0.7.0".to_owned())
    );
}

#[test]
fn test_update_next() {
    let args = tvec!["skyreport", "update", "next"];
    assert_eq!(Action::from_args(args), Action::UpdateNext);
}

#[test]
fn test_new_bench() {
    let args = tvec!["skyreport", "bench", "12345abcde", "234"];
    assert_eq!(
        Action::from_args(args),
        Action::NewBench(NewBench::new("12345abcde".to_owned(), 234))
    )
}

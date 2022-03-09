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

macro_rules! err {
    ($e:expr) => {{
        ::log::error!("{}", $e);
        std::process::exit(0x01);
    }};
}

macro_rules! cmd {
    ($program:expr, $($arg:expr),*) => {{
        let mut cmd = std::process::Command::new($program);
        $(cmd.arg($arg);)*
        cmd
    }};
}

macro_rules! cmderr {
    ($program:expr, $($arg:expr),*) => {
        let mut cmd = cmd!($program, $($arg),*);
        let output = cmd.output()?;
        if !output.status.success() {
            ::log::error!("Child failed with: {}", String::from_utf8_lossy(&output.stderr));
            err!("Fatal error in child process");
        }
    };
}

macro_rules! hspawnerr {
    ($program:expr, $($arg:expr),*) => {
        let mut cmd = cmd!($program, $($arg),*);
        let mut child = cmd.spawn()?;
        let exit_code = child.wait()?;
        if !exit_code.success() {
            return Err("The child process failed".into());
        }
    };
}

macro_rules! sleep {
    ($dursec:literal) => {
        std::thread::sleep(std::time::Duration::from_secs($dursec))
    };
}

macro_rules! rerr {
    ($e:expr) => {
        Err($e.into())
    };
}

macro_rules! cd {
    () => {
        std::env::current_dir()?
    };
    ($chdir:expr) => {
        std::env::set_current_dir($chdir)?
    };
}

macro_rules! commit {
    ($($msg:expr),*) => {
        let token = env::var("GH_TOKEN")?;
        trace!("Adding files ...");
        cmderr!("git", "add", ".");
        trace!("Committing files ...");
        cmderr!(
            "git",
            "commit",
            $(
                "-m",
                $msg
            ),*
        );
        trace!("Pulling latest ... ");
        cmderr!("git", "pull", "--rebase");
        trace!("Publishing results ...");
        cmderr!(
            "git",
            "push",
            format!(
                "https://glydr:{token}@github.com/{org}/{repo}",
                org = util::ORG_NAME,
                repo = util::REPO_PERF,
                token = token
            ),
            "--all"
        );
    };
}

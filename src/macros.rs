macro_rules! err {
    ($e:expr) => {{
        log::error!("{}", $e);
        std::process::exit(0x10);
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
            log::error!("Child failed with: {}", String::from_utf8_lossy(&output.stderr));
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
        Err($e.into());
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

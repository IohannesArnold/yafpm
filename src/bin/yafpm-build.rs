// SPDX-License-Identifier: GPL-2.0-or-later
// 
// Copyright (C) 2021 John Arnold
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::io;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::ffi::OsString;
use std::error::Error;
use std::os::unix::ffi::OsStrExt;
use yafpm::{BuildCxt, BuildError};

#[cfg(feature = "serde_json")]
use serde_json;
#[cfg(feature = "toml")]
use toml;

const USAGE: &str =
"Usage: yafpm-build [-hv] [-P|--package-dir=<pkg_dir>] [--toml|--json] <file>";
const PACKAGE_DIR: &str = "/yafpm";

enum FileType {
    JSON,
    TOML,
    Unknown
}

fn parse_args() -> Result<(
    FileType,
    Option<OsString>,
    Option<OsString>,
    u8
), lexopt::Error> {
    use lexopt::prelude::*;
    let mut ft = FileType::Unknown;
    let mut file_str = None;
    let mut pkg_dir = None;
    let mut verbosity: u8 = 0;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("json") => { ft = FileType::JSON; }
            Long("toml") => { ft = FileType::TOML; }
            Short('v') => { verbosity += 1;}
            Short('P') | Long("package-dir") => {
                pkg_dir = Some(parser.value()?);
            }
            Short('h') | Long("help") => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
            Value(val) => {
                file_str = Some(val);
            }
            _ => return Err(arg.unexpected()),
        }
    }
    Ok((ft, file_str, pkg_dir, verbosity))
}

fn read_path_to_string<P: AsRef<Path>>(file_name: P) -> Result<String, io::Error> {
    let fd = File::open(&file_name)?;
    let mut buf_reader = BufReader::new(fd);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    Ok(contents)
}

fn get_config_format(ft: FileType, file_path: &Path) -> FileType {
    match ft {
        FileType::JSON => FileType::JSON,
        FileType::TOML => FileType::TOML,
        FileType::Unknown => {
            match file_path.extension().map(|s| s.as_bytes()) {
                Some(b"json") => FileType::JSON,
                Some(b"toml") => FileType::TOML,
                _ => FileType::Unknown
            }
        }
    }
}

// So our config file can have things like "url = './build.sh'"
fn set_serde_base_url(file_path: &Path) -> Result<(), io::Error> {
    use url::Url;
    use yafpm::SERDE_BASE_URL;

    let absolute_path = match file_path.is_absolute() {
        true => file_path.canonicalize()?,
        false => {
            let mut pwd = std::env::current_dir()?;
            pwd.push(file_path);
            pwd.canonicalize()?
        }
    };

    // Unwraping this should be fine because we already
    // canonicalize absolute_path
    let url = Url::from_file_path(absolute_path).unwrap();

    unsafe {
        SERDE_BASE_URL = Some(url);
    }

    Ok(())

}

fn main() {
    let (ft, file_str, pkg_dir, _verbosity) = match parse_args() {
        Ok((ft, Some(file_str), pkg_dir, verbosity)) =>
            (ft, file_str, pkg_dir, verbosity),
        Ok((_, None, _, _)) => {
            eprintln!("Missing command line argument: <file>");
            eprintln!("{}", USAGE);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Command line parsing error: {}", e);
            eprintln!("{}", USAGE);
            std::process::exit(1);
        }
    };
    let file_path = Path::new(&file_str);
    if let Err(e) = set_serde_base_url(&file_path) {
        eprintln!("Unable to determine canonical directory of {}", file_path.display());
        eprintln!("Encountered error: {}", e);
        std::process::exit(1);
    }
    let contents = match read_path_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path.display(), e);
            std::process::exit(1);
        }
    };
    let build_context: BuildCxt = match get_config_format(ft, file_path) {
        #[cfg(feature = "serde_json")]
        FileType::JSON => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Error parsing JSON: {}", e);
            std::process::exit(1);
        }),
        #[cfg(feature = "toml")]
        FileType::TOML => toml::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Error parsing TOML: {}", e);
            std::process::exit(1);
        }),
        _ => {
            eprint!("Unable to recognize config encoding. ");
            eprintln!("Try specifying --toml or --json");
            std::process::exit(1);
        }
    };

    let pkg_name = build_context.pkg_info.pkg_name;
    let pkg_dir = pkg_dir.unwrap_or(OsString::from(PACKAGE_DIR));

    if let Err(top_err) = build_context.exec_build(pkg_dir) {
        eprintln!("Error building {}:", pkg_name);
        let mut depth = 1;
        eprintln!("{:>5}. {}", depth, top_err);
        let mut source_err_opt = top_err.source();
        while let Some(err) = source_err_opt {
            depth += 1;
            // TODO For custom printing certain errors
            match err {
                e => eprintln!("{:>5}. {}", depth, e)
            };
            source_err_opt = err.source();
        }
        if let BuildError::HashError{err: _, teardown_err: Some(e2)} = top_err {
            eprintln!("");
            eprintln!("Furthermore, could not remove corrupted directory due to error:",
            );
            eprintln!("{:>5}. {}", 1, e2);
        }
        std::process::exit(1);
    }
    std::process::exit(0);

}

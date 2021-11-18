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

use std::fs;
use std::io;
use std::env;
use std::path::{Path, PathBuf};

pub fn create_context_dir(context_name: &str) -> Result<PathBuf, io::Error> {
    let mut context_dir = env::temp_dir();
    context_dir.push(context_name);
    fs::create_dir(&context_dir)?;
    Ok(context_dir)
}

pub fn create_outdir<P: AsRef<Path>>(
    pkg_dir:P,
    pkg_ident: &str
) -> Result<PathBuf, io::Error> {
    let out_dir = pkg_dir.as_ref().join(pkg_ident);
    fs::create_dir(&out_dir)?;
    Ok(out_dir)
}

pub fn set_readonly_all<P: AsRef<Path>> (
    dir: P,
    ro_bool: bool
) -> Result<(), io::Error> {
    let mut dir_perms = fs::metadata(&dir)?.permissions();
    dir_perms.set_readonly(ro_bool);
    fs::set_permissions(&dir, dir_perms)?;
    if dir.as_ref().is_dir() {
        for entry in fs::read_dir(dir)? {
            set_readonly_all(entry?.path(), ro_bool)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_builddir() {
        let mut test_path = env::temp_dir();
        let val = create_builddir("pkgname").unwrap();
        test_path.push("pkgname-build");
        assert_eq!(test_path, val);
        assert!(test_path.exists());
        fs::remove_dir(test_path).unwrap();
    }

    #[test]
    fn test_create_outdir() {
        let mut test_path = env::temp_dir();
        create_outdir(&mut test_path.clone(), "ident").unwrap();
        test_path.push("ident");
        assert!(test_path.exists());
        fs::remove_dir(test_path).unwrap();
    }

    #[test]
    fn test_set_readonly_all() {
        let mut test_path = env::temp_dir();
        test_path.push("level1");
        fs::create_dir(&test_path).unwrap();
        test_path.push("level2");
        fs::create_dir(&test_path).unwrap();
        test_path.pop();
        set_readonly_all(&test_path, true).unwrap();
        let level1_perms = fs::metadata(&test_path).unwrap().permissions();
        assert!(level1_perms.readonly());
        test_path.push("level2");
        let level2_perms = fs::metadata(&test_path).unwrap().permissions();
        assert!(level2_perms.readonly());
        test_path.pop();
        set_readonly_all(&test_path, false).unwrap();
        fs::remove_dir_all(test_path).unwrap();
    }
}

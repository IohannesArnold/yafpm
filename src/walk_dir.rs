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
use std::path::Path;
use std::os::unix::ffi::OsStrExt;

pub fn calculate_directory_hash<P: AsRef<Path>, D: io::Write> (
    dir: P,
    hasher: &mut D
) -> Result<(), io::Error> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by(|x, y| x.path().cmp(&y.path()));
    for entry in entries {
        hasher.write_all(entry.file_name().as_bytes())?;
        if entry.file_type()?.is_file() {
            let mut fd = fs::File::open(entry.path())?;
            io::copy(&mut fd, hasher)?;
        } else if entry.file_type()?.is_symlink() {
            let target = fs::read_link(entry.path())?;
            hasher.write_all(target.as_os_str().as_bytes())?;
        } else if entry.file_type()?.is_dir() {
            calculate_directory_hash(entry.path(), hasher)?;
        }
    }
    Ok(())
}

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
use std::io::Write;
use std::path::{Path, PathBuf};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::geteuid;
use nix::mount::{mount,umount,MsFlags};

use crate::package::Package as PKG;

#[derive(Debug, thiserror::Error)]
pub enum NSError {
    #[error("Unable to create new namespace")]
    NewError(#[source] nix::Error),
    #[error("Unable to create new user map")]
    UMapError(#[source] io::Error),
    #[error("Error while creating {}", .0.display())]
    MkDirError(PathBuf, #[source]io::Error),
    #[error("Error while mounting {} to {}",
            .source_dir.display(),
            .target_dir.display())]
    BindMountError{
        source_dir: PathBuf,
        target_dir: PathBuf,
        #[source]
        err: nix::Error
    },
    #[error("Error while unmounting {}", .0.display())]
    BindUMountError(PathBuf, #[source] nix::Error)
}

fn get_uid_map() -> String {
    let euid = geteuid();
    format!("0 {} 1\n", euid)
}

pub fn setup_new_namespace() -> Result<(), NSError> {
    let uid_map = get_uid_map();
    let flags = CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWNET | CloneFlags::CLONE_NEWPID;
    unshare(flags).map_err(|e| NSError::NewError(e))?;
    let mut file = File::create("/proc/self/uid_map").map_err(
        |e| NSError::UMapError(e))?;
    file.write_all(uid_map.as_bytes()).map_err(
        |e| NSError::UMapError(e))?;
    Ok(())
}

pub fn mount_dep_dirs<'a, P: AsRef<Path>>(
    pkg_store_dir: P,
    build_dir: &Path,
    deps: impl IntoIterator<Item = &'a PKG<'a>>,
) -> Result<(), NSError> {
    let flags = MsFlags::MS_BIND;
    //let ro_flags = MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY;

    let mut bind_dir = build_dir.to_path_buf();
    let mut dep_dir = pkg_store_dir.as_ref().to_path_buf();
    for dep in deps {
        dep_dir.push(dep.pkg_ident());
        // This should be safe because of logic in build_cxt::exec_build
        bind_dir.push(dep_dir.strip_prefix("/").unwrap());
        std::fs::create_dir_all(&bind_dir).map_err(
            |e| NSError::MkDirError(bind_dir.clone(),e)
        )?;
        mount(Some(&dep_dir), &bind_dir, None::<&str>, flags, None::<&str>).map_err(
            |e| NSError::BindMountError{
                source_dir: dep_dir.clone(),
                target_dir: bind_dir.clone(),
                err: e
        })?;
        //mount(None::<&str>, &bind_dir, None::<&str>, ro_flags, None::<&str>)?;
        bind_dir.push(build_dir); // resets bind_dir to build dir
        dep_dir.pop(); // strips dependency package identifier
    }
    Ok(())
}

pub fn mount_out_dir(
    build_dir: &Path,
    out_dir: &Path,
) -> Result<(), NSError> {
    let flags = MsFlags::MS_BIND;
    let mut bind_dir = build_dir.to_path_buf();

    // This should be safe because of logic in build_cxt::exec_build
    bind_dir.push(out_dir.strip_prefix("/").unwrap());
    std::fs::create_dir_all(&bind_dir).map_err(
        |e| NSError::MkDirError(bind_dir.clone(),e)
    )?;
    mount(Some(out_dir), &bind_dir, None::<&str>, flags, None::<&str>).map_err(
        |e| NSError::BindMountError{
            source_dir: out_dir.into(),
            target_dir: bind_dir.clone(),
            err: e
    })?;
    Ok(())
}

pub fn umount_dep_dirs<'a> (
    pkg_store_dir: &Path,
    build_dir: &Path,
    deps: impl IntoIterator<Item=&'a PKG<'a>>
) -> Result<(), NSError> {
    let mut bind_dir = build_dir.to_path_buf();
    let mut dep_dir = pkg_store_dir.to_path_buf();
    for dep in deps {
        dep_dir.push(dep.pkg_ident());
        // This should be safe because of logic in build_cxt::exec_build
        bind_dir.push(dep_dir.strip_prefix("/").unwrap());
        umount(&bind_dir).map_err(
            |e| NSError::BindUMountError(bind_dir.clone(),e)
        )?;
        bind_dir.push(build_dir); // resets bind_dir to build dir
        dep_dir.pop(); // strips dependency package identifier
    }
    Ok(())
}

pub fn umount_out_dir(
    build_dir: &Path,
    out_dir: &Path,
) -> Result<(), NSError> {
    let mut bind_dir = build_dir.to_path_buf();
    // This should be safe because of logic in build_cxt::exec_build
    bind_dir.push(out_dir.strip_prefix("/").unwrap());
    umount(&bind_dir).map_err(
        |e| NSError::BindUMountError(bind_dir.clone(),e)
    )?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_uid_map() {
        let map = get_uid_map();
        assert!(map.len() > 4);
    }
}

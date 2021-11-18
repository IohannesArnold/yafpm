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
use std::slice::Iter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::os::unix::process::CommandExt;
use nix::unistd::chroot;

use super::Context;
use crate::namespace;
use crate::resource::Resource as RS;
use crate::package::Package as PKG;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug, thiserror::Error)]
pub enum InnerShellError {
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    NSError(#[from] namespace::NSError),
}

#[derive(Debug, thiserror::Error)]
/// The error returned by [ShellCxt].
pub enum ShellError {
    #[error("Unable to determine canonical path of {}", .path.display())]
    CanonicalizeError{err: io::Error, path: PathBuf},
    #[error("Error while setting up shell environment")]
    SetupError(#[source] super::ContextPrepError),
    #[error("Unable to execute shell command")]
    ExecCmdError(#[source] io::Error),
    #[error("Error while tearing down shell environment")]
    TeardownError(#[from] InnerShellError)
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShellCxt<'a> {
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    resources: Vec<RS<'a>>,
    #[cfg_attr(feature = "serde", serde(rename = "shell_dependencies"))]
    #[cfg_attr(feature = "serde", serde(alias = "build_dependencies"))]
    #[cfg_attr(feature = "serde", serde(alias = "dependencies"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    shell_deps: Vec<PKG<'a>>,
    #[cfg_attr(feature = "serde", serde(rename = "shell_command"))]
    #[cfg_attr(feature = "serde", serde(alias = "build_command"))]
    shell_cmd: &'a str,
}

impl<'a> Context<'a> for ShellCxt<'a> {
    type R = Iter<'a, RS<'a>>;
    type D = Iter<'a, PKG<'a>>;

    fn context_name(&self) -> String {
        use std::time::SystemTime;
        let d = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|e| e.duration());
        format!("shell-{}", d.as_secs())
    }


    fn resources(&'a self) -> Self::R {
        self.resources.iter()
    }

    fn dependencies(&'a self) -> Self::D {
        self.shell_deps.iter()
    }
}

impl<'a> ShellCxt<'a> {
    pub fn new(shell_cmd: &'a str) -> Self {
        ShellCxt {
            resources: Vec::new(),
            shell_deps: Vec::new(),
            shell_cmd
        }
    }

    pub fn add_resources<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = RS<'a>>
    {
        self.resources.extend(iter);
        self
    }

    pub fn add_deps<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = PKG<'a>>
    {
        self.shell_deps.extend(iter);
        self
    }

    pub fn change_shell_cmd(&mut self, new: &'a str) -> &mut Self {
        self.shell_cmd = new;
        self
    }

    fn exec_shell_cmd(
        &'a self,
        pkg_store_dir: &Path,
        context_dir: &PathBuf,
    ) -> Result<(), ShellError> {
        let dep_env_clos = |d: &PKG<'a>|
            (d.pkg_name, pkg_store_dir.join(d.pkg_ident()));
        let mut child = Command::new(self.shell_cmd);
        child.env_clear()
             .envs(self.dependencies().map(dep_env_clos))
             .env("PATH", self.make_path_string(pkg_store_dir))
             .current_dir(context_dir);
        // TODO there has to be an more elegant way of doing this
        let build_dir_clone = context_dir.clone();
        unsafe {
            child.pre_exec(move || {
                let res = chroot(&build_dir_clone);
                res.map_err(|e| if let Some(errno) = e.as_errno() {
                    io::Error::from_raw_os_error(errno as i32)
                } else {
                    io::Error::from_raw_os_error(0)
                })
            });
        }
        child.status().map_err(
            |e| ShellError::ExecCmdError(e)
        )?;

        Ok(())

    }

    fn teardown_shell(
        &self,
        pkg_store_dir: &Path,
        build_dir: &PathBuf,
    ) -> Result<(), InnerShellError> {
        namespace::umount_dep_dirs(&pkg_store_dir.as_ref(),
                                   &build_dir,
                                   self.dependencies())?;
        fs::remove_dir_all(&build_dir)?;
        Ok(())
    }

    pub fn enter_shell<P: AsRef<Path>> (
        self,
        pkg_store_dir: P
    ) -> Result<(), ShellError> {
        let abs_dir: PathBuf;
        // Be careful editing this. There are unwraps that rely on
        // pkg_store_dir and its derivatives being absolute.
        let pkg_store_dir = if pkg_store_dir.as_ref().is_absolute() {
            pkg_store_dir.as_ref()
        } else {
            abs_dir = pkg_store_dir.as_ref().canonicalize().map_err(
                |e| ShellError::CanonicalizeError {
                    err: e,
                    path: pkg_store_dir.as_ref().into()
            })?;
            abs_dir.as_ref()
        };
        let context_dir = self.prepare_context_dir(&pkg_store_dir).map_err(
            |e| ShellError::SetupError(e.into()))?;
        self.exec_shell_cmd(&pkg_store_dir, &context_dir)?;
        self.teardown_shell(&pkg_store_dir, &context_dir)?;

        Ok(())
    }
}

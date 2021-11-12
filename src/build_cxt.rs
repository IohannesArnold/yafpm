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
use std::path::{Path, PathBuf};
use std::process::{Command,ExitStatus};
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use blake2::Blake2s;
use nix::unistd::chroot;

use crate::dirs;
use crate::hashes;
use crate::walk_dir;
use crate::namespace;
use crate::resource;
use crate::resource::Resource as RS;
use crate::package::Package as PKG;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Debug, thiserror::Error)]
pub enum InnerBuildError {
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    NSError(#[from] namespace::NSError),
    #[error(transparent)]
    RSError(#[from] resource::ResourceError),
    #[error("The output directory already exists")]
    MaybeAlreadyInstalled(String),
}
#[derive(Debug, thiserror::Error)]
/// The error returned by [BuildCxt].
pub enum BuildError {
    #[error("Unable to determine canonical path of {}", .path.display())]
    CanonicalizeError{err:io::Error, path: PathBuf},
    #[error("Error while setting up build environment")]
    SetupError(#[source] InnerBuildError),
    #[error("Unable to execute build command")]
    ExecBuildCmdError(#[source] io::Error),
    #[error("Build process error: {0}")]
    BuildCmdError(ExitStatus),
    #[error("Error while hashing build result")]
    HashError{#[source] err: hashes::HashError, teardown_err: Option<io::Error>},
    #[error("Error while tearing down build environment")]
    TeardownError(#[source] InnerBuildError)
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// An environment for deterministically building a package.
pub struct BuildCxt<'a> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub pkg_info: PKG<'a>,
    #[cfg_attr(feature = "serde", serde(rename = "resources"))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    srcs: Vec<RS<'a>>,
    #[cfg_attr(feature = "serde", serde(rename = "build_dependencies"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    build_deps: Vec<PKG<'a>>,
    #[cfg_attr(feature = "serde", serde(rename = "build_command"))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    build_cmd: &'a str,
    #[cfg_attr(feature = "serde", serde(rename = "build_command_args"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    build_cmd_args: Vec<&'a str>,
    #[cfg_attr(feature = "serde", serde(rename = "build_env_vars"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    build_envs: HashMap<&'a str, &'a str>,
}

impl<'a> BuildCxt<'a> {
    pub fn new(
        pkg_name: &'a str,
        pkg_version: &'a str,
        hash: hashes::ItemHash<Blake2s>,
        build_cmd: &'a str,
        build_envs: HashMap<&'a str, &'a str>,
    ) -> Self {
        let pgk_info = PKG::new(
            pkg_name,
            pkg_version,
            hash
        );
        BuildCxt {
            pkg_info: pgk_info,
            srcs: Vec::new(),
            build_deps: Vec::new(),
            build_cmd,
            build_cmd_args: Vec::new(),
            build_envs,
        }
    }

    pub fn add_pkg_deps<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = PKG<'a>>
    {
        self.pkg_info.add_deps(iter);
        self
    }

    pub fn add_srcs<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = RS<'a>>
    {
        self.srcs.extend(iter);
        self
    }

    pub fn add_build_deps<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = PKG<'a>>
    {
        self.build_deps.extend(iter);
        self
    }

    pub fn add_build_cmd_args<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = &'a str>
    {
        self.build_cmd_args.extend(iter);
        self
    }

    fn setup_build_env<P: AsRef<Path>> (
        &self,
        pkg_store_dir: P
    ) -> Result<(PathBuf, PathBuf), InnerBuildError> {
        let build_dir = dirs::create_builddir(self.pkg_info.pkg_name)?;
        let pkg_ident = self.pkg_info.pkg_ident();
        let out_dir = dirs::create_outdir(&pkg_store_dir, &pkg_ident).map_err(
            |e| if let Some(17) = e.raw_os_error() {
                InnerBuildError::MaybeAlreadyInstalled(pkg_ident)
            } else { InnerBuildError::IOError(e) })?;
        for src in &self.srcs {
            src.fetch_resource(&build_dir)?;
        }
        namespace::setup_new_namespace()?;
        let all_deps = self.pkg_info.deps.iter().chain(self.build_deps.iter());
        namespace::mount_dep_dirs(&pkg_store_dir,
                                  &build_dir,
                                  &out_dir,
                                  all_deps)?;
        Ok((build_dir, out_dir))
    }

    fn exec_build_cmd(
        &self,
        build_dir: &PathBuf,
        out_dir: &PathBuf
    ) -> Result<(), BuildError> {
        let mut child = Command::new(self.build_cmd);
        child.env_clear()
             .args(&self.build_cmd_args)
             .env("OUT", out_dir.as_os_str())
             .envs(&self.build_envs)
             .current_dir(&build_dir);
        // TODO there has to be an more elegant way of doing this
        let build_dir_clone = build_dir.clone();
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
        let exit_status = child.status().map_err(
            |e| BuildError::ExecBuildCmdError(e)
        )?;
        if exit_status.success() {
            Ok(())
        } else {
            Err(BuildError::BuildCmdError(exit_status))
        }

    }

    fn verify_build_hash(&self, out_dir: &PathBuf) -> Result<(), BuildError> {
        let res = self.pkg_info.hash.verify_hash_from_fn(
            walk_dir::calculate_directory_hash,
            &out_dir);
        if let Err(e) = res {
            let e2 = fs::remove_dir_all(&out_dir).err();
            return Err(BuildError::HashError{
                err: e,
                teardown_err: e2,
            });
        }
        Ok(())
    }

    fn cleanup_post_build<P: AsRef<Path>> (
        &self,
        pkg_store_dir: P,
        build_dir: &PathBuf,
        out_dir: &PathBuf
    ) -> Result<(), InnerBuildError> {
        dirs::set_readonly_all(&out_dir, true)?;
        let all_deps = self.pkg_info.deps.iter().chain(self.build_deps.iter());
        namespace::umount_dep_dirs(&pkg_store_dir,
                                   &build_dir,
                                   &out_dir,
                                   all_deps)?;
        fs::remove_dir_all(&build_dir)?;
        Ok(())
    }

    pub fn exec_build<P: AsRef<Path>> (
        self,
        pkg_store_dir: P
    ) -> Result<PKG<'a>, BuildError> {
        let abs_dir: PathBuf;
        // Be careful editing this. There are unwraps in mod namespace that
        // rely on pkg_store_dir and its derivatives being absolute.
        let pkg_store_dir = if pkg_store_dir.as_ref().is_absolute() {
            pkg_store_dir.as_ref()
        } else {
            abs_dir = pkg_store_dir.as_ref().canonicalize().map_err(
                |e| BuildError::CanonicalizeError {
                    err: e,
                    path: pkg_store_dir.as_ref().into()
            })?;
            abs_dir.as_ref()
        };

        let build_dir: PathBuf;
        let out_dir: PathBuf;

        match self.setup_build_env(&pkg_store_dir) {
            Ok((bd, od)) => {
                build_dir = bd;
                out_dir = od;
            }
            Err(InnerBuildError::MaybeAlreadyInstalled(id)) => {
                out_dir = pkg_store_dir.join(id);
                return self.verify_build_hash(&out_dir).and(Ok(self.pkg_info));
            }
            Err(e) => { return Err(BuildError::SetupError(e)); }
        }
        self.exec_build_cmd(&build_dir, &out_dir)?;
        self.verify_build_hash(&out_dir)?;
        self.cleanup_post_build(&pkg_store_dir, &build_dir, &out_dir).map_err(
            |e| BuildError::TeardownError(e))?;
        Ok(self.pkg_info)
    }
}


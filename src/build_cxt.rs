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
use std::iter::Chain;
use std::path::{Path, PathBuf};
use std::process::{Command,ExitStatus};
use std::slice::Iter;
use std::os::unix::process::CommandExt;
use blake2::Blake2s;
use nix::unistd::chroot;

use crate::dirs;
use crate::hashes;
use crate::walk_dir;
use crate::namespace;
use crate::resource;
use crate::context;
use crate::context::Context;
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
    #[error(transparent)]
    CXTError(#[from] context::ContextPrepError),
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
    build_cmd: &'a str,
    #[cfg_attr(feature = "serde", serde(rename = "build_command_args"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    build_cmd_args: Vec<&'a str>,
}

impl<'a> Context<'a> for BuildCxt<'a> {
    type R = Iter< 'a, RS<'a>>;
    type D = Chain<Iter< 'a, PKG<'a>>, Iter< 'a, PKG<'a>>>;

    fn context_name(&self) -> &str {
        self.pkg_info.pkg_name
    }

    fn resources(&'a self) -> Self::R {
        self.srcs.iter()
    }

    fn dependencies(&'a self) -> Self::D {
        self.pkg_info.deps.iter().chain(&self.build_deps)
    }
}

impl<'a> BuildCxt<'a> {
    pub fn new(
        pkg_name: &'a str,
        pkg_version: &'a str,
        hash: hashes::ItemHash<Blake2s>,
        build_cmd: &'a str,
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

    fn setup_out_dir(
        &self,
        pkg_store_dir: &Path,
        build_dir: &Path,
    ) -> Result<PathBuf, InnerBuildError> {
        let pkg_ident = self.pkg_info.pkg_ident();
        let out_dir = dirs::create_outdir(&pkg_store_dir, &pkg_ident).map_err(
            |e| if let Some(17) = e.raw_os_error() {
                InnerBuildError::MaybeAlreadyInstalled(pkg_ident)
            } else { InnerBuildError::IOError(e) })?;
        namespace::mount_out_dir(build_dir, &out_dir)?;
        Ok(out_dir)
    }

    fn exec_build_cmd<P: AsRef<Path>> (
        &self,
        pkg_store_dir: P,
        build_dir: &PathBuf,
        out_dir: &PathBuf
    ) -> Result<(), BuildError> {
        let dep_env_clos = |d: &PKG<'a>|
            (d.pkg_name, pkg_store_dir.as_ref().join(d.pkg_ident()));
        let mut child = Command::new(self.build_cmd);
        child.env_clear()
             .args(&self.build_cmd_args)
             .envs(self.build_deps.iter().map(dep_env_clos))
             .envs(self.pkg_info.deps.iter().map(dep_env_clos))
             .envs(&self.pkg_info.build_settings)
             .env("out", out_dir.as_os_str())
             .env("PATH", self.make_path_string(pkg_store_dir.as_ref()))
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
        namespace::umount_out_dir(build_dir, out_dir)?;
        namespace::umount_dep_dirs(&pkg_store_dir.as_ref(),
                                   &build_dir,
                                   self.dependencies())?;
        fs::remove_dir_all(&build_dir)?;
        Ok(())
    }

    pub fn exec_build<P: AsRef<Path>> (
        self,
        pkg_store_dir: P
    ) -> Result<PKG<'a>, BuildError> {
        let abs_dir: PathBuf;
        // Be careful editing this. There are unwraps that rely on
        // pkg_store_dir and its derivatives being absolute.
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
        let out_dir: PathBuf;

        let build_dir = self.prepare_context_dir(&pkg_store_dir).map_err(
            |e| BuildError::SetupError(e.into()))?;
        match self.setup_out_dir(&pkg_store_dir, &build_dir) {
            Ok(od) => {
                out_dir = od;
            }
            Err(InnerBuildError::MaybeAlreadyInstalled(id)) => {
                out_dir = pkg_store_dir.join(id);
                return self.verify_build_hash(&out_dir).and(Ok(self.pkg_info));
            }
            Err(e) => { return Err(BuildError::SetupError(e)); }
        }
        self.exec_build_cmd(&pkg_store_dir, &build_dir, &out_dir)?;
        self.verify_build_hash(&out_dir)?;
        self.cleanup_post_build(&pkg_store_dir, &build_dir, &out_dir).map_err(
            |e| BuildError::TeardownError(e))?;
        Ok(self.pkg_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blake2::Digest;

    fn example_buildcxt() -> BuildCxt<'static> {
        let mut new = BuildCxt::new(
            "example",
            "1.0.0",
            Blake2s::digest(b"hello_world").into(),
            "./build.sh"
        );
        let dep = PKG::new(
            "dependency",
            "1.0.0",
            Blake2s::digest(b"hello_world").into(),
        );
        new.pkg_info.add_deps(Some(dep));
        new
    }

    #[test]
    fn test_make_path_string() {
        let ex = example_buildcxt();
        let s = ex.make_path_string("/root");
        assert_eq!(
            s,
            "/root/dependency-1.0.0-GNC4RH2YRCDAH7AHVIISWYE2JSD3PJXAQTRCMTGQLXJRULOJKI5A/bin/:"
        );
    }
}

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

mod build_cxt;
pub use build_cxt::BuildCxt;
pub use build_cxt::BuildError;

use std::io;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::dirs;
use crate::namespace;
use crate::resource;
use crate::package::Package as PKG;
use crate::resource::Resource as RS;

#[derive(Debug, thiserror::Error)]
pub enum ContextPrepError {
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    NSError(#[from] namespace::NSError),
    #[error(transparent)]
    RSError(#[from] resource::ResourceError),
}

pub trait Context<'a> {
    type R: IntoIterator<Item = &'a RS<'a>>;
    type D: IntoIterator<Item = &'a PKG<'a>>;

    fn context_name(&self) -> String;

    fn resources(&'a self) -> Self::R;

    fn dependencies(&'a self) -> Self::D;

    fn prepare_context_dir(
        &'a self,
        pkg_store_dir: &Path
    ) -> Result<PathBuf, ContextPrepError> {
        let context_dir = dirs::create_context_dir(&self.context_name())?;
        for src in self.resources() {
            src.fetch_resource(&context_dir)?;
        }
        namespace::setup_new_namespace()?;
        namespace::mount_dep_dirs(
            pkg_store_dir, &context_dir, self.dependencies()
        )?;

        Ok(context_dir)
    }

    fn make_path_string(&'a self, pkg_store_dir: &Path) -> OsString {
        let mut s = OsString::with_capacity(200);
        for dep in self.dependencies() {
            s.push(pkg_store_dir);
            s.push("/");
            s.push(dep.pkg_ident());
            s.push("/bin/:");
        }
        s
    }

}

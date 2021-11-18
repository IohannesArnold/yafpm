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

//! This is a library for system package management. It is still experimental,
//! so at present it only offers a way to build packages in a deterministic
//! way. The API is object-oriented, and at present the main object is
//! [BuildCxt].

mod context;
mod namespace;
mod walk_dir;
mod resource;
mod dirs;
mod hashes;
mod package;

pub use context::{BuildCxt, BuildError, ShellCxt, ShellError};
pub use resource::Resource;
#[cfg(feature = "serde")]
pub use resource::url_serde::SERDE_BASE_URL;
pub use package::Package;

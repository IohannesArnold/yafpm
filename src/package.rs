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

use std::path::PathBuf;
use blake2::Blake2s;
use data_encoding::BASE32_NOPAD;

use crate::hashes;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// A package that one might install to a system.
pub struct Package<'a> {
    #[cfg_attr(feature = "serde", serde(rename = "package_name"))]
    #[cfg_attr(feature = "serde", serde(alias = "name"))]
    pub pkg_name: &'a str,
    #[cfg_attr(feature = "serde", serde(rename = "package_version"))]
    #[cfg_attr(feature = "serde", serde(alias = "version"))]
    pkg_version: &'a str,
    #[cfg_attr(feature = "serde", serde(rename = "dependencies"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub(crate) deps: Vec<Package<'a>>,
    pub(crate) hash: hashes::ItemHash<Blake2s>
}

impl<'a> Package<'a> {
    pub fn new(
        pkg_name: &'a str,
        pkg_version: &'a str,
        hash: hashes::ItemHash<Blake2s>
    ) -> Self {
        Package {
            pkg_name,
            pkg_version,
            deps: Vec::new(),
            hash
        }
    }

    pub fn add_deps<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator<Item = Self>
    {
        self.deps.extend(iter);
        self
    }

    pub fn pkg_ident(&self) -> String {
        let mut ident = format!("{}-{}-", self.pkg_name, self.pkg_version);
        BASE32_NOPAD.encode_append(&self.hash.as_ref(), &mut ident);
        ident
    }

    pub fn is_installed(&self, pkg_store_dir: &mut PathBuf) -> bool {
        let ident = self.pkg_ident();
        pkg_store_dir.push(ident);
        let res = pkg_store_dir.exists();
        pkg_store_dir.pop();
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blake2::Blake2s;
    use blake2::Digest;

    #[test]
    fn test_pkg_ident() {
        let pkg = Package::new(
            "test",
            "1.0.0",
            Blake2s::digest(b"hello_world").into()
        );
        assert_eq!(
            pkg.pkg_ident().as_str(),
            "test-1.0.0-GNC4RH2YRCDAH7AHVIISWYE2JSD3PJXAQTRCMTGQLXJRULOJKI5A"
        );
    }
}

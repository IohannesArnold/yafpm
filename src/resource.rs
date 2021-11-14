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
use url::Url;
use blake2::Blake2s;

use crate::hashes;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "minreq")]
use minreq;

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Error while hashing resource {name}")]
    HashError {
        #[source]
        err: hashes::HashError,
        name: String
    },
    #[error("IO error while accessing {}", .file.display())]
    IOError {
        #[source]
        err: io::Error,
        file: PathBuf
    },
    #[cfg(feature = "minreq")]
    #[error("HTTP error while accessing {url}")]
    HTTPError {
        #[source]
        err: minreq::Error,
        url: Url
    },
    #[cfg(feature = "minreq")]
    #[error("Received HTTP response {} from {url}", response.status_code)]
    HTTPStatus {
        url: Url,
        response: minreq::Response
    },
    #[error("Resource {name} has unrecognized URL scheme: {scheme}")]
    Unrecognized{
        name: String,
        scheme: String,
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// A file that is used in the building of a package.
pub struct Resource<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    name: &'a str,
    hash: hashes::ItemHash<Blake2s>,
    #[cfg_attr(feature = "serde", serde(with = "url_serde"))]
    url: Url,
}

impl<'a> Resource<'a> {
    pub fn new (name: &'a str, hash: hashes::ItemHash<Blake2s>, url: Url) -> Self {
        Resource { name, hash, url }
    }

    fn verify_hash(&self, fd: &mut fs::File) -> Result <u64, hashes::HashError> {
        self.hash.verify_hash_from_fn(io::copy, fd)
    }

    fn fetch_file<P: AsRef<Path>>(
        &self,
        build_dir: P,
    ) -> Result <(), ResourceError> {
        let src_path = Path::new(self.url.path());
        let mut file = fs::File::open(src_path).map_err(
            |e| ResourceError::IOError{err: e, file: PathBuf::from(src_path)})?;
        self.verify_hash(&mut file).map_err(
            |e| ResourceError::HashError{err: e, name: self.name.to_string()})?;
        let target = build_dir.as_ref().join(self.name);
        fs::copy(src_path, target).map_err(
            |e| ResourceError::IOError{err: e, file: PathBuf::from(src_path)})?;
        Ok(())
    }

    #[cfg(feature = "minreq")]
    fn fetch_http<P: AsRef<Path>>(
        &self,
        build_dir: P,
    ) -> Result <(), ResourceError> {
        let response = minreq::get(self.url.as_str()).send().map_err(
            |e| ResourceError::HTTPError{err: e, url: self.url.clone()})?;
        if response.status_code != 200 {
            return Err(ResourceError::HTTPStatus{
                url: self.url.clone(),
                response: response });
        }
        self.hash.verify_hash_from_fn(io::copy, &mut response.as_bytes()).map_err(
            |e| ResourceError::HashError{err: e, name: self.name.to_string()})?;
        let target = build_dir.as_ref().join(self.name);
        fs::write(&target, response.into_bytes()).map_err(
            |e| ResourceError::IOError{err: e, file: target})?;
        Ok(())
    }

    pub(crate) fn fetch_resource<P: AsRef<Path>>(
        &self,
        build_dir: P
    ) -> Result <(), ResourceError> {
        match self.url.scheme() {
            "file" =>  self.fetch_file(&build_dir),
            #[cfg(feature = "minreq")]
            "http" => self.fetch_http(&build_dir),
            #[cfg(feature = "minreq-https")]
            "https" => self.fetch_http(&build_dir),
            scheme =>  Err(ResourceError::Unrecognized{
                scheme: scheme.to_string(),
                name: self.name.to_string()
            })
        }
    }
}

#[cfg(feature = "serde")]
pub mod url_serde {
    use std::fmt;
    use serde::{ser,de};
    use url::Url;

    pub fn serialize<S: ser::Serializer>(
        url: &Url,
        serializer: S
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(url.as_str())
    }

    /// This static is only useful for library users who will be deserializing
    /// build contexts. It allows end users to refer to a local by writing:
    /// ```TOML
    /// url = "./example.sh"
    /// ```
    /// instead of having to write:
    /// ```TOML
    /// url = "file://absolute/path/to/example.sh"
    /// ```
    /// Note that there is no mutex or other type of protective wrapper
    /// around this; it's just an option. `yafpm-build` is single-threaded and
    /// hasn't needed such protections. But if your use case does, then please
    /// file an issue.
    pub static mut SERDE_BASE_URL: Option<Url> = None;

    struct UrlVisitor;

    impl<'de> de::Visitor<'de> for UrlVisitor {
        type Value = Url;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string representing an URL")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let base_url;
            // I don't know any way to provide another argument to deserialize
            // functions, so a static is all I can think of to smuggle in
            // a base url. Right now there are no mutexes or other protections,
            // but it is modified by yafpm-build a maximum of one time before
            // use, so I think it should be okay.
            unsafe {
                let options = Url::options();
                base_url = options.base_url(SERDE_BASE_URL.as_ref());
            }

            base_url.parse(s).map_err(|err| {
                let err_s = format!("{}", err);
                E::invalid_value(de::Unexpected::Str(s), &err_s.as_str())
            })
        }
    }

    pub fn deserialize<'de, D: de::Deserializer<'de>>(
        deserializer: D
    ) -> Result<Url, D::Error> {
        deserializer.deserialize_str(UrlVisitor)
    }
}

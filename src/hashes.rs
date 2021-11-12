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
use std::fmt;
use digest::Digest;
use digest::generic_array::GenericArray;
use data_encoding::HEXLOWER;

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("Expected hash: {} Found hash: {}",
            HEXLOWER.encode(.expected),
            HEXLOWER.encode(.found))]
    BadHash{expected: Vec<u8>, found: Vec<u8>},
    #[error(transparent)]
    IOError(#[from]io::Error)
}

type InnerGA<D> = GenericArray<u8, <D as Digest>::OutputSize>;

pub struct ItemHash<D: Digest>(InnerGA<D>);

impl<D: Digest> ItemHash<D> {
    pub fn verify_hash_from_fn<T,S>(
        &self,
        func: impl Fn(T, &mut D) -> Result<S, io::Error>,
        object: T
    ) -> Result<S, HashError> {
        let mut hasher = D::new();
        let ok = func(object, &mut hasher)?;
        let hasher_result = hasher.result();
        if hasher_result != self.0 {
            return Err(HashError::BadHash {
                expected: self.0.to_vec(),
                found: hasher_result.to_vec()
            });
        }
        Ok(ok)
    }
}

impl<D: Digest> fmt::LowerHex for ItemHash<D>
    where InnerGA<D>: fmt::LowerHex
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl<D: Digest> From<InnerGA<D>> for ItemHash<D> {
    fn from(d: InnerGA<D>) -> Self {
        ItemHash(d)
    }
}

impl<D: Digest> AsRef<[u8]> for ItemHash<D> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}


// TODO: Find some way (enum? trait object?) to encapsulate the type argument
// of ItemHash and make Resource, BuildCxt, etc, able to use different hash
// algos without needing a type argument of their own.

#[cfg(feature = "serde")]
mod serde_impl{
    use super::*;
    use serde::{ser,de};
    use data_encoding::HEXLOWER_PERMISSIVE as HEX;

    impl<D: Digest> ser::Serialize for ItemHash<D> where 
        InnerGA<D>: fmt::LowerHex
    {
        fn serialize<S: ser::Serializer>(
            &self,
            serializer: S
        ) -> Result<S::Ok, S::Error> {
            let output = format!("{:x}", self.0);
            serializer.serialize_str(&output)
        }
    }

    struct ItemHashVisitor<H: Digest>(std::marker::PhantomData<H>);
    
    impl<H: Digest> ItemHashVisitor<H> {
        fn new() -> Self {ItemHashVisitor(std::marker::PhantomData)}
    }

    impl<'de, H: Digest> de::Visitor<'de> for ItemHashVisitor<H> {
        type Value = ItemHash<H>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let expected_len = HEX.encode_len(<H as Digest>::output_size());
            write!(f, "a {} char hexadecimal string", expected_len)
        }

        fn visit_borrowed_str<E: de::Error> (
            self,
            v:&'de str
        ) -> Result<Self::Value, E> {
            let mut arr = InnerGA::<H>::default();
            let expected_len = HEX.encode_len(<H as Digest>::output_size());
            let found_len = v.len();
            if found_len != expected_len {
                return Err(E::invalid_length(found_len, &self));
            }
            // On its own, this can panic, but we should have ruled out the
            // possibility above
            HEX.decode_mut(v.as_bytes(), &mut arr).map_err(
                |e| E::custom(format!("hex parsing error, {}", e.error)))?;

            Ok(ItemHash(arr))
        }
    }

    impl<'de, 'a, H: Digest> de::Deserialize<'de> for ItemHash<H> {
        fn deserialize<D: de::Deserializer<'de>>(
            deserializer: D
        ) -> Result<Self, D::Error> {
            deserializer.deserialize_str(ItemHashVisitor::<H>::new())
        }
    }
}

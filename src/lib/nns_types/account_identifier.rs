// Copied (and trimmed) from https://raw.githubusercontent.com/dfinity/ic/master/rs/rosetta-api/ledger_canister/src/account_identifier.rs
// Commit: 779549eccfcf61ac702dfc2ee6d76ffdc2db1f7f

use candid::CandidType;
use ic_types::principal::Principal as PrincipalId;
use openssl::sha::Sha224;
use serde::{de, de::Error, Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{Display, Formatter},
    str::FromStr,
};

/// While this is backed by an array of length 28, it's canonical representation
/// is a hex string of length 64. The first 8 characters are the CRC-32 encoded
/// hash of the following 56 characters of hex. Both, upper and lower case
/// characters are valid in the input string and can even be mixed.
///
/// When it is encoded or decoded it will always be as a string to make it
/// easier to use from DFX.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountIdentifier {
    pub hash: [u8; 28],
}

pub static SUB_ACCOUNT_ZERO: Subaccount = Subaccount([0; 32]);
static ACCOUNT_DOMAIN_SEPERATOR: &[u8] = b"\x0Aaccount-id";

impl AccountIdentifier {
    pub fn new(account: PrincipalId, sub_account: Option<Subaccount>) -> AccountIdentifier {
        let mut hash = Sha224::new();
        hash.update(ACCOUNT_DOMAIN_SEPERATOR);
        hash.update(account.as_slice());

        let sub_account = sub_account.unwrap_or(SUB_ACCOUNT_ZERO);
        hash.update(&sub_account.0[..]);

        AccountIdentifier {
            hash: hash.finish(),
        }
    }

    pub fn from_hex(hex_str: &str) -> Result<AccountIdentifier, String> {
        let hex: Vec<u8> = hex::decode(hex_str).map_err(|e| e.to_string())?;
        Self::from_slice(&hex[..])
    }

    /// Goes from the canonical format (with checksum) encoded in bytes rather
    /// than hex to AccountIdentifier
    pub fn from_slice(v: &[u8]) -> Result<AccountIdentifier, String> {
        // Trim this down when we reach rust 1.48
        let hex: Box<[u8; 32]> = match v.to_vec().into_boxed_slice().try_into() {
            Ok(h) => h,
            Err(_) => {
                let hex_str = hex::encode(v);
                return Err(format!(
                    "{} has a length of {} but we expected a length of 64",
                    hex_str,
                    hex_str.len()
                ));
            }
        };
        check_sum(*hex)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.to_vec())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        [&self.generate_checksum()[..], &self.hash[..]].concat()
    }

    pub fn generate_checksum(&self) -> [u8; 4] {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.hash);
        hasher.finalize().to_be_bytes()
    }
}

impl Display for AccountIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_hex().fmt(f)
    }
}

impl FromStr for AccountIdentifier {
    type Err = String;

    fn from_str(s: &str) -> Result<AccountIdentifier, String> {
        AccountIdentifier::from_hex(s)
    }
}

impl Serialize for AccountIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccountIdentifier {
    // This is the canonical way to read a this from string
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
        D::Error: de::Error,
    {
        let hex: [u8; 32] = hex::serde::deserialize(deserializer)?;
        check_sum(hex).map_err(D::Error::custom)
    }
}

impl From<PrincipalId> for AccountIdentifier {
    fn from(pid: PrincipalId) -> Self {
        AccountIdentifier::new(pid, None)
    }
}

fn check_sum(hex: [u8; 32]) -> Result<AccountIdentifier, String> {
    // Get the checksum provided
    let found_checksum = &hex[0..4];

    // Copy the hash into a new array
    let mut hash = [0; 28];
    hash.copy_from_slice(&hex[4..32]);

    let account_id = AccountIdentifier { hash };
    let expected_checksum = account_id.generate_checksum();

    // Check the generated checksum matches
    if expected_checksum == found_checksum {
        Ok(account_id)
    } else {
        Err(format!(
            "Checksum failed for {}, expected check bytes {} but found {}",
            hex::encode(&hex[..]),
            hex::encode(expected_checksum),
            hex::encode(found_checksum),
        ))
    }
}

impl CandidType for AccountIdentifier {
    // The type expected for account identifier is
    fn _ty() -> candid::types::Type {
        String::_ty()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        self.to_hex().idl_serialize(serializer)
    }
}

/// Subaccounts are arbitrary 32-byte values.
#[derive(Serialize, Deserialize, CandidType, Clone, Hash, Debug, PartialEq, Eq, Copy)]
#[serde(transparent)]
pub struct Subaccount(pub [u8; 32]);

impl From<&PrincipalId> for Subaccount {
    fn from(principal_id: &PrincipalId) -> Self {
        let mut subaccount = [0; std::mem::size_of::<Subaccount>()];
        let principal_id = principal_id.as_slice();
        subaccount[0] = principal_id.len().try_into().unwrap();
        subaccount[1..1 + principal_id.len()].copy_from_slice(principal_id);
        Subaccount(subaccount)
    }
}

impl Into<Vec<u8>> for Subaccount {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl TryFrom<&[u8]> for Subaccount {
    type Error = std::array::TryFromSliceError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        slice.try_into().map(Subaccount)
    }
}

impl Display for Subaccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

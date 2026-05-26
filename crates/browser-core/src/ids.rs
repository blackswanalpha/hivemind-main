use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::StoreError;

macro_rules! ulid_newtype {
    ($name:ident) => {
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Ulid);

        impl $name {
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            pub fn into_inner(self) -> Ulid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl FromStr for $name {
            type Err = StoreError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ulid::from_string(s)
                    .map(Self)
                    .map_err(|e| StoreError::invalid(format!(
                        "invalid {} `{}`: {}",
                        stringify!($name),
                        s,
                        e
                    )))
            }
        }

        impl From<Ulid> for $name {
            fn from(u: Ulid) -> Self {
                Self(u)
            }
        }
    };
}

ulid_newtype!(TabId);
ulid_newtype!(WorkspaceId);
ulid_newtype!(ConversationId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_tab_id() {
        let id = TabId::new();
        let s = id.to_string();
        let parsed: TabId = s.parse().expect("parse round-trip");
        assert_eq!(id, parsed);
    }

    #[test]
    fn rejects_garbage() {
        let err = "not-a-ulid".parse::<TabId>().unwrap_err();
        assert!(matches!(err, StoreError::Invalid(_)));
    }
}

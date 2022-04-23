use scylla::{FromRow, ValueList};
use poem_openapi::Object;
use backend_common::types::{JsSafeBigInt, Set, Timestamp};
use backend_common::FieldNamesAsArray;
use backend_common::tags::PackTags;

#[derive(
    Object,
    FromRow,
    ValueList,
    FieldNamesAsArray,
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
#[oai(rename_all = "camelCase")]
pub struct Pack {
    /// The ID of the pack.
    pub id: JsSafeBigInt,

    /// The name of the pack.
    pub name: String,

    /// The description of the pack.
    pub description: String,

    /// The timestamp of when the pack was created.
    pub created_on: Timestamp,

    /// The tag associated with this pack.
    pub tag: PackTags,

    /// The bots that this pack contains. (In the form of IDs)
    pub bots: Vec<JsSafeBigInt>,

    /// If true the pack is removed from any public viewing.
    ///
    /// The owner is free to enable/disable this however they please.
    pub is_hidden: bool,

    /// If true the pack is removed from any public viewing.
    ///
    /// Only Dlist admins are able to enable/disable this.
    pub is_forced_into_hiding: bool,

    /// The primary owner of this pack.
    pub owner_id: JsSafeBigInt,

    /// The set of co-owners of this pack.
    pub co_owner_ids: Set<JsSafeBigInt>,
}

use poem_openapi::Enum;

#[derive(Enum, Debug, Copy, Clone)]
pub enum PacksSortBy {
    /// Sort by relevance.
    Relevance,

    /// Sort by votes.
    Likes,

    /// Sort by the trending score.
    Trending,

    /// How many bots the pack is in.
    NumBots,

    /// Premium Packs.
    Premium,
}

impl Default for PacksSortBy {
    fn default() -> Self {
        Self::Relevance
    }
}

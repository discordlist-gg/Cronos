use poem_openapi::Object;
use backend_common::FieldNamesAsArray;
use backend_common::tags::BotTags;
use backend_common::types::{DiscordUrl, JsSafeBigInt, JsSafeInt, Set, Timestamp};

#[derive(Object, FieldNamesAsArray)]
#[oai(rename_all = "camelCase")]
pub struct Bot {
    /// The snowflake ID of the bot.
    pub id: JsSafeBigInt,

    /// The bot's username.
    pub username: String,

    /// The bot's avatar hash if applicable.
    pub avatar: Option<String>,

    /// The bot's discriminator i.e `0001`
    pub discriminator: JsSafeInt,

    /// The bot's given prefix.
    pub prefix: Option<String>,

    /// Is the bot able to be put in a pack?
    pub is_packable: bool,

    /// The custom invite URL.
    ///
    /// If set this will be used over the auto-generated invite URL.
    pub custom_url: Option<DiscordUrl>,

    /// The bot's website.
    pub website_url: Option<DiscordUrl>,

    /// The bot's code repository.
    pub repo_url: Option<DiscordUrl>,

    /// The bot's twitter.
    pub twitter_url: Option<DiscordUrl>,

    /// The bot's instagram.
    pub instagram_url: Option<DiscordUrl>,

    /// The bot's support server.
    pub support_server_url: Option<DiscordUrl>,

    /// The bot's custom slug which can be used to access the bot.
    pub slug: Option<String>,

    /// The given Dlist flags.
    pub flags: JsSafeBigInt,

    /// The bot's given list of features.
    ///
    /// This is stored in the form of a bitflag(s).
    pub features: JsSafeBigInt,

    /// The bot's required invite permissions.
    pub permissions: JsSafeBigInt,

    /// The bot's associated tags.
    pub tags: Set<BotTags>,

    /// The timestamp that the bot was first created on.
    pub created_on: Timestamp,

    /// Is the guild temporarily hidden from the public listing.
    pub is_hidden: bool,

    /// Is the guild forced into being hidden by a Dlist admin.
    pub is_forced_into_hiding: bool,

    /// The bot's primary owner.
    pub owner_id: JsSafeBigInt,

    /// The bot's secondary/co-owners
    pub co_owner_ids: Set<JsSafeBigInt>,

    /// The amount of guilds the bot is in.
    pub guild_count: Option<JsSafeInt>,

    /// The short description of the bot.
    pub brief_description: String,
}

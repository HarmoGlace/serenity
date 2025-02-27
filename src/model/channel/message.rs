//! Models relating to Discord channels.

#[cfg(feature = "model")]
use std::fmt::Display;
#[cfg(all(feature = "cache", feature = "model"))]
use std::fmt::Write;

#[cfg(all(feature = "model", feature = "utils"))]
use crate::builder::{CreateEmbed, EditMessage};
#[cfg(all(feature = "cache", feature = "model"))]
use crate::cache::Cache;
#[cfg(feature = "collector")]
use crate::client::bridge::gateway::ShardMessenger;
#[cfg(feature = "collector")]
use crate::collector::{
    CollectComponentInteraction,
    CollectModalInteraction,
    CollectReaction,
    ComponentInteractionCollectorBuilder,
    ModalInteractionCollectorBuilder,
    ReactionCollectorBuilder,
};
#[cfg(feature = "model")]
use crate::http::{CacheHttp, Http};
#[cfg(feature = "model")]
use crate::json;
use crate::json::prelude::*;
use crate::model::application::component::ActionRow;
use crate::model::application::interaction::MessageInteraction;
use crate::model::prelude::*;
#[cfg(feature = "model")]
use crate::{
    constants,
    model::{
        id::{ApplicationId, ChannelId, GuildId, MessageId},
        sticker::StickerItem,
        timestamp::Timestamp,
    },
};

/// A representation of a message over a guild's text channel, a group, or a
/// private channel.
///
/// [Discord docs](https://discord.com/developers/docs/resources/channel#message-object) with some
/// [extra fields](https://discord.com/developers/docs/topics/gateway-events#message-create-message-create-extra-fields).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Message {
    /// The unique Id of the message. Can be used to calculate the creation date
    /// of the message.
    pub id: MessageId,
    /// The Id of the [`Channel`] that the message was sent to.
    pub channel_id: ChannelId,
    /// The user that sent the message.
    pub author: User,
    /// The content of the message.
    pub content: String,
    /// Initial message creation timestamp, calculated from its Id.
    pub timestamp: Timestamp,
    /// The timestamp of the last time the message was updated, if it was.
    pub edited_timestamp: Option<Timestamp>,
    /// Indicator of whether the command is to be played back via
    /// text-to-speech.
    ///
    /// In the client, this is done via the `/tts` slash command.
    pub tts: bool,
    /// Indicator of whether the message mentions everyone.
    pub mention_everyone: bool,
    /// Array of users mentioned in the message.
    pub mentions: Vec<User>,
    /// Array of [`Role`]s' Ids mentioned in the message.
    pub mention_roles: Vec<RoleId>,
    /// Channels specifically mentioned in this message.
    ///
    /// **Note**:
    /// Not all channel mentions in a message will appear in [`Self::mention_channels`]. Only textual
    /// channels that are visible to everyone in a lurkable guild will ever be included.
    ///
    /// A lurkable guild is one that allows users to read public channels in a server without
    /// actually joining the server. It also allows users to look at these channels without being
    /// logged in to Discord.
    ///
    /// Only crossposted messages (via Channel Following) currently include [`Self::mention_channels`] at
    /// all. If no mentions in the message meet these requirements, this field will not be sent.
    /// [Refer to Discord's documentation for more information][discord-docs].
    ///
    /// [discord-docs]: https://discord.com/developers/docs/resources/channel#message-object
    #[serde(default = "Vec::new")]
    pub mention_channels: Vec<ChannelMention>,
    /// An vector of the files attached to a message.
    pub attachments: Vec<Attachment>,
    /// Array of embeds sent with the message.
    pub embeds: Vec<Embed>,
    /// Array of reactions performed on the message.
    #[serde(default)]
    pub reactions: Vec<MessageReaction>,
    /// Non-repeating number used for ensuring message order.
    #[serde(default)]
    pub nonce: Value,
    /// Indicator of whether the message is pinned.
    pub pinned: bool,
    /// The Id of the webhook that sent this message, if one did.
    pub webhook_id: Option<WebhookId>,
    /// Indicator of the type of message this is, i.e. whether it is a regular
    /// message or a system message.
    #[serde(rename = "type")]
    pub kind: MessageType,
    /// Sent with Rich Presence-related chat embeds.
    pub activity: Option<MessageActivity>,
    /// Sent with Rich Presence-related chat embeds.
    pub application: Option<MessageApplication>,
    /// If the message is an Interaction or application-owned webhook, this is the id of the
    /// application.
    pub application_id: Option<ApplicationId>,
    /// Reference data sent with crossposted messages.
    pub message_reference: Option<MessageReference>,
    /// Bit flags describing extra features of the message.
    pub flags: Option<MessageFlags>,
    /// The message that was replied to using this message.
    pub referenced_message: Option<Box<Message>>, // Boxed to avoid recursion
    /// Sent if the message is a response to an [`Interaction`].
    ///
    /// [`Interaction`]: crate::model::application::interaction::Interaction
    pub interaction: Option<MessageInteraction>,
    /// The thread that was started from this message, includes thread member object.
    pub thread: Option<GuildChannel>,
    /// The components of this message
    #[serde(default)]
    pub components: Vec<ActionRow>,
    /// Array of message sticker item objects.
    #[serde(default)]
    pub sticker_items: Vec<StickerItem>,
    // Field omitted: stickers (it's deprecated by Discord)
    /// The Id of the [`Guild`] that the message was sent in. This value will
    /// only be present if this message was received over the gateway.
    pub guild_id: Option<GuildId>,
    /// A partial amount of data about the user's member data, if this message
    /// was sent in a guild.
    pub member: Option<PartialMember>,
}

#[cfg(feature = "model")]
impl Message {
    /// Crossposts this message.
    ///
    /// Requires either to be the message author or to have manage [Manage Messages] permissions on this channel.
    ///
    /// **Note**: Only available on news channels.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// Returns a [`ModelError::MessageAlreadyCrossposted`] if the message has already been crossposted.
    ///
    /// Returns a [`ModelError::CannotCrosspostMessage`] if the message cannot be crossposted.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn crosspost(&self, cache_http: impl CacheHttp) -> Result<Message> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.author.id != cache.current_user_id() && self.guild_id.is_some() {
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::MANAGE_MESSAGES,
                    )?;
                }
            }
        }

        if let Some(flags) = self.flags {
            if flags.contains(MessageFlags::CROSSPOSTED) {
                return Err(Error::Model(ModelError::MessageAlreadyCrossposted));
            } else if flags.contains(MessageFlags::IS_CROSSPOST)
                || self.kind != MessageType::Regular
            {
                return Err(Error::Model(ModelError::CannotCrosspostMessage));
            }
        }

        self.channel_id.crosspost(cache_http.http(), self.id.0).await
    }

    /// First attempts to find a [`Channel`] by its Id in the cache,
    /// upon failure requests it via the REST API.
    ///
    /// **Note**: If the `cache`-feature is enabled permissions will be checked and upon
    /// owning the required permissions the HTTP-request will be issued.
    ///
    /// # Errors
    ///
    /// Can return an error if the HTTP request fails.
    #[inline]
    pub async fn channel(&self, cache_http: impl CacheHttp) -> Result<Channel> {
        self.channel_id.to_channel(cache_http).await
    }

    /// A util function for determining whether this message was sent by someone else, or the
    /// bot.
    #[cfg(feature = "cache")]
    pub fn is_own(&self, cache: impl AsRef<Cache>) -> bool {
        self.author.id == cache.as_ref().current_user().id
    }

    /// Deletes the message.
    ///
    /// **Note**: The logged in user must either be the author of the message or
    /// have the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` feature is enabled, then returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn delete(&self, cache_http: impl CacheHttp) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.author.id != cache.current_user_id() {
                    if self.is_private() {
                        return Err(Error::Model(ModelError::NotAuthor));
                    }
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::MANAGE_MESSAGES,
                    )?;
                }
            }
        }

        self.channel_id.delete_message(&cache_http.http(), self.id).await
    }

    /// Deletes all of the [`Reaction`]s associated with the message.
    ///
    /// **Note**: Requires the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` feature is enabled, then returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn delete_reactions(&self, cache_http: impl CacheHttp) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                utils::user_has_perms_cache(
                    cache,
                    self.channel_id,
                    self.guild_id,
                    Permissions::MANAGE_MESSAGES,
                )?;
            }
        }

        cache_http.http().as_ref().delete_message_reactions(self.channel_id.0, self.id.0).await
    }

    /// Deletes all of the [`Reaction`]s of a given emoji associated with the message.
    ///
    /// **Note**: Requires the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` feature is enabled, then returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn delete_reaction_emoji(
        &self,
        cache_http: impl CacheHttp,
        reaction_type: impl Into<ReactionType>,
    ) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                utils::user_has_perms_cache(
                    cache,
                    self.channel_id,
                    self.guild_id,
                    Permissions::MANAGE_MESSAGES,
                )?;
            }
        }

        cache_http
            .http()
            .as_ref()
            .delete_message_reaction_emoji(self.channel_id.0, self.id.0, &reaction_type.into())
            .await
    }

    /// Edits this message, replacing the original content with new content.
    ///
    /// Message editing preserves all unchanged message data.
    ///
    /// Refer to the documentation for [`EditMessage`] for more information
    /// regarding message restrictions and requirements.
    ///
    /// **Note**: Requires that the current user be the author of the message.
    ///
    /// # Examples
    ///
    /// Edit a message with new content:
    ///
    /// ```rust,ignore
    /// // assuming a `message` has already been bound
    ///
    /// message.edit(&context, |m| m.content("new content"));
    /// ```
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a [`ModelError::InvalidUser`] if the
    /// current user is not the author.
    ///
    /// Returns a [`ModelError::MessageTooLong`] if the content of the message
    /// is over [`the limit`], containing the number of unicode code points
    /// over the limit.
    ///
    /// [`the limit`]: crate::builder::EditMessage::content
    pub async fn edit<'a, F>(&mut self, cache_http: impl CacheHttp, f: F) -> Result<()>
    where
        F: for<'b> FnOnce(&'b mut EditMessage<'a>) -> &'b mut EditMessage<'a>,
    {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.author.id != cache.current_user_id() {
                    return Err(Error::Model(ModelError::InvalidUser));
                }
            }
        }
        let mut builder = self._prepare_edit_builder();
        f(&mut builder);
        self._send_edit(cache_http.http(), builder).await
    }

    fn _prepare_edit_builder<'a>(&self) -> EditMessage<'a> {
        let mut builder = EditMessage::default();

        if !self.content.is_empty() {
            builder.content(&self.content);
        }

        let embeds: Vec<_> = self.embeds.iter().map(|e| CreateEmbed::from(e.clone())).collect();
        builder.set_embeds(embeds);

        for attachment in &self.attachments {
            builder.add_existing_attachment(attachment.id);
        }
        builder
    }

    async fn _send_edit<'a>(&mut self, http: &Http, builder: EditMessage<'a>) -> Result<()> {
        let map = json::hashmap_to_json_map(builder.0);

        *self = http
            .edit_message_and_attachments(
                self.channel_id.0,
                self.id.0,
                &Value::from(map),
                builder.1,
            )
            .await?;
        Ok(())
    }

    pub(crate) fn transform_content(&mut self) {
        match self.kind {
            MessageType::PinsAdd => {
                self.content =
                    format!("{} pinned a message to this channel. See all the pins.", self.author);
            },
            MessageType::MemberJoin => {
                let sec = self.timestamp.unix_timestamp() as usize;
                let chosen = constants::JOIN_MESSAGES[sec % constants::JOIN_MESSAGES.len()];

                self.content = if chosen.contains("$user") {
                    chosen.replace("$user", &self.author.mention().to_string())
                } else {
                    chosen.to_string()
                };
            },
            _ => {},
        }
    }

    /// Returns message content, but with user and role mentions replaced with
    /// names and everyone/here mentions cancelled.
    #[cfg(feature = "cache")]
    pub fn content_safe(&self, cache: impl AsRef<Cache>) -> String {
        let mut result = self.content.clone();

        // First replace all user mentions.
        for u in &self.mentions {
            let mut at_distinct = String::with_capacity(38);
            at_distinct.push('@');
            at_distinct.push_str(&u.name);
            at_distinct.push('#');
            write!(at_distinct, "{:04}", u.discriminator).unwrap();

            let mut m = u.mention().to_string();
            // Check whether we're replacing a nickname mention or a normal mention.
            // `UserId::mention` returns a normal mention. If it isn't present in the message, it's a nickname mention.
            if !result.contains(&m) {
                m.insert(2, '!');
            }

            result = result.replace(&m, &at_distinct);
        }

        // Then replace all role mentions.
        for id in &self.mention_roles {
            let mention = id.mention().to_string();

            if let Some(role) = id.to_role_cached(&cache) {
                result = result.replace(&mention, &format!("@{}", role.name));
            } else {
                result = result.replace(&mention, "@deleted-role");
            }
        }

        // And finally replace everyone and here mentions.
        result.replace("@everyone", "@\u{200B}everyone").replace("@here", "@\u{200B}here")
    }

    /// Gets the list of [`User`]s who have reacted to a [`Message`] with a
    /// certain [`Emoji`].
    ///
    /// The default `limit` is `50` - specify otherwise to receive a different
    /// maximum number of users. The maximum that may be retrieve at a time is
    /// `100`, if a greater number is provided then it is automatically reduced.
    ///
    /// The optional `after` attribute is to retrieve the users after a certain
    /// user. This is useful for pagination.
    ///
    /// **Note**: Requires the [Read Message History] permission.
    ///
    /// **Note**: If the passed reaction_type is a custom guild emoji, it must contain the name. So,
    /// [`Emoji`] or [`EmojiIdentifier`] will always work, [`ReactionType`] only if
    /// [`ReactionType::Custom::name`] is Some, and **[`EmojiId`] will never work**.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if the current user lacks permission.
    ///
    /// [Read Message History]: Permissions::READ_MESSAGE_HISTORY
    #[inline]
    pub async fn reaction_users(
        &self,
        http: impl AsRef<Http>,
        reaction_type: impl Into<ReactionType>,
        limit: Option<u8>,
        after: impl Into<Option<UserId>>,
    ) -> Result<Vec<User>> {
        self.channel_id.reaction_users(&http, self.id, reaction_type, limit, after).await
    }

    /// Returns the associated [`Guild`] for the message if one is in the cache.
    ///
    /// Returns [`None`] if the guild's Id could not be found via [`Self::guild_id`] or
    /// if the Guild itself is not cached.
    ///
    /// Requires the `cache` feature be enabled.
    #[cfg(feature = "cache")]
    pub fn guild(&self, cache: impl AsRef<Cache>) -> Option<Guild> {
        cache.as_ref().guild(self.guild_id?)
    }

    /// Returns a field to the [`Guild`] for the message if one is in the cache.
    /// The field can be selected via the `field_accessor`.
    ///
    /// Returns [`None`] if the guild's ID could not be found via [`Self::guild_id`] or
    /// if the Guild itself is not cached.
    ///
    /// Requires the `cache` feature be enabled.
    #[cfg(feature = "cache")]
    pub fn guild_field<Ret, Fun>(
        &self,
        cache: impl AsRef<Cache>,
        field_accessor: Fun,
    ) -> Option<Ret>
    where
        Ret: Clone,
        Fun: FnOnce(&Guild) -> Ret,
    {
        cache.as_ref().guild_field(self.guild_id?, field_accessor)
    }

    /// True if message was sent using direct messages.
    #[inline]
    #[must_use]
    pub fn is_private(&self) -> bool {
        self.guild_id.is_none()
    }

    /// Retrieves a clone of the author's Member instance, if this message was
    /// sent in a guild.
    ///
    /// If the instance cannot be found in the cache, or the `cache` feature is
    /// disabled, a HTTP request is performed to retrieve it from Discord's API.
    ///
    /// # Errors
    ///
    /// [`ModelError::ItemMissing`] is returned if [`Self::guild_id`] is [`None`].
    pub async fn member(&self, cache_http: impl CacheHttp) -> Result<Member> {
        let guild_id = match self.guild_id {
            Some(guild_id) => guild_id,
            None => return Err(Error::Model(ModelError::ItemMissing)),
        };

        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if let Some(member) = cache.member(guild_id, self.author.id) {
                    return Ok(member);
                }
            }
        }

        cache_http.http().get_member(guild_id.0, self.author.id.0).await
    }

    /// Checks the length of a string to ensure that it is within Discord's
    /// maximum message length limit.
    ///
    /// Returns [`None`] if the message is within the limit, otherwise returns
    /// [`Some`] with an inner value of how many unicode code points the message
    /// is over.
    #[must_use]
    pub fn overflow_length(content: &str) -> Option<usize> {
        // Check if the content is over the maximum number of unicode code
        // points.
        let count = content.chars().count();

        if count > constants::MESSAGE_CODE_LIMIT {
            Some(count - constants::MESSAGE_CODE_LIMIT)
        } else {
            None
        }
    }

    /// Pins this message to its channel.
    ///
    /// **Note**: Requires the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn pin(&self, cache_http: impl CacheHttp) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.guild_id.is_some() {
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::MANAGE_MESSAGES,
                    )?;
                }
            }
        }

        self.channel_id.pin(cache_http.http(), self.id.0).await
    }

    /// React to the message with a custom [`Emoji`] or unicode character.
    ///
    /// **Note**: Requires the [Add Reactions] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have the
    /// required [permissions].
    ///
    /// [Add Reactions]: Permissions::ADD_REACTIONS
    /// [permissions]: super::permissions
    #[inline]
    pub async fn react(
        &self,
        cache_http: impl CacheHttp,
        reaction_type: impl Into<ReactionType>,
    ) -> Result<Reaction> {
        self._react(cache_http, reaction_type.into()).await
    }

    async fn _react(
        &self,
        cache_http: impl CacheHttp,
        reaction_type: ReactionType,
    ) -> Result<Reaction> {
        #[allow(unused_mut)]
        let mut user_id = None;

        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.guild_id.is_some() {
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::ADD_REACTIONS,
                    )?;
                }

                user_id = Some(cache.current_user_id());
            }
        }

        cache_http.http().create_reaction(self.channel_id.0, self.id.0, &reaction_type).await?;

        Ok(Reaction {
            channel_id: self.channel_id,
            emoji: reaction_type,
            message_id: self.id,
            user_id,
            guild_id: self.guild_id,
            member: self.member.clone(),
        })
    }

    /// Uses Discord's inline reply to a user without pinging them.
    ///
    /// User mentions are generally around 20 or 21 characters long.
    ///
    /// **Note**: Requires the [Send Messages] permission.
    ///
    /// **Note**: Message contents must be under 2000 unicode code points.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// Returns a [`ModelError::MessageTooLong`] if the content of the message
    /// is over the above limit, containing the number of unicode code points
    /// over the limit.
    ///
    /// [Send Messages]: Permissions::SEND_MESSAGES
    #[inline]
    pub async fn reply(
        &self,
        cache_http: impl CacheHttp,
        content: impl Display,
    ) -> Result<Message> {
        self._reply(cache_http, content, Some(false)).await
    }

    /// Uses Discord's inline reply to a user with a ping.
    ///
    /// **Note**: Requires the [Send Messages] permission.
    ///
    /// **Note**: Message contents must be under 2000 unicode code points.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// Returns a [`ModelError::MessageTooLong`] if the content of the message
    /// is over the above limit, containing the number of unicode code points
    /// over the limit.
    ///
    /// [Send Messages]: Permissions::SEND_MESSAGES
    #[inline]
    pub async fn reply_ping(
        &self,
        cache_http: impl CacheHttp,
        content: impl Display,
    ) -> Result<Message> {
        self._reply(cache_http, content, Some(true)).await
    }

    /// Replies to the user, mentioning them prior to the content in the form
    /// of: `@<USER_ID> YOUR_CONTENT`.
    ///
    /// User mentions are generally around 20 or 21 characters long.
    ///
    /// **Note**: Requires the [Send Messages] permission.
    ///
    /// **Note**: Message contents must be under 2000 unicode code points.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// Returns a [`ModelError::MessageTooLong`] if the content of the message
    /// is over the above limit, containing the number of unicode code points
    /// over the limit.
    ///
    /// [Send Messages]: Permissions::SEND_MESSAGES
    #[inline]
    pub async fn reply_mention(
        &self,
        cache_http: impl CacheHttp,
        content: impl Display,
    ) -> Result<Message> {
        self._reply(cache_http, format!("{} {}", self.author.mention(), content), None).await
    }

    /// `inlined` decides whether this reply is inlined and whether it pings.
    async fn _reply(
        &self,
        cache_http: impl CacheHttp,
        content: impl Display,
        inlined: Option<bool>,
    ) -> Result<Message> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.guild_id.is_some() {
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::SEND_MESSAGES,
                    )?;
                }
            }
        }

        self.channel_id
            .send_message(cache_http.http(), |builder| {
                if let Some(ping_user) = inlined {
                    builder.reference_message(self).allowed_mentions(|f| {
                        f.replied_user(ping_user)
                            // By providing allowed_mentions, Discord disabled _all_ pings by
                            // default so we need to re-enable them
                            .parse(crate::builder::ParseValue::Everyone)
                            .parse(crate::builder::ParseValue::Users)
                            .parse(crate::builder::ParseValue::Roles)
                    });
                }

                builder.content(content)
            })
            .await
    }

    /// Delete all embeds in this message
    /// **Note**: The logged in user must either be the author of the message or
    /// have the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` feature is enabled, then returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// Otherwise returns [`Error::Http`] if the current user lacks permission.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn suppress_embeds(&mut self, cache_http: impl CacheHttp) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                utils::user_has_perms_cache(
                    cache,
                    self.channel_id,
                    self.guild_id,
                    Permissions::MANAGE_MESSAGES,
                )?;

                if self.author.id != cache.current_user_id() {
                    return Err(Error::Model(ModelError::NotAuthor));
                }
            }
        }

        let mut suppress = EditMessage::default();
        suppress.suppress_embeds(true);

        let map = json::hashmap_to_json_map(suppress.0);

        *self =
            cache_http.http().edit_message(self.channel_id.0, self.id.0, &Value::from(map)).await?;

        Ok(())
    }

    /// Checks whether the message mentions passed [`UserId`].
    #[inline]
    pub fn mentions_user_id(&self, id: impl Into<UserId>) -> bool {
        let id = id.into();
        self.mentions.iter().any(|mentioned_user| mentioned_user.id.0 == id.0)
    }

    /// Checks whether the message mentions passed [`User`].
    #[inline]
    #[must_use]
    pub fn mentions_user(&self, user: &User) -> bool {
        self.mentions_user_id(user.id)
    }

    /// Checks whether the message mentions the current user.
    ///
    /// # Errors
    ///
    /// May return [`Error::Http`] if the `cache` feature is not enabled,
    /// or if the cache is otherwise unavailable.
    pub async fn mentions_me(&self, cache_http: impl CacheHttp) -> Result<bool> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                return Ok(self.mentions_user_id(cache.current_user_id()));
            }
        }

        let current_user = cache_http.http().get_current_user().await?;
        Ok(self.mentions_user_id(current_user.id))
    }

    /// Unpins the message from its channel.
    ///
    /// **Note**: Requires the [Manage Messages] permission.
    ///
    /// # Errors
    ///
    /// If the `cache` is enabled, returns a
    /// [`ModelError::InvalidPermissions`] if the current user does not have
    /// the required permissions.
    ///
    /// [Manage Messages]: Permissions::MANAGE_MESSAGES
    pub async fn unpin(&self, cache_http: impl CacheHttp) -> Result<()> {
        #[cfg(feature = "cache")]
        {
            if let Some(cache) = cache_http.cache() {
                if self.guild_id.is_some() {
                    utils::user_has_perms_cache(
                        cache,
                        self.channel_id,
                        self.guild_id,
                        Permissions::MANAGE_MESSAGES,
                    )?;
                }
            }
        }

        cache_http.http().unpin_message(self.channel_id.0, self.id.0, None).await
    }

    /// Tries to return author's nickname in the current channel's guild.
    ///
    /// Refer to [`User::nick_in()`] inside and [`None`] outside of a guild.
    #[inline]
    pub async fn author_nick(&self, cache_http: impl CacheHttp) -> Option<String> {
        self.author.nick_in(cache_http, self.guild_id?).await
    }

    /// Returns a link referencing this message. When clicked, users will jump to the message.
    /// The link will be valid for messages in either private channels or guilds.
    #[inline]
    #[must_use]
    pub fn link(&self) -> String {
        self.id.link(self.channel_id, self.guild_id)
    }

    /// Same as [`Self::link`] but tries to find the [`GuildId`]
    /// if Discord does not provide it.
    ///
    /// [`guild_id`]: Self::guild_id
    #[inline]
    pub async fn link_ensured(&self, cache_http: impl CacheHttp) -> String {
        self.id.link_ensured(cache_http, self.channel_id, self.guild_id).await
    }

    /// Await a single reaction on this message.
    #[cfg(feature = "collector")]
    pub fn await_reaction(&self, shard_messenger: impl AsRef<ShardMessenger>) -> CollectReaction {
        CollectReaction::new(shard_messenger).message_id(self.id.0)
    }

    /// Returns a stream builder which can be awaited to obtain a stream of reactions on this message.
    #[cfg(feature = "collector")]
    pub fn await_reactions(
        &self,
        shard_messenger: impl AsRef<ShardMessenger>,
    ) -> ReactionCollectorBuilder {
        ReactionCollectorBuilder::new(shard_messenger).message_id(self.id.0)
    }

    /// Await a single component interaction on this message.
    #[cfg(feature = "collector")]
    pub fn await_component_interaction(
        &self,
        shard_messenger: impl AsRef<ShardMessenger>,
    ) -> CollectComponentInteraction {
        CollectComponentInteraction::new(shard_messenger).message_id(self.id.0)
    }

    /// Returns a stream builder which can be awaited to obtain a stream of component interactions on this message.
    #[cfg(feature = "collector")]
    pub fn await_component_interactions(
        &self,
        shard_messenger: impl AsRef<ShardMessenger>,
    ) -> ComponentInteractionCollectorBuilder {
        ComponentInteractionCollectorBuilder::new(shard_messenger).message_id(self.id.0)
    }

    /// Await a single modal submit interaction on this message.
    #[cfg(feature = "collector")]
    pub fn await_modal_interaction(
        &self,
        shard_messenger: impl AsRef<ShardMessenger>,
    ) -> CollectModalInteraction {
        CollectModalInteraction::new(shard_messenger).message_id(self.id.0)
    }

    /// Returns a stream builder which can be awaited to obtain a stream of modal submit interactions on this message.
    #[cfg(feature = "collector")]
    pub fn await_modal_interactions(
        &self,
        shard_messenger: impl AsRef<ShardMessenger>,
    ) -> ModalInteractionCollectorBuilder {
        ModalInteractionCollectorBuilder::new(shard_messenger).message_id(self.id.0)
    }

    /// Retrieves the message channel's category ID if the channel has one.
    #[cfg(feature = "cache")]
    pub fn category_id(&self, cache: impl AsRef<Cache>) -> Option<ChannelId> {
        cache.as_ref().channel_category_id(self.channel_id)
    }

    pub(crate) fn check_lengths(map: &JsonMap) -> Result<()> {
        Self::check_content_length(map)?;
        Self::check_embed_length(map)?;
        Self::check_sticker_ids_length(map)?;

        Ok(())
    }

    pub(crate) fn check_content_length(map: &JsonMap) -> Result<()> {
        if let Some(Value::String(content)) = map.get("content") {
            if let Some(length_over) = Message::overflow_length(content) {
                return Err(Error::Model(ModelError::MessageTooLong(length_over)));
            }
        }

        Ok(())
    }

    pub(crate) fn check_embed_length(map: &JsonMap) -> Result<()> {
        let embeds = match map.get("embeds") {
            Some(&Value::Array(ref value)) => value,
            _ => return Ok(()),
        };

        if embeds.len() > 10 {
            return Err(Error::Model(ModelError::EmbedAmount));
        }

        for embed in embeds {
            let mut total: usize = 0;

            if let Some(&Value::Object(ref author)) = embed.get("author") {
                if let Some(&Value::Object(ref name)) = author.get("name") {
                    total += name.len();
                }
            }

            if let Some(&Value::String(ref description)) = embed.get("description") {
                total += description.len();
            }

            if let Some(&Value::Array(ref fields)) = embed.get("fields") {
                for field_as_value in fields {
                    if let Value::Object(ref field) = *field_as_value {
                        if let Some(&Value::String(ref field_name)) = field.get("name") {
                            total += field_name.len();
                        }

                        if let Some(&Value::String(ref field_value)) = field.get("value") {
                            total += field_value.len();
                        }
                    }
                }
            }

            if let Some(&Value::Object(ref footer)) = embed.get("footer") {
                if let Some(&Value::String(ref text)) = footer.get("text") {
                    total += text.len();
                }
            }

            if let Some(&Value::String(ref title)) = embed.get("title") {
                total += title.len();
            }

            if total > constants::EMBED_MAX_LENGTH {
                let overflow = total - constants::EMBED_MAX_LENGTH;
                return Err(Error::Model(ModelError::EmbedTooLarge(overflow)));
            }
        }

        Ok(())
    }

    pub(crate) fn check_sticker_ids_length(map: &JsonMap) -> Result<()> {
        if let Some(Value::Array(sticker_ids)) = map.get("sticker_ids") {
            if sticker_ids.len() > constants::STICKER_MAX_COUNT {
                return Err(Error::Model(ModelError::StickerAmount));
            }
        }

        Ok(())
    }
}

impl AsRef<MessageId> for Message {
    fn as_ref(&self) -> &MessageId {
        &self.id
    }
}

impl From<Message> for MessageId {
    /// Gets the Id of a [`Message`].
    fn from(message: Message) -> MessageId {
        message.id
    }
}

impl<'a> From<&'a Message> for MessageId {
    /// Gets the Id of a [`Message`].
    fn from(message: &Message) -> MessageId {
        message.id
    }
}

/// A representation of a reaction to a message.
///
/// Multiple of the same [reaction type] are sent into one [`MessageReaction`],
/// with an associated [`Self::count`].
///
/// [Discord docs](https://discord.com/developers/docs/resources/channel#reaction-object).
///
/// [reaction type]: ReactionType
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MessageReaction {
    /// The amount of the type of reaction that have been sent for the
    /// associated message.
    pub count: u64,
    /// Indicator of whether the current user has sent the type of reaction.
    pub me: bool,
    /// The type of reaction.
    #[serde(rename = "emoji")]
    pub reaction_type: ReactionType,
}

/// Differentiates between regular and different types of system messages.
///
/// [Discord docs](https://discord.com/developers/docs/resources/channel#message-object-message-types).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MessageType {
    /// A regular message.
    Regular = 0,
    /// An indicator that a recipient was added by the author.
    GroupRecipientAddition = 1,
    /// An indicator that a recipient was removed by the author.
    GroupRecipientRemoval = 2,
    /// An indicator that a call was started by the author.
    GroupCallCreation = 3,
    /// An indicator that the group name was modified by the author.
    GroupNameUpdate = 4,
    /// An indicator that the group icon was modified by the author.
    GroupIconUpdate = 5,
    /// An indicator that a message was pinned by the author.
    PinsAdd = 6,
    /// An indicator that a member joined the guild.
    MemberJoin = 7,
    /// An indicator that someone has boosted the guild.
    NitroBoost = 8,
    /// An indicator that the guild has reached nitro tier 1
    NitroTier1 = 9,
    /// An indicator that the guild has reached nitro tier 2
    NitroTier2 = 10,
    /// An indicator that the guild has reached nitro tier 3
    NitroTier3 = 11,
    /// An indicator that the channel is now following a news channel
    ChannelFollowAdd = 12,
    /// An indicator that the guild is disqualified for Discovery Feature
    GuildDiscoveryDisqualified = 14,
    /// An indicator that the guild is requalified for Discovery Feature
    GuildDiscoveryRequalified = 15,
    /// The first warning before guild discovery removal.
    GuildDiscoveryGracePeriodInitialWarning = 16,
    /// The last warning before guild discovery removal.
    GuildDiscoveryGracePeriodFinalWarning = 17,
    /// Message sent to inform users that a thread was created.
    ThreadCreated = 18,
    /// A message reply.
    InlineReply = 19,
    /// A slash command.
    ChatInputCommand = 20,
    /// A thread start message.
    ThreadStarterMessage = 21,
    /// Server setup tips.
    GuildInviteReminder = 22,
    /// A context menu command.
    ContextMenuCommand = 23,
    /// A message from an auto moderation action.
    AutoModerationAction = 24,
    /// An indicator that the message is of unknown type.
    Unknown = !0,
}

enum_number!(MessageType {
    Regular,
    GroupRecipientAddition,
    GroupRecipientRemoval,
    GroupCallCreation,
    GroupNameUpdate,
    GroupIconUpdate,
    PinsAdd,
    MemberJoin,
    NitroBoost,
    NitroTier1,
    NitroTier2,
    NitroTier3,
    ChannelFollowAdd,
    GuildDiscoveryDisqualified,
    GuildDiscoveryRequalified,
    GuildDiscoveryGracePeriodInitialWarning,
    GuildDiscoveryGracePeriodFinalWarning
    ThreadCreated,
    InlineReply,
    ChatInputCommand,
    ThreadStarterMessage,
    GuildInviteReminder,
    ContextMenuCommand,
    AutoModerationAction,
});

/// [Discord docs](https://discord.com/developers/docs/resources/channel#message-object-message-activity-types).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MessageActivityKind {
    #[allow(clippy::upper_case_acronyms)]
    JOIN = 1,
    #[allow(clippy::upper_case_acronyms)]
    SPECTATE = 2,
    #[allow(clippy::upper_case_acronyms)]
    LISTEN = 3,
    #[allow(non_camel_case_types)]
    JOIN_REQUEST = 5,
    Unknown = !0,
}

enum_number!(MessageActivityKind {
    JOIN,
    SPECTATE,
    LISTEN,
    JOIN_REQUEST
});

/// Rich Presence application information.
///
/// [Discord docs](https://discord.com/developers/docs/resources/application#application-object).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MessageApplication {
    /// ID of the application.
    pub id: ApplicationId,
    /// ID of the embed's image asset.
    pub cover_image: Option<String>,
    /// Application's description.
    pub description: String,
    /// ID of the application's icon.
    pub icon: Option<String>,
    /// Name of the application.
    pub name: String,
}

/// Rich Presence activity information.
///
/// [Discord docs](https://discord.com/developers/docs/resources/channel#message-object-message-activity-structure).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MessageActivity {
    /// Kind of message activity.
    #[serde(rename = "type")]
    pub kind: MessageActivityKind,
    /// `party_id` from a Rich Presence event.
    pub party_id: Option<String>,
}

/// Reference data sent with crossposted messages.
///
/// [Discord docs](https://discord.com/developers/docs/resources/channel#message-reference-object-message-reference-structure).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MessageReference {
    /// ID of the originating message.
    pub message_id: Option<MessageId>,
    /// ID of the originating message's channel.
    pub channel_id: ChannelId,
    /// ID of the originating message's guild.
    pub guild_id: Option<GuildId>,
}

impl From<&Message> for MessageReference {
    fn from(m: &Message) -> Self {
        Self {
            message_id: Some(m.id),
            channel_id: m.channel_id,
            guild_id: m.guild_id,
        }
    }
}

impl From<(ChannelId, MessageId)> for MessageReference {
    fn from(pair: (ChannelId, MessageId)) -> Self {
        Self {
            message_id: Some(pair.1),
            channel_id: pair.0,
            guild_id: None,
        }
    }
}

/// [Discord docs](https://discord.com/developers/docs/resources/channel#channel-mention-object).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChannelMention {
    /// ID of the channel.
    pub id: ChannelId,
    /// ID of the guild containing the channel.
    pub guild_id: GuildId,
    /// The kind of channel
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// The name of the channel
    pub name: String,
}

bitflags! {
    /// Describes extra features of the message.
    ///
    /// [Discord docs](https://discord.com/developers/docs/resources/channel#message-object-message-flags).
    #[derive(Default)]
    pub struct MessageFlags: u64 {
        /// This message has been published to subscribed channels (via Channel Following).
        const CROSSPOSTED = 1 << 0;
        /// This message originated from a message in another channel (via Channel Following).
        const IS_CROSSPOST = 1 << 1;
        /// Do not include any embeds when serializing this message.
        const SUPPRESS_EMBEDS = 1 << 2;
        /// The source message for this crosspost has been deleted (via Channel Following).
        const SOURCE_MESSAGE_DELETED = 1 << 3;
        /// This message came from the urgent message system.
        const URGENT = 1 << 4;
        /// This message has an associated thread, with the same id as the message.
        const HAS_THREAD = 1 << 5;
        /// This message is only visible to the user who invoked the Interaction.
        const EPHEMERAL = 1 << 6;
        /// This message is an Interaction Response and the bot is "thinking".
        const LOADING = 1 << 7;
        /// This message failed to mention some roles and add their members to the thread.
        const FAILED_TO_MENTION_SOME_ROLES_IN_THREAD = 1 << 8;
    }
}

#[cfg(feature = "model")]
impl MessageId {
    /// Returns a link referencing this message. When clicked, users will jump to the message.
    /// The link will be valid for messages in either private channels or guilds.
    #[must_use]
    pub fn link(&self, channel_id: ChannelId, guild_id: Option<GuildId>) -> String {
        if let Some(guild_id) = guild_id {
            format!("https://discord.com/channels/{}/{}/{}", guild_id, channel_id, self)
        } else {
            format!("https://discord.com/channels/@me/{}/{}", channel_id, self)
        }
    }

    /// Same as [`Self::link`] but tries to find the [`GuildId`]
    /// if it is not provided.
    pub async fn link_ensured(
        &self,
        cache_http: impl CacheHttp,
        channel_id: ChannelId,
        mut guild_id: Option<GuildId>,
    ) -> String {
        if guild_id.is_none() {
            let found_channel = channel_id.to_channel(cache_http).await;

            if let Ok(channel) = found_channel {
                if let Some(c) = channel.guild() {
                    guild_id = Some(c.guild_id);
                }
            }
        }

        self.link(channel_id, guild_id)
    }
}

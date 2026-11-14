use chrono::{DateTime, Local};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct User {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

impl User {
    pub fn display_name(&self) -> String {
        self.global_name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if self.username.is_empty() {
                    "Desconhecido".to_string()
                } else {
                    self.username.clone()
                }
            })
    }

    pub fn handle(&self) -> String {
        match &self.discriminator {
            Some(d) if d != "0" && !d.is_empty() => format!("{}#{}", self.username, d),
            _ => format!("@{}", self.username),
        }
    }

    /// URL do avatar no CDN do Discord. Avatares animados usam GIF.
    pub fn avatar_url(&self) -> Option<String> {
        let hash = self.avatar.as_deref().filter(|hash| !hash.is_empty())?;
        let extension = if hash.starts_with("a_") { "gif" } else { "png" };
        Some(format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}?size=64",
            self.id, hash, extension
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Guild {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub member_count: Option<u32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub roles: Vec<Role>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Role {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub hoist: bool,
}

impl Role {
    /// Cor RGB da role, se ela tiver uma cor configurada (0 = padrão).
    pub fn rgb(&self) -> Option<(u8, u8, u8)> {
        if self.color == 0 {
            return None;
        }
        Some((
            ((self.color >> 16) & 0xff) as u8,
            ((self.color >> 8) & 0xff) as u8,
            (self.color & 0xff) as u8,
        ))
    }
}

/// Membro de um servidor, como devolvido por `/guilds/{id}/members`.
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Member {
    #[serde(default)]
    pub user: User,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub joined_at: Option<String>,
}

impl Member {
    pub fn display_name(&self) -> String {
        self.nick
            .clone()
            .filter(|nick| !nick.trim().is_empty())
            .unwrap_or_else(|| self.user.display_name())
    }

    pub fn handle(&self) -> String {
        self.user.handle()
    }

    pub fn id(&self) -> &str {
        &self.user.id
    }

    /// Cor da role mais alta (com cor) que o membro possui.
    pub fn role_color(&self, roles: &[Role]) -> Option<(u8, u8, u8)> {
        self.roles
            .iter()
            .filter_map(|id| roles.iter().find(|role| &role.id == id))
            .filter_map(|role| role.rgb().map(|rgb| (role.position, rgb)))
            .max_by_key(|(position, _)| *position)
            .map(|(_, rgb)| rgb)
    }
}

impl Guild {
    pub fn icon_url(&self) -> Option<String> {
        let hash = self.icon.as_deref().filter(|hash| !hash.is_empty())?;
        let extension = if hash.starts_with("a_") { "gif" } else { "png" };
        Some(format!(
            "https://cdn.discordapp.com/icons/{}/{}.{}?size=64",
            self.id, hash, extension
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: u8,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub recipients: Vec<User>,
    #[serde(default)]
    pub guild_id: Option<String>,
}

impl Channel {
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            if !name.is_empty() {
                return name.clone();
            }
        }

        let names: Vec<String> = self
            .recipients
            .iter()
            .map(|user| user.display_name())
            .collect();

        if names.is_empty() {
            "Mensagem direta".to_string()
        } else {
            names.join(", ")
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, 0 | 5 | 15 | 16)
    }

    pub fn is_voice(&self) -> bool {
        matches!(self.kind, 2 | 13)
    }

    pub fn is_category(&self) -> bool {
        self.kind == 4
    }

    pub fn is_dm(&self) -> bool {
        matches!(self.kind, 1 | 3)
    }

    pub fn prefix(&self) -> &'static str {
        if self.is_dm() {
            "@"
        } else {
            "#"
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Attachment {
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub width: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub height: Option<u32>,
}

impl Attachment {
    pub fn is_image(&self) -> bool {
        if let Some(content_type) = &self.content_type {
            if content_type.starts_with("image/") {
                return true;
            }
        }
        let lower = self.filename.to_ascii_lowercase();
        [".png", ".jpg", ".jpeg", ".gif", ".webp"]
            .iter()
            .any(|extension| lower.ends_with(extension))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedImage {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedAuthor {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedField {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedFooter {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedProvider {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// Embeds cobrem desde previews de link até GIFs (Tenor/Klipy).
/// Alguns provedores só mandam `video` (MP4) e `thumbnail`; nesse caso
/// mostramos a miniatura, que é a única mídia que o cliente sabe decodificar.
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Embed {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub image: Option<EmbedImage>,
    #[serde(default)]
    pub thumbnail: Option<EmbedImage>,
    #[serde(default)]
    pub video: Option<EmbedVideo>,
    #[serde(default)]
    pub footer: Option<EmbedFooter>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
    #[serde(default)]
    pub provider: Option<EmbedProvider>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmbedVideo {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

impl Embed {
    /// Melhor mídia estática disponível para desenhar.
    pub fn best_image(&self) -> Option<&EmbedImage> {
        self.image
            .as_ref()
            .filter(|image| !image.url.is_empty())
            .or_else(|| {
                self.thumbnail
                    .as_ref()
                    .filter(|image| !image.url.is_empty())
            })
    }

    /// Um embed sem nenhum conteúdo visível não deve ocupar espaço.
    pub fn has_content(&self) -> bool {
        self.title
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
            || self
                .description
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
            || self
                .author
                .as_ref()
                .is_some_and(|author| !author.name.trim().is_empty())
            || !self.fields.is_empty()
            || self.best_image().is_some()
            || self
                .footer
                .as_ref()
                .is_some_and(|footer| !footer.text.trim().is_empty())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub author: User,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub pending: bool,
}

impl Message {
    pub fn time_label(&self) -> String {
        format_time(&self.timestamp)
    }
}

pub fn format_time(timestamp: &str) -> String {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

pub fn initials(name: &str) -> String {
    let mut result = String::new();
    for word in name.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            result.extend(c.to_uppercase());
        }
    }
    if result.is_empty() {
        "?".to_string()
    } else {
        result
    }
}

/// Remove as construções cruas do Discord (<@123>, <#456>, <:emoji:789>)
/// e deixa o texto legível para quem está lendo.
pub fn clean_content(input: &str) -> String {
    // Seletores de variação (emoji vs. texto) não têm largura e algumas
    // fontes não os possuem. Removê-los evita quadradinhos no lugar do nada.
    let input = if input.contains(['\u{FE0F}', '\u{FE0E}']) {
        std::borrow::Cow::Owned(
            input
                .chars()
                .filter(|c| !matches!(c, '\u{FE0F}' | '\u{FE0E}'))
                .collect(),
        )
    } else {
        std::borrow::Cow::Borrowed(input)
    };

    let mut output = String::with_capacity(input.len());
    let mut rest = input.as_ref();

    while let Some(start) = rest.find('<') {
        output.push_str(&rest[..start]);
        let tail = &rest[start..];

        let Some(end) = tail.find('>') else {
            output.push_str(tail);
            return output;
        };

        let token = &tail[1..end];
        let consumed = &tail[end + 1..];

        if let Some(id) = token.strip_prefix("@&") {
            if is_snowflake(id) {
                output.push_str("@cargo");
                rest = consumed;
                continue;
            }
        }
        if let Some(id) = token.strip_prefix("@!").or_else(|| token.strip_prefix('@')) {
            if is_snowflake(id) {
                output.push_str("@usuario");
                rest = consumed;
                continue;
            }
        }
        if let Some(id) = token.strip_prefix('#') {
            if is_snowflake(id) {
                output.push_str("#canal");
                rest = consumed;
                continue;
            }
        }
        if let Some(name) = custom_emoji_name(token) {
            output.push(':');
            output.push_str(name);
            output.push(':');
            rest = consumed;
            continue;
        }
        if token.starts_with("t:") {
            // Timestamp dinâmico do Discord: mantém algo legível e curto.
            output.push_str("[data]");
            rest = consumed;
            continue;
        }

        output.push('<');
        rest = &tail[1..];
    }

    output.push_str(rest);
    output
}

fn is_snowflake(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
}

fn custom_emoji_name(token: &str) -> Option<&str> {
    let body = token.strip_prefix('a').unwrap_or(token);
    let body = body.strip_prefix(':')?;
    let (name, id) = body.rsplit_once(':')?;
    if name.is_empty() || !is_snowflake(id) {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_mentions_and_emoji() {
        assert_eq!(
            clean_content("oi <@123> veja <#456> <:rust:789> <@&555>"),
            "oi @usuario veja #canal :rust: @cargo"
        );
    }

    #[test]
    fn initials_use_two_words() {
        assert_eq!(initials("Discord Phoenix"), "DP");
        assert_eq!(initials("ana"), "A");
        assert_eq!(initials(""), "?");
    }

    #[test]
    fn avatar_urls_use_gif_for_animated_hashes() {
        let user = User {
            id: "123".to_string(),
            avatar: Some("a_abc".to_string()),
            ..Default::default()
        };
        assert_eq!(
            user.avatar_url().as_deref(),
            Some("https://cdn.discordapp.com/avatars/123/a_abc.gif?size=64")
        );

        let guild = Guild {
            id: "42".to_string(),
            icon: Some("def".to_string()),
            ..Default::default()
        };
        assert_eq!(
            guild.icon_url().as_deref(),
            Some("https://cdn.discordapp.com/icons/42/def.png?size=64")
        );
    }

    #[test]
    fn detects_image_attachments() {
        let by_type = Attachment {
            content_type: Some("image/png".to_string()),
            ..Default::default()
        };
        let by_name = Attachment {
            filename: "foto.JPEG".to_string(),
            ..Default::default()
        };
        let other = Attachment {
            filename: "documento.pdf".to_string(),
            content_type: Some("application/pdf".to_string()),
            ..Default::default()
        };
        assert!(by_type.is_image());
        assert!(by_name.is_image());
        assert!(!other.is_image());
    }

    #[test]
    fn removes_variation_selectors() {
        assert_eq!(clean_content("coração ❤️ pronto"), "coração ❤ pronto");
    }

    #[test]
    fn member_prefers_nickname() {
        let member: Member = serde_json::from_str(
            r#"{"nick":"Apelido","user":{"id":"1","username":"ana","global_name":"Ana"}}"#,
        )
        .unwrap();
        assert_eq!(member.display_name(), "Apelido");
        assert_eq!(member.handle(), "@ana");
    }

    #[test]
    fn embed_falls_back_to_thumbnail() {
        let embed: Embed = serde_json::from_str(
            r#"{
                "type": "gifv",
                "url": "https://klipy.com/gifs/exemplo",
                "video": {"url": "https://static.klipy.com/exemplo.mp4"},
                "thumbnail": {"url": "https://static.klipy.com/exemplo.webp"}
            }"#,
        )
        .unwrap();
        assert!(embed.has_content());
        assert_eq!(
            embed.best_image().map(|image| image.url.as_str()),
            Some("https://static.klipy.com/exemplo.webp")
        );
    }
}

use base64::Engine;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::time::Duration;

use crate::model::{Channel, Guild, Member, Message, User};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Cabeçalho `X-Super-Properties` usado tanto na API REST quanto no login.
/// O Discord usa esses dados para identificar o cliente; manter um valor
/// estável evita que cada requisição pareça um dispositivo diferente.
pub fn super_properties() -> String {
    static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let props = json!({
                "os": "Windows",
                "browser": "Chrome",
                "device": "",
                "system_locale": "pt-BR",
                "browser_user_agent": USER_AGENT,
                "browser_version": "131.0.0.0",
                "os_version": "10",
                "referrer": "",
                "referring_domain": "",
                "referrer_current": "",
                "referring_domain_current": "",
                "release_channel": "stable",
                "client_build_number": 350000,
                "client_event_source": null
            });
            base64::engine::general_purpose::STANDARD.encode(props.to_string().as_bytes())
        })
        .clone()
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn friendly(&self) -> String {
        match self.status {
            0 => self.message.clone(),
            401 => "Token inválido ou expirado (401). Confira o token e o tipo de conta.".to_string(),
            403 => format!("Acesso negado (403). {}", self.message),
            404 => format!("Não encontrado (404). {}", self.message),
            429 => "Limite de requisições do Discord atingido. Tente de novo em instantes."
                .to_string(),
            _ => format!("Erro {}: {}", self.status, self.message),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.friendly())
    }
}

impl std::error::Error for ApiError {}

pub struct Api {
    client: Client,
    token: String,
    is_bot: bool,
    super_properties: String,
}

impl Api {
    pub fn new(token: String, is_bot: bool) -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("falha ao criar cliente HTTP");

        let super_properties = super_properties();

        Self {
            client,
            token,
            is_bot,
            super_properties,
        }
    }

    fn authorization(&self) -> String {
        if self.is_bot {
            format!("Bot {}", self.token)
        } else {
            self.token.clone()
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("https://discord.com/api/v10{path}")
        };

        self.client
            .request(method, url)
            .header("Authorization", self.authorization())
            .header("X-Super-Properties", &self.super_properties)
            .header("X-Discord-Locale", "pt-BR")
            .header("Accept", "application/json")
    }

    async fn json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T, ApiError> {
        let mut builder = self.request(method, path);
        if let Some(body) = &body {
            builder = builder.json(body);
        }

        let mut attempts = 0u8;
        loop {
            let request = builder.try_clone().ok_or_else(|| ApiError {
                status: 0,
                message: "Não foi possível preparar a requisição.".to_string(),
            })?;

            let response = request.send().await.map_err(|err| ApiError {
                status: 0,
                message: format!("Falha de rede: {err}"),
            })?;

            let status = response.status();

            if status == StatusCode::TOO_MANY_REQUESTS && attempts < 3 {
                let wait = retry_after_seconds(response).await;
                tokio::time::sleep(Duration::from_secs_f32(wait.clamp(0.2, 10.0))).await;
                attempts += 1;
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(build_error(status, &body));
            }

            let body = response.text().await.map_err(|err| ApiError {
                status: 0,
                message: format!("Falha ao ler resposta: {err}"),
            })?;

            if body.trim().is_empty() {
                return Err(ApiError {
                    status: 0,
                    message: "O Discord devolveu uma resposta vazia.".to_string(),
                });
            }

            return serde_json::from_str(&body).map_err(|err| ApiError {
                status: 0,
                message: format!("Resposta inesperada do Discord: {err}"),
            });
        }
    }

    async fn execute(&self, method: Method, path: &str, body: Option<Value>) -> Result<(), ApiError> {
        let mut builder = self.request(method, path);
        if let Some(body) = &body {
            builder = builder.json(body);
        }

        let mut attempts = 0u8;
        loop {
            let request = builder.try_clone().ok_or_else(|| ApiError {
                status: 0,
                message: "Não foi possível preparar a requisição.".to_string(),
            })?;

            let response = request.send().await.map_err(|err| ApiError {
                status: 0,
                message: format!("Falha de rede: {err}"),
            })?;

            let status = response.status();

            if status == StatusCode::TOO_MANY_REQUESTS && attempts < 3 {
                let wait = retry_after_seconds(response).await;
                tokio::time::sleep(Duration::from_secs_f32(wait.clamp(0.2, 10.0))).await;
                attempts += 1;
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(build_error(status, &body));
            }

            return Ok(());
        }
    }

    pub async fn me(&self) -> Result<User, ApiError> {
        self.json(Method::GET, "/users/@me", None).await
    }

    pub async fn guilds(&self) -> Result<Vec<Guild>, ApiError> {
        self.json(Method::GET, "/users/@me/guilds", None).await
    }

    pub async fn guild_channels(&self, guild_id: &str) -> Result<Vec<Channel>, ApiError> {
        self.json(
            Method::GET,
            &format!("/guilds/{guild_id}/channels"),
            None,
        )
        .await
    }

    pub async fn guild(&self, guild_id: &str) -> Result<Guild, ApiError> {
        self.json(Method::GET, &format!("/guilds/{guild_id}"), None)
            .await
    }

    pub async fn guild_members(
        &self,
        guild_id: &str,
        limit: u16,
    ) -> Result<Vec<Member>, ApiError> {
        self.json(
            Method::GET,
            &format!("/guilds/{guild_id}/members?limit={limit}"),
            None,
        )
        .await
    }

    pub async fn modify_guild_name(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Guild, ApiError> {
        self.json(
            Method::PATCH,
            &format!("/guilds/{guild_id}"),
            Some(json!({ "name": name })),
        )
        .await
    }

    pub async fn direct_messages(&self) -> Result<Vec<Channel>, ApiError> {
        let channels: Vec<Channel> = self
            .json(Method::GET, "/users/@me/channels", None)
            .await?;
        Ok(channels
            .into_iter()
            .filter(|channel| channel.is_dm())
            .collect())
    }

    pub async fn messages(
        &self,
        channel_id: &str,
        limit: u8,
    ) -> Result<Vec<Message>, ApiError> {
        let mut messages: Vec<Message> = self
            .json(
                Method::GET,
                &format!("/channels/{channel_id}/messages?limit={limit}"),
                None,
            )
            .await?;
        messages.reverse();
        Ok(messages)
    }

    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<Message, ApiError> {
        self.json(
            Method::POST,
            &format!("/channels/{channel_id}/messages"),
            Some(json!({ "content": content })),
        )
        .await
    }

    pub async fn edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<Message, ApiError> {
        self.json(
            Method::PATCH,
            &format!("/channels/{channel_id}/messages/{message_id}"),
            Some(json!({ "content": content })),
        )
        .await
    }

    pub async fn delete_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), ApiError> {
        self.execute(
            Method::DELETE,
            &format!("/channels/{channel_id}/messages/{message_id}"),
            None,
        )
        .await
    }

    pub async fn send_typing(&self, channel_id: &str) -> Result<(), ApiError> {
        self.execute(
            Method::POST,
            &format!("/channels/{channel_id}/typing"),
            None,
        )
        .await
    }
}

fn build_error(status: StatusCode, body: &str) -> ApiError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(|message| message.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| {
            let cleaned = body.trim();
            if cleaned.is_empty() {
                "sem detalhes".to_string()
            } else {
                cleaned.chars().take(240).collect()
            }
        });

    ApiError {
        status: status.as_u16(),
        message,
    }
}

async fn retry_after_seconds(response: Response) -> f32 {
    let header_wait = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<f32>().ok());

    let body_wait = response
        .text()
        .await
        .ok()
        .and_then(|body| serde_json::from_str::<Value>(&body).ok())
        .and_then(|value| {
            value
                .get("retry_after")
                .and_then(|retry| retry.as_f64())
                .map(|retry| retry as f32)
        });

    body_wait.or(header_wait).unwrap_or(2.0)
}

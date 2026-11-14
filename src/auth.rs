//! Login de conta: e-mail/senha (com 2FA) e QR Code (remote auth).
//!
//! O Discord protege o login por senha com captcha em boa parte das redes. O
//! fluxo abaixo fala o mesmo protocolo do cliente oficial e devolve um erro
//! claro quando o captcha é exigido. O login por QR Code usa o gateway de
//! autenticação remota do Discord: o usuário escaneia o código com o app do
//! celular e confirma a entrada, sem digitar senha no Phoenix.

use std::time::Duration;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rand::rngs::OsRng;
use reqwest::{Client, Method, RequestBuilder};
use rsa::pkcs8::EncodePublicKey;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::api::{super_properties, USER_AGENT};

const API_BASE: &str = "https://discord.com/api/v9";
const REMOTE_AUTH_GATEWAY: &str = "wss://remote-auth-gateway.discord.gg/?v=2";
const REMOTE_AUTH_ORIGIN: &str = "https://discord.com";

pub const CAPTCHA_HELP: &str =
    "O Discord pediu captcha para entrar com senha neste computador. \
     Use a aba QR Code: escanear o código com o app do celular entra sem captcha.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordOutcome {
    Token(String),
    Mfa { ticket: String },
    Captcha { sitekey: Option<String>, service: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteAuthEvent {
    Qr { url: String, expires_in: Duration },
    Scanned { username: String, discriminator: String },
}

#[derive(Debug)]
pub enum RemoteAuthEnd {
    Token(String),
    Expired,
    Cancelled,
    Failed(String),
}

pub fn http_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("falha ao criar cliente HTTP de autenticação")
}

fn auth_request(client: &Client, method: Method, path: &str) -> RequestBuilder {
    client
        .request(method, format!("{API_BASE}{path}"))
        .header("Accept", "application/json")
        .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Content-Type", "application/json")
        .header("X-Super-Properties", super_properties())
        .header("X-Discord-Locale", "pt-BR")
        .header("X-Discord-Timezone", "America/Sao_Paulo")
        .header("X-Debug-Options", "bugReporterEnabled")
        .header("Origin", REMOTE_AUTH_ORIGIN)
        .header("Referer", "https://discord.com/login")
}

/// Entra com e-mail/telefone e senha. Pode devolver um token, um pedido de
/// segundo fator (TOTP) ou um pedido de captcha.
pub async fn password_login(
    client: &Client,
    login: &str,
    password: &str,
) -> Result<PasswordOutcome, String> {
    let body = json!({
        "login": login,
        "password": password,
        "undelete": false,
        "captcha_key": Value::Null,
        "login_source": Value::Null,
        "gift_code_sku_id": Value::Null,
    });

    let response = auth_request(client, Method::POST, "/auth/login")
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Falha de rede ao entrar: {err}"))?;

    let status = response.status();
    let raw = response.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

    if let Some(token) = token_from(&value) {
        return Ok(PasswordOutcome::Token(token));
    }

    if value.get("mfa").and_then(Value::as_bool) == Some(true) {
        if let Some(ticket) = value.get("ticket").and_then(Value::as_str) {
            return Ok(PasswordOutcome::Mfa {
                ticket: ticket.to_string(),
            });
        }
    }

    if value.get("captcha_key").is_some() || value.get("captcha_sitekey").is_some() {
        return Ok(PasswordOutcome::Captcha {
            sitekey: string_field(&value, "captcha_sitekey"),
            service: string_field(&value, "captcha_service"),
        });
    }

    Err(describe_error(status.as_u16(), &value, &raw))
}

/// Conclui o login com o código do app autenticador.
pub async fn mfa_login(client: &Client, ticket: &str, code: &str) -> Result<String, String> {
    let body = json!({
        "code": code,
        "ticket": ticket,
        "login_source": Value::Null,
        "gift_code_sku_id": Value::Null,
        "captcha_key": Value::Null,
    });

    let response = auth_request(client, Method::POST, "/auth/mfa/totp")
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Falha de rede ao verificar o código: {err}"))?;

    let status = response.status();
    let raw = response.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

    if let Some(token) = token_from(&value) {
        return Ok(token);
    }

    if value.get("captcha_key").is_some() || value.get("captcha_sitekey").is_some() {
        return Err(CAPTCHA_HELP.to_string());
    }

    Err(describe_error(status.as_u16(), &value, &raw))
}

fn token_from(value: &Value) -> Option<String> {
    value
        .get("token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|token| !token.is_empty())
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.is_empty())
}

fn describe_error(status: u16, value: &Value, raw: &str) -> String {
    let detail = value
        .get("errors")
        .and_then(|errors| errors.as_object())
        .and_then(|errors| errors.values().next())
        .and_then(|field| field.get("_errors"))
        .and_then(|list| list.as_array())
        .and_then(|list| list.first())
        .and_then(|first| first.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if let Some(detail) = detail {
        return format!("Não foi possível entrar: {detail}");
    }

    match status {
        401 => "E-mail/telefone ou senha incorretos.".to_string(),
        403 => "O Discord bloqueou este login (403). Tente pelo QR Code.".to_string(),
        429 => "Muitas tentativas de login. Aguarde alguns minutos e tente de novo.".to_string(),
        _ if message.eq_ignore_ascii_case("invalid form body") => {
            "Confira o e-mail/telefone e a senha digitados.".to_string()
        }
        _ if !message.is_empty() => format!("Não foi possível entrar: {message}"),
        _ => {
            let cleaned: String = raw.trim().chars().take(200).collect();
            if cleaned.is_empty() {
                format!("Não foi possível entrar (erro {status}).")
            } else {
                format!("Não foi possível entrar (erro {status}): {cleaned}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Remote auth (QR Code)
// ---------------------------------------------------------------------------

struct RemoteKey {
    private: RsaPrivateKey,
    public_der: Vec<u8>,
}

impl RemoteKey {
    fn generate() -> Result<Self, String> {
        let private = RsaPrivateKey::new(&mut OsRng, 2048)
            .map_err(|err| format!("Falha ao gerar a chave do QR Code: {err}"))?;
        let public = RsaPublicKey::from(&private);
        let public_der = public
            .to_public_key_der()
            .map_err(|err| format!("Falha ao exportar a chave pública: {err}"))?
            .as_bytes()
            .to_vec();
        Ok(Self { private, public_der })
    }

    fn encoded_public_key(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.public_der)
    }

    fn decrypt(&self, encoded: &str) -> Result<Vec<u8>, String> {
        let cipher = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| format!("Pacote criptografado inválido: {err}"))?;
        let padding = Oaep::new::<Sha256>();
        self.private
            .decrypt(padding, &cipher)
            .map_err(|err| format!("Falha ao descriptografar o pacote do Discord: {err}"))
    }
}

pub fn qr_url(fingerprint: &str) -> String {
    format!("{REMOTE_AUTH_ORIGIN}/ra/{fingerprint}")
}

pub fn nonce_proof(nonce: &[u8]) -> String {
    let digest = Sha256::digest(nonce);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

pub fn parse_user_payload(payload: &str) -> Option<(String, String)> {
    let mut parts = payload.splitn(4, ':');
    let _id = parts.next()?;
    let discriminator = parts.next()?.to_string();
    let _avatar = parts.next()?;
    let username = parts.next()?.trim().to_string();
    if username.is_empty() {
        return None;
    }
    Some((username, discriminator))
}

/// Roda uma sessão completa de login por QR Code.
///
/// Eventos intermediários (QR gerado e usuário que escaneou) são enviados por
/// `notify`; o token só é devolvido quando o celular confirma o login.
pub async fn remote_auth_session<F>(client: &Client, notify: F) -> RemoteAuthEnd
where
    F: Fn(RemoteAuthEvent),
{
    match remote_auth_inner(client, &notify).await {
        Ok(token) => RemoteAuthEnd::Token(token),
        Err(end) => end,
    }
}

async fn remote_auth_inner<F>(
    client: &Client,
    notify: &F,
) -> Result<String, RemoteAuthEnd>
where
    F: Fn(RemoteAuthEvent),
{
    let mut request = REMOTE_AUTH_GATEWAY
        .into_client_request()
        .map_err(|err| RemoteAuthEnd::Failed(format!("Endereço do login por QR inválido: {err}")))?;
    request.headers_mut().insert(
        "Origin",
        HeaderValue::from_static(REMOTE_AUTH_ORIGIN),
    );
    request.headers_mut().insert(
        "User-Agent",
        HeaderValue::from_static(USER_AGENT),
    );

    let (socket, _) = tokio::time::timeout(Duration::from_secs(20), tokio_tungstenite::connect_async(request))
        .await
        .map_err(|_| RemoteAuthEnd::Failed("O login por QR demorou para conectar. Tente de novo.".into()))?
        .map_err(|err| RemoteAuthEnd::Failed(format!("Não foi possível abrir o login por QR: {err}")))?;

    let (mut sink, mut stream) = socket.split();

    let key = RemoteKey::generate().map_err(RemoteAuthEnd::Failed)?;

    let mut heartbeat = Duration::from_secs(30);
    let mut next_heartbeat = tokio::time::Instant::now() + heartbeat;
    let mut heartbeat_started = false;
    let mut expires_at: Option<tokio::time::Instant> = None;
    let mut qr_shown = false;

    loop {
        let now = tokio::time::Instant::now();

        if let Some(deadline) = expires_at {
            if now >= deadline {
                return Err(RemoteAuthEnd::Expired);
            }
        }

        if heartbeat_started && now >= next_heartbeat {
            let payload = json!({ "op": "heartbeat" }).to_string();
            if sink.send(WsMessage::Text(payload)).await.is_err() {
                return Err(RemoteAuthEnd::Failed("O login por QR caiu. Tente de novo.".into()));
            }
            next_heartbeat = now + heartbeat;
        }

        let wait = if heartbeat_started {
            next_heartbeat.saturating_duration_since(now)
        } else {
            Duration::from_secs(30)
        };
        let wait = expires_at
            .map(|deadline| deadline.saturating_duration_since(now).min(wait))
            .unwrap_or(wait)
            .clamp(Duration::from_millis(50), Duration::from_secs(1));

        let message = match tokio::time::timeout(wait, stream.next()).await {
            Ok(Some(Ok(message))) => message,
            Ok(Some(Err(err))) => {
                return Err(RemoteAuthEnd::Failed(format!(
                    "Falha na conexão do login por QR: {err}"
                )))
            }
            Ok(None) => {
                if qr_shown {
                    return Err(RemoteAuthEnd::Expired);
                }
                return Err(RemoteAuthEnd::Failed(
                    "O Discord encerrou a conexão do login por QR.".into(),
                ))
            }
            Err(_) => continue,
        };

        let text = match message {
            WsMessage::Text(text) => text,
            WsMessage::Ping(payload) => {
                let _ = sink.send(WsMessage::Pong(payload)).await;
                continue;
            }
            WsMessage::Close(_) => {
                if qr_shown {
                    return Err(RemoteAuthEnd::Expired);
                }
                return Err(RemoteAuthEnd::Failed(
                    "O Discord encerrou a conexão do login por QR.".into(),
                ))
            }
            _ => continue,
        };

        let packet: Value = match serde_json::from_str(&text) {
            Ok(packet) => packet,
            Err(_) => continue,
        };

        let op = packet.get("op").and_then(Value::as_str).unwrap_or_default();

        match op {
            "hello" => {
                if let Some(interval) = packet.get("heartbeat_interval").and_then(Value::as_u64) {
                    heartbeat = Duration::from_millis(interval.max(1_000));
                }
                if let Some(timeout) = packet.get("timeout_ms").and_then(Value::as_u64) {
                    expires_at = Some(
                        tokio::time::Instant::now() + Duration::from_millis(timeout.max(30_000)),
                    );
                }
                heartbeat_started = true;
                next_heartbeat = tokio::time::Instant::now() + heartbeat;

                let payload = json!({
                    "op": "init",
                    "encoded_public_key": key.encoded_public_key(),
                })
                .to_string();
                if sink.send(WsMessage::Text(payload)).await.is_err() {
                    return Err(RemoteAuthEnd::Failed(
                        "O login por QR caiu ao enviar a chave.".into(),
                    ));
                }
            }

            "nonce_proof" => {
                let encrypted = packet
                    .get("encrypted_nonce")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let nonce = key.decrypt(encrypted).map_err(RemoteAuthEnd::Failed)?;
                let payload = json!({ "op": "nonce_proof", "proof": nonce_proof(&nonce) }).to_string();
                if sink.send(WsMessage::Text(payload)).await.is_err() {
                    return Err(RemoteAuthEnd::Failed(
                        "O login por QR caiu antes da confirmação.".into(),
                    ));
                }
            }

            "pending_remote_init" => {
                let fingerprint = packet
                    .get("fingerprint")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if fingerprint.is_empty() {
                    return Err(RemoteAuthEnd::Failed(
                        "O Discord enviou um QR Code inválido.".into(),
                    ));
                }
                let expires_in = expires_at
                    .map(|deadline| deadline.saturating_duration_since(tokio::time::Instant::now()))
                    .unwrap_or(Duration::from_secs(120));
                notify(RemoteAuthEvent::Qr {
                    url: qr_url(fingerprint),
                    expires_in,
                });
                qr_shown = true;
            }

            "pending_ticket" => {
                let encrypted = packet
                    .get("encrypted_user_payload")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Ok(plain) = key.decrypt(encrypted) {
                    if let Ok(text) = String::from_utf8(plain) {
                        if let Some((username, discriminator)) = parse_user_payload(&text) {
                            notify(RemoteAuthEvent::Scanned {
                                username,
                                discriminator,
                            });
                        }
                    }
                }
            }

            "pending_login" => {
                let ticket = packet
                    .get("ticket")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if ticket.is_empty() {
                    return Err(RemoteAuthEnd::Failed(
                        "O Discord não enviou o ticket do login por QR.".into(),
                    ));
                }
                let encrypted_token = remote_auth_login(client, ticket).await?;
                let token = key.decrypt(&encrypted_token).map_err(RemoteAuthEnd::Failed)?;
                return String::from_utf8(token).map_err(|_| {
                    RemoteAuthEnd::Failed("O token devolvido pelo QR Code é inválido.".into())
                });
            }

            "cancel" => {
                if qr_shown {
                    return Err(RemoteAuthEnd::Expired);
                }
                return Err(RemoteAuthEnd::Cancelled);
            }

            "_error" => {
                let message = packet
                    .get("error")
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "erro desconhecido".to_string());
                return Err(RemoteAuthEnd::Failed(format!(
                    "O Discord recusou o login por QR: {message}"
                )));
            }

            "heartbeat_ack" => {}

            _ => {}
        }
    }
}

async fn remote_auth_login(client: &Client, ticket: &str) -> Result<String, RemoteAuthEnd> {
    let response = auth_request(client, Method::POST, "/users/@me/remote-auth/login")
        .json(&json!({ "ticket": ticket }))
        .send()
        .await
        .map_err(|err| RemoteAuthEnd::Failed(format!("Falha ao concluir o login por QR: {err}")))?;

    let status = response.status();
    let raw = response.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

    if let Some(token) = value.get("encrypted_token").and_then(Value::as_str) {
        return Ok(token.to_string());
    }

    Err(RemoteAuthEnd::Failed(describe_error(
        status.as_u16(),
        &value,
        &raw,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_from_login_response() {
        let value = json!({ "token": "abc.def.ghi", "user_id": "1" });
        assert_eq!(token_from(&value), Some("abc.def.ghi".to_string()));
        assert_eq!(token_from(&json!({ "mfa": true })), None);
    }

    #[test]
    fn detects_mfa_and_captcha_fields() {
        let mfa = json!({ "mfa": true, "ticket": "ticket-123" });
        assert_eq!(
            mfa.get("ticket").and_then(Value::as_str),
            Some("ticket-123")
        );
        let captcha = json!({
            "captcha_key": ["captcha-required"],
            "captcha_sitekey": "site-key",
            "captcha_service": "hcaptcha"
        });
        assert_eq!(string_field(&captcha, "captcha_service").as_deref(), Some("hcaptcha"));
    }

    #[test]
    fn parses_user_payload() {
        assert_eq!(
            parse_user_payload("123456789:0:a_123:ana"),
            Some(("ana".to_string(), "0".to_string()))
        );
        assert_eq!(
            parse_user_payload("1:4242:hash:ana:com:dois"),
            Some(("ana:com:dois".to_string(), "4242".to_string()))
        );
        assert_eq!(parse_user_payload("invalido"), None);
    }

    #[test]
    fn builds_qr_url_and_proof() {
        assert_eq!(
            qr_url("fingerprint"),
            "https://discord.com/ra/fingerprint"
        );
        let proof = nonce_proof(b"nonce");
        assert!(!proof.contains('='));
        assert!(!proof.contains('+'));
        assert!(!proof.contains('/'));
    }

    #[test]
    fn describes_invalid_credentials() {
        let message = describe_error(401, &json!({ "message": "Invalid Form Body" }), "");
        assert!(message.contains("incorretos"));
    }

    /// Teste de rede: confirma que o gateway de remote auth aceita nossa chave
    /// RSA e devolve um QR Code. Rode com:
    /// `cargo test remote_auth_gateway_returns_qr -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn remote_auth_gateway_returns_qr() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let client = http_client();
        let notify = move |event: RemoteAuthEvent| {
            if let RemoteAuthEvent::Qr { url, .. } = event {
                let _ = sender.send(url);
            }
        };

        let session = remote_auth_session(&client, notify);
        let _ = tokio::time::timeout(Duration::from_secs(25), session).await;

        let url = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("o gateway não devolveu um QR Code");
        assert!(url.starts_with("https://discord.com/ra/"), "url: {url}");
    }
}

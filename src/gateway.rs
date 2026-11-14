use std::sync::mpsc::Sender;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::{interval_at, timeout, Instant as TokioInstant, MissedTickBehavior};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::api::USER_AGENT;
use crate::model::{Channel, Guild, Member, Message};
use crate::worker::{emit, Event};

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

const INTENT_GUILDS: u64 = 1;
const INTENT_GUILD_MESSAGES: u64 = 512;
const INTENT_DIRECT_MESSAGES: u64 = 4096;
const INTENT_MESSAGE_CONTENT: u64 = 32768;

struct GatewayError {
    fatal: bool,
    code: Option<u16>,
    message: String,
}

fn gateway_error(message: impl Into<String>) -> GatewayError {
    GatewayError {
        fatal: false,
        code: None,
        message: message.into(),
    }
}

pub async fn run(
    token: String,
    is_bot: bool,
    events: Sender<Event>,
    ctx: eframe::egui::Context,
) {
    let mut intents = if is_bot {
        INTENT_GUILDS | INTENT_GUILD_MESSAGES | INTENT_DIRECT_MESSAGES | INTENT_MESSAGE_CONTENT
    } else {
        0
    };

    loop {
        emit(
            &events,
            &ctx,
            Event::Gateway {
                text: "Conectando ao gateway...".to_string(),
                ready: false,
            },
        );

        match connect_once(&token, is_bot, intents, &events, &ctx).await {
            Ok(()) => {}
            Err(err) => {
                if err.code == Some(4014) && is_bot && intents & INTENT_MESSAGE_CONTENT != 0 {
                    intents &= !INTENT_MESSAGE_CONTENT;
                    emit(
                        &events,
                        &ctx,
                        Event::Error(
                            "O bot não tem o intent MESSAGE_CONTENT aprovado. \
                             Reconectando sem ler o conteúdo das mensagens."
                                .to_string(),
                        ),
                    );
                    continue;
                }

                emit(&events, &ctx, Event::Error(err.message));
                emit(
                    &events,
                    &ctx,
                    Event::Gateway {
                        text: "Desconectado".to_string(),
                        ready: false,
                    },
                );

                if err.fatal {
                    return;
                }
            }
        }

        emit(
            &events,
            &ctx,
            Event::Gateway {
                text: "Reconectando...".to_string(),
                ready: false,
            },
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn connect_once(
    token: &str,
    is_bot: bool,
    intents: u64,
    events: &Sender<Event>,
    ctx: &eframe::egui::Context,
) -> Result<(), GatewayError> {
    let socket = match timeout(Duration::from_secs(20), connect_async(GATEWAY_URL)).await {
        Err(_) => return Err(gateway_error("Tempo esgotado ao conectar ao gateway.")),
        Ok(Err(err)) => return Err(gateway_error(format!("Falha ao conectar: {err}"))),
        Ok(Ok((socket, _response))) => socket,
    };

    let (mut write, mut read) = socket.split();

    let hello = match timeout(Duration::from_secs(20), read.next()).await {
        Err(_) => return Err(gateway_error("O gateway não enviou o HELLO a tempo.")),
        Ok(Some(Ok(message))) => message,
        Ok(Some(Err(err))) => {
            return Err(gateway_error(format!("Erro ao ler o gateway: {err}")))
        }
        Ok(None) => return Err(gateway_error("Conexão encerrada antes do HELLO.")),
    };

    let hello_text = ws_text(hello)
        .ok_or_else(|| gateway_error("Resposta inesperada do gateway (HELLO)."))?;
    let hello: Value = serde_json::from_str(&hello_text)
        .map_err(|_| gateway_error("HELLO inválido recebido do gateway."))?;

    if hello["op"].as_i64() != Some(10) {
        return Err(gateway_error("O gateway não enviou o opcode HELLO (10)."));
    }

    let heartbeat_ms = hello["d"]["heartbeat_interval"]
        .as_u64()
        .unwrap_or(41_250)
        .max(5_000);
    let heartbeat = Duration::from_millis(heartbeat_ms);

    let mut identify_data = json!({
        "token": token,
        "properties": {
            "os": "Windows",
            "browser": "Discord Phoenix",
            "device": "Discord Phoenix",
            "system_locale": "pt-BR",
            "browser_user_agent": USER_AGENT,
            "browser_version": "1.0.0",
            "os_version": "10",
            "referrer": "",
            "referring_domain": "",
            "referrer_current": "",
            "referring_domain_current": "",
            "release_channel": "stable",
            "client_build_number": 350000,
            "client_event_source": null
        },
        "presence": {
            "status": "online",
            "since": 0,
            "activities": [],
            "afk": false
        },
        "compress": false
    });

    if is_bot {
        identify_data["intents"] = json!(intents);
    } else {
        identify_data["capabilities"] = json!(16_381);
    }

    let identify = json!({ "op": 2, "d": identify_data });
    write
        .send(WsMessage::Text(identify.to_string()))
        .await
        .map_err(|err| gateway_error(format!("Falha ao identificar no gateway: {err}")))?;

    let mut sequence: Option<i64> = None;
    let mut heartbeat_timer = interval_at(TokioInstant::now() + heartbeat, heartbeat);
    heartbeat_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut awaiting_ack = false;
    let mut heartbeat_sent: Option<TokioInstant> = None;
    let mut force_heartbeat = false;

    loop {
        tokio::select! {
            _ = heartbeat_timer.tick() => {
                if awaiting_ack {
                    return Err(gateway_error(
                        "O gateway parou de responder aos heartbeats; reconectando.",
                    ));
                }

                let payload = json!({ "op": 1, "d": sequence });
                if let Err(err) = write.send(WsMessage::Text(payload.to_string())).await {
                    return Err(gateway_error(format!("Falha no heartbeat: {err}")));
                }
                awaiting_ack = true;
                heartbeat_sent = Some(TokioInstant::now());
            }

            incoming = read.next() => {
                let incoming = match incoming {
                    Some(Ok(message)) => message,
                    Some(Err(err)) => {
                        return Err(gateway_error(format!("Conexão com o gateway caiu: {err}")));
                    }
                    None => return Ok(()),
                };

                match incoming {
                    WsMessage::Text(text) => {
                        handle_payload(
                            &text,
                            &mut sequence,
                            &mut force_heartbeat,
                            &mut awaiting_ack,
                            &mut heartbeat_sent,
                            events,
                            ctx,
                        )?;
                    }
                    WsMessage::Binary(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        handle_payload(
                            &text,
                            &mut sequence,
                            &mut force_heartbeat,
                            &mut awaiting_ack,
                            &mut heartbeat_sent,
                            events,
                            ctx,
                        )?;
                    }
                    WsMessage::Ping(payload) => {
                        let _ = write.send(WsMessage::Pong(payload)).await;
                    }
                    WsMessage::Close(frame) => {
                        let code = frame.as_ref().map(|frame| u16::from(frame.code));
                        if let Some(code) = code {
                            if is_fatal_close(code) {
                                return Err(GatewayError {
                                    fatal: true,
                                    code: Some(code),
                                    message: close_message(code),
                                });
                            }
                        }
                        return Ok(());
                    }
                    _ => {}
                }

                if force_heartbeat {
                    let payload = json!({ "op": 1, "d": sequence });
                    write
                        .send(WsMessage::Text(payload.to_string()))
                        .await
                        .map_err(|err| {
                            gateway_error(format!("Falha no heartbeat: {err}"))
                        })?;
                    awaiting_ack = true;
                    heartbeat_sent = Some(TokioInstant::now());
                    force_heartbeat = false;
                }
            }
        }
    }
}

fn handle_payload(
    text: &str,
    sequence: &mut Option<i64>,
    force_heartbeat: &mut bool,
    awaiting_ack: &mut bool,
    heartbeat_sent: &mut Option<TokioInstant>,
    events: &Sender<Event>,
    ctx: &eframe::egui::Context,
) -> Result<(), GatewayError> {
    let payload: Value = serde_json::from_str(text)
        .map_err(|_| gateway_error("Payload inválido recebido do gateway."))?;

    if let Some(seq) = payload["s"].as_i64() {
        *sequence = Some(seq);
    }

    match payload["op"].as_i64() {
        Some(11) => {
            *awaiting_ack = false;
            if let Some(sent) = heartbeat_sent.take() {
                let latency = sent.elapsed().as_millis() as u64;
                emit(events, ctx, Event::Ping(latency));
            }
        }
        Some(1) => {
            *force_heartbeat = true;
        }
        Some(7) => {
            return Err(gateway_error("O gateway pediu reconexão."));
        }
        Some(9) => {
            return Err(gateway_error("Sessão do gateway invalidada; reconectando."));
        }
        Some(0) => {
            let event_name = payload["t"].as_str().unwrap_or_default();
            let data = &payload["d"];
            dispatch(event_name, data, events, ctx);
        }
        _ => {}
    }

    Ok(())
}

fn dispatch(
    event_name: &str,
    data: &Value,
    events: &Sender<Event>,
    ctx: &eframe::egui::Context,
) {
    match event_name {
        "READY" => {
            emit(
                events,
                ctx,
                Event::Gateway {
                    text: "Conectado".to_string(),
                    ready: true,
                },
            );
        }
        "GUILD_CREATE" => {
            let guild_id = data["id"].as_str().unwrap_or_default().to_string();
            if guild_id.is_empty() {
                return;
            }
            if let Some(channels_value) = data.get("channels") {
                if let Ok(channels) =
                    serde_json::from_value::<Vec<Channel>>(channels_value.clone())
                {
                    emit(
                        events,
                        ctx,
                        Event::Channels {
                            guild_id: guild_id.clone(),
                            channels,
                        },
                    );
                }
            }
            if let Ok(guild) = serde_json::from_value::<Guild>(data.clone()) {
                emit(events, ctx, Event::GuildUpdated(guild));
            }
            if let Some(members_value) = data.get("members") {
                if let Ok(members) = serde_json::from_value::<Vec<Member>>(members_value.clone())
                {
                    if !members.is_empty() {
                        emit(
                            events,
                            ctx,
                            Event::Members {
                                guild_id,
                                members,
                                error: None,
                            },
                        );
                    }
                }
            }
        }
        "GUILD_UPDATE" => {
            if let Ok(guild) = serde_json::from_value::<Guild>(data.clone()) {
                if !guild.id.is_empty() {
                    emit(events, ctx, Event::GuildUpdated(guild));
                }
            }
        }
        "GUILD_MEMBER_ADD" | "GUILD_MEMBER_UPDATE" => {
            let guild_id = data["guild_id"].as_str().unwrap_or_default().to_string();
            if !guild_id.is_empty() {
                if let Ok(member) = serde_json::from_value::<Member>(data.clone()) {
                    if !member.id().is_empty() {
                        emit(
                            events,
                            ctx,
                            Event::MemberUpsert { guild_id, member },
                        );
                    }
                }
            }
        }
        "GUILD_MEMBER_REMOVE" => {
            let guild_id = data["guild_id"].as_str().unwrap_or_default().to_string();
            let user_id = data["user"]["id"].as_str().unwrap_or_default().to_string();
            if !guild_id.is_empty() && !user_id.is_empty() {
                emit(
                    events,
                    ctx,
                    Event::MemberRemoved {
                        guild_id,
                        user_id,
                    },
                );
            }
        }
        "MESSAGE_CREATE" => {
            if let Ok(message) = serde_json::from_value::<Message>(data.clone()) {
                emit(events, ctx, Event::MessageSent(message));
            }
        }
        "MESSAGE_UPDATE" => {
            if let Ok(message) = serde_json::from_value::<Message>(data.clone()) {
                emit(events, ctx, Event::MessageUpdated(message));
            }
        }
        "MESSAGE_DELETE" => {
            let channel_id = data["channel_id"].as_str().unwrap_or_default().to_string();
            let message_id = data["id"].as_str().unwrap_or_default().to_string();
            if !channel_id.is_empty() && !message_id.is_empty() {
                emit(
                    events,
                    ctx,
                    Event::MessageDeleted {
                        channel_id,
                        message_id,
                    },
                );
            }
        }
        "TYPING_START" => {
            let channel_id = data["channel_id"].as_str().unwrap_or_default().to_string();
            let user_id = data["user_id"].as_str().unwrap_or_default().to_string();

            let name = data
                .pointer("/member/user/global_name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    data.pointer("/member/user/username")
                        .and_then(Value::as_str)
                })
                .or_else(|| data.pointer("/user/global_name").and_then(Value::as_str))
                .or_else(|| data.pointer("/user/username").and_then(Value::as_str))
                .unwrap_or("Alguém")
                .to_string();

            if !channel_id.is_empty() {
                emit(
                    events,
                    ctx,
                    Event::TypingStart {
                        channel_id,
                        name,
                        user_id,
                    },
                );
            }
        }
        _ => {}
    }
}

fn ws_text(message: WsMessage) -> Option<String> {
    match message {
        WsMessage::Text(text) => Some(text),
        WsMessage::Binary(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        _ => None,
    }
}

fn is_fatal_close(code: u16) -> bool {
    matches!(code, 4004 | 4010 | 4011 | 4012 | 4013 | 4014)
}

fn close_message(code: u16) -> String {
    match code {
        4004 => "Token rejeitado pelo gateway (4004). Confira o token e o tipo de conta."
            .to_string(),
        4010 => "Shard inválido (4010).".to_string(),
        4011 => "O bot precisa de sharding (4011).".to_string(),
        4012 => "Versão de API inválida (4012).".to_string(),
        4013 => "Intents inválidos (4013).".to_string(),
        4014 => {
            "Intent não autorizado (4014). Para bots, habilite MESSAGE_CONTENT no portal \
             de desenvolvedores do Discord."
                .to_string()
        }
        _ => format!("Gateway encerrado com o código {code}."),
    }
}

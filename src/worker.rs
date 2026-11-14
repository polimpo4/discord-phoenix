use std::sync::mpsc::Sender;
use std::time::Duration;

use base64::Engine;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::api::Api;
use crate::auth;
use crate::gateway;
use crate::model::{Channel, Guild, Member, Message, User};

const MAX_MEDIA_BYTES: usize = 8 * 1024 * 1024;

pub enum Command {
    Login {
        token: String,
        is_bot: bool,
    },
    LoginPassword {
        email: String,
        password: String,
    },
    SubmitMfa {
        ticket: String,
        code: String,
    },
    RemoteAuthStart,
    RemoteAuthCancel,
    Logout,
    LoadChannels {
        guild_id: String,
    },
    LoadMessages {
        channel_id: String,
    },
    LoadGuildMembers {
        guild_id: String,
    },
    LoadGuildInfo {
        guild_id: String,
    },
    UpdateGuildName {
        guild_id: String,
        name: String,
    },
    SendMessage {
        channel_id: String,
        content: String,
    },
    EditMessage {
        channel_id: String,
        message_id: String,
        content: String,
    },
    DeleteMessage {
        channel_id: String,
        message_id: String,
    },
    Typing {
        channel_id: String,
    },
    FetchImage {
        url: String,
    },
}

pub enum Event {
    LoggedIn {
        token: String,
        me: User,
        guilds: Vec<Guild>,
        dms: Vec<Channel>,
        is_bot: bool,
    },
    LoginFailed(String),
    LoginMfa {
        ticket: String,
    },
    RemoteAuthQr {
        url: String,
        expires_in_secs: u64,
    },
    RemoteAuthScanned {
        username: String,
        discriminator: String,
    },
    RemoteAuthExpired,
    RemoteAuthFailed(String),
    Channels {
        guild_id: String,
        channels: Vec<Channel>,
    },
    Messages {
        channel_id: String,
        messages: Vec<Message>,
    },
    Members {
        guild_id: String,
        members: Vec<Member>,
        error: Option<String>,
    },
    GuildInfo(Guild),
    GuildRenamed {
        guild_id: String,
        name: String,
    },
    GuildUpdated(Guild),
    MemberUpsert {
        guild_id: String,
        member: Member,
    },
    MemberRemoved {
        guild_id: String,
        user_id: String,
    },
    MessageSent(Message),
    MessageUpdated(Message),
    MessageDeleted {
        channel_id: String,
        message_id: String,
    },
    TypingStart {
        channel_id: String,
        name: String,
        user_id: String,
    },
    Gateway {
        text: String,
        ready: bool,
    },
    Ping(u64),
    Error(String),
    LoggedOut,
    ImageLoaded {
        url: String,
        result: Result<Vec<u8>, String>,
    },
}

pub fn spawn(
    runtime: &tokio::runtime::Runtime,
    events: Sender<Event>,
    ctx: eframe::egui::Context,
) -> tokio::sync::mpsc::UnboundedSender<Command> {
    let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
    let sender = commands.clone();
    runtime.spawn(actor(receiver, sender, events, ctx));
    commands
}

async fn actor(
    mut commands: UnboundedReceiver<Command>,
    sender: UnboundedSender<Command>,
    events: Sender<Event>,
    ctx: eframe::egui::Context,
) {
    let mut api: Option<Api> = None;
    let mut gateway_task: Option<JoinHandle<()>> = None;
    let mut remote_task: Option<JoinHandle<()>> = None;
    let auth_http = auth::http_client();
    let media = reqwest::Client::builder()
        .user_agent(crate::api::USER_AGENT)
        .timeout(Duration::from_secs(25))
        .build()
        .expect("falha ao criar cliente de mídia");

    while let Some(command) = commands.recv().await {
        match command {
            Command::Login { token, is_bot } => {
                abort_all(&mut gateway_task, &mut remote_task);
                api = None;

                if let Some((new_api, task)) =
                    establish(token, is_bot, &events, &ctx).await
                {
                    api = Some(new_api);
                    gateway_task = Some(task);
                }
            }

            Command::LoginPassword { email, password } => {
                abort_all(&mut gateway_task, &mut remote_task);
                api = None;

                match auth::password_login(&auth_http, &email, &password).await {
                    Ok(auth::PasswordOutcome::Token(token)) => {
                        if let Some((new_api, task)) = establish(token, false, &events, &ctx).await {
                            api = Some(new_api);
                            gateway_task = Some(task);
                        }
                    }
                    Ok(auth::PasswordOutcome::Mfa { ticket }) => {
                        emit(&events, &ctx, Event::LoginMfa { ticket });
                    }
                    Ok(auth::PasswordOutcome::Captcha { .. }) => {
                        emit(
                            &events,
                            &ctx,
                            Event::LoginFailed(auth::CAPTCHA_HELP.to_string()),
                        );
                    }
                    Err(err) => emit(&events, &ctx, Event::LoginFailed(err)),
                }
            }

            Command::SubmitMfa { ticket, code } => {
                match auth::mfa_login(&auth_http, &ticket, &code).await {
                    Ok(token) => {
                        if let Some((new_api, task)) = establish(token, false, &events, &ctx).await {
                            api = Some(new_api);
                            gateway_task = Some(task);
                        }
                    }
                    Err(err) => emit(&events, &ctx, Event::LoginFailed(err)),
                }
            }

            Command::RemoteAuthStart => {
                abort_all(&mut gateway_task, &mut remote_task);
                api = None;

                emit(
                    &events,
                    &ctx,
                    Event::Gateway {
                        text: "Aguardando confirmação no celular...".to_string(),
                        ready: false,
                    },
                );

                let http = auth_http.clone();
                let events = events.clone();
                let ctx = ctx.clone();
                let commands = sender.clone();
                remote_task = Some(tokio::spawn(async move {
                    loop {
                        let notify_events = events.clone();
                        let notify_ctx = ctx.clone();
                        let notify = move |event: auth::RemoteAuthEvent| match event {
                            auth::RemoteAuthEvent::Qr { url, expires_in } => emit(
                                &notify_events,
                                &notify_ctx,
                                Event::RemoteAuthQr {
                                    url,
                                    expires_in_secs: expires_in.as_secs().max(1),
                                },
                            ),
                            auth::RemoteAuthEvent::Scanned {
                                username,
                                discriminator,
                            } => emit(
                                &notify_events,
                                &notify_ctx,
                                Event::RemoteAuthScanned {
                                    username,
                                    discriminator,
                                },
                            ),
                        };

                        match auth::remote_auth_session(&http, notify).await {
                            auth::RemoteAuthEnd::Token(token) => {
                                let _ = commands.send(Command::Login {
                                    token,
                                    is_bot: false,
                                });
                                break;
                            }
                            auth::RemoteAuthEnd::Expired => {
                                emit(&events, &ctx, Event::RemoteAuthExpired);
                                tokio::time::sleep(Duration::from_millis(350)).await;
                            }
                            auth::RemoteAuthEnd::Cancelled => {
                                break;
                            }
                            auth::RemoteAuthEnd::Failed(message) => {
                                emit(&events, &ctx, Event::RemoteAuthFailed(message));
                                break;
                            }
                        }
                    }
                }));
            }

            Command::RemoteAuthCancel => {
                if let Some(task) = remote_task.take() {
                    task.abort();
                }
            }

            Command::Logout => {
                abort_all(&mut gateway_task, &mut remote_task);
                api = None;
                emit(&events, &ctx, Event::LoggedOut);
            }

            Command::LoadChannels { guild_id } => {
                if let Some(api) = &api {
                    match api.guild_channels(&guild_id).await {
                        Ok(channels) => emit(
                            &events,
                            &ctx,
                            Event::Channels {
                                guild_id,
                                channels,
                            },
                        ),
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::LoadMessages { channel_id } => {
                if let Some(api) = &api {
                    match api.messages(&channel_id, 50).await {
                        Ok(messages) => emit(
                            &events,
                            &ctx,
                            Event::Messages {
                                channel_id,
                                messages,
                            },
                        ),
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::LoadGuildMembers { guild_id } => {
                if let Some(api) = &api {
                    match api.guild_members(&guild_id, 1000).await {
                        Ok(members) => emit(
                            &events,
                            &ctx,
                            Event::Members {
                                guild_id,
                                members,
                                error: None,
                            },
                        ),
                        Err(err) => emit(
                            &events,
                            &ctx,
                            Event::Members {
                                guild_id,
                                members: Vec::new(),
                                error: Some(err.to_string()),
                            },
                        ),
                    }
                }
            }

            Command::LoadGuildInfo { guild_id } => {
                if let Some(api) = &api {
                    match api.guild(&guild_id).await {
                        Ok(guild) => emit(&events, &ctx, Event::GuildInfo(guild)),
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::UpdateGuildName { guild_id, name } => {
                if let Some(api) = &api {
                    match api.modify_guild_name(&guild_id, &name).await {
                        Ok(guild) => {
                            emit(&events, &ctx, Event::GuildInfo(guild.clone()));
                            emit(
                                &events,
                                &ctx,
                                Event::GuildRenamed {
                                    guild_id: guild.id,
                                    name: guild.name,
                                },
                            );
                        }
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::SendMessage {
                channel_id,
                content,
            } => {
                if let Some(api) = &api {
                    match api.send_message(&channel_id, &content).await {
                        Ok(message) => emit(&events, &ctx, Event::MessageSent(message)),
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::EditMessage {
                channel_id,
                message_id,
                content,
            } => {
                if let Some(api) = &api {
                    match api
                        .edit_message(&channel_id, &message_id, &content)
                        .await
                    {
                        Ok(message) => {
                            emit(&events, &ctx, Event::MessageUpdated(message))
                        }
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::DeleteMessage {
                channel_id,
                message_id,
            } => {
                if let Some(api) = &api {
                    match api.delete_message(&channel_id, &message_id).await {
                        Ok(()) => emit(
                            &events,
                            &ctx,
                            Event::MessageDeleted {
                                channel_id,
                                message_id,
                            },
                        ),
                        Err(err) => emit(&events, &ctx, Event::Error(err.to_string())),
                    }
                }
            }

            Command::Typing { channel_id } => {
                if let Some(api) = &api {
                    let _ = api.send_typing(&channel_id).await;
                }
            }

            Command::FetchImage { url } => {
                let events = events.clone();
                let ctx = ctx.clone();
                let client = media.clone();
                // Baixa em paralelo para não travar canais/mensagens enquanto
                // uma imagem grande está sendo buscada.
                tokio::spawn(async move {
                    let result = fetch_media(&client, &url).await;
                    emit(&events, &ctx, Event::ImageLoaded { url, result });
                });
            }
        }
    }
}

fn abort_all(gateway: &mut Option<JoinHandle<()>>, remote: &mut Option<JoinHandle<()>>) {
    if let Some(task) = gateway.take() {
        task.abort();
    }
    if let Some(task) = remote.take() {
        task.abort();
    }
}

/// Valida o token, carrega o usuário/servidores e inicia o gateway. Devolve a
/// API pronta e a tarefa do gateway para o ator guardar.
async fn establish(
    token: String,
    is_bot: bool,
    events: &Sender<Event>,
    ctx: &eframe::egui::Context,
) -> Option<(Api, JoinHandle<()>)> {
    emit(
        events,
        ctx,
        Event::Gateway {
            text: "Conectando ao Discord...".to_string(),
            ready: false,
        },
    );

    let candidate = Api::new(token.clone(), is_bot);
    let me = match candidate.me().await {
        Ok(me) => me,
        Err(err) => {
            emit(events, ctx, Event::LoginFailed(err.to_string()));
            return None;
        }
    };

    let guilds = match candidate.guilds().await {
        Ok(guilds) => guilds,
        Err(err) => {
            emit(events, ctx, Event::LoginFailed(err.to_string()));
            return None;
        }
    };

    let dms = if is_bot {
        Vec::new()
    } else {
        candidate.direct_messages().await.unwrap_or_default()
    };

    emit(
        events,
        ctx,
        Event::LoggedIn {
            token: token.clone(),
            me,
            guilds,
            dms,
            is_bot,
        },
    );

    let task = tokio::spawn(gateway::run(token, is_bot, events.clone(), ctx.clone()));
    Some((candidate, task))
}

async fn fetch_media(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    if let Some(data) = url.strip_prefix("data:") {
        let (_, payload) = data
            .split_once(',')
            .ok_or_else(|| "data URI inválida".to_string())?;
        return base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|err| format!("imagem embutida inválida: {err}"));
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("falha ao baixar imagem: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("imagem: HTTP {}", response.status()));
    }
    if let Some(length) = response.content_length() {
        if length as usize > MAX_MEDIA_BYTES {
            return Err("imagem grande demais para o cache".to_string());
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("falha ao ler imagem: {err}"))?;
    if bytes.len() > MAX_MEDIA_BYTES {
        return Err("imagem grande demais para o cache".to_string());
    }

    Ok(bytes.to_vec())
}

pub fn emit(events: &Sender<Event>, ctx: &eframe::egui::Context, event: Event) {
    let _ = events.send(event);
    ctx.request_repaint();
}

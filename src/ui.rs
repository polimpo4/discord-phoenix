use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Label, Layout, Margin, RichText, Rounding,
    ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
};

use crate::config::{self, Settings};
use crate::demo;
use crate::emoji;
use crate::images::{paint_rounded_texture, ImageCache};
use crate::markdown;
use crate::model::{
    clean_content, initials, Attachment, Channel, Embed, EmbedImage, Guild, Member, Message, User,
};
use crate::theme;
use crate::worker::{self, Command, Event};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Login,
    Main,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Guild,
    Dm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginMode {
    Password,
    Qr,
    Token,
}

struct QrView {
    texture: egui::TextureHandle,
    expires_at: Instant,
    scanned: Option<String>,
}

enum MsgAction {
    Edit(Message),
    Delete(Message),
}

pub struct App {
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
    commands: tokio::sync::mpsc::UnboundedSender<Command>,
    events: std::sync::mpsc::Receiver<Event>,

    screen: Screen,
    token: String,
    is_bot: bool,
    remember: bool,
    busy: bool,
    login_error: Option<String>,
    auto_login: bool,
    settings: Settings,

    login_mode: LoginMode,
    login_email: String,
    login_password: String,
    login_code: String,
    login_mfa: Option<String>,
    qr: Option<QrView>,
    qr_error: Option<String>,
    force_remember: bool,
    logo: Option<egui::TextureHandle>,

    me: Option<User>,
    demo: bool,
    guilds: Vec<Guild>,
    channels_by_guild: HashMap<String, Vec<Channel>>,
    members_by_guild: HashMap<String, Vec<Member>>,
    loaded_guild_info: HashSet<String>,
    loaded_members: HashSet<String>,
    member_error: Option<String>,
    loading_members: bool,
    show_members: bool,
    settings_open: bool,
    guild_name_edit: String,
    emoji_picker_open: bool,
    emoji_query: String,
    emoji_group: Option<&'static str>,
    dms: Vec<Channel>,
    selected_guild: Option<String>,
    view: View,
    active: Option<Channel>,
    messages: Vec<Message>,
    demo_messages: HashMap<String, Vec<Message>>,
    drafts: HashMap<String, String>,
    loading_channels: bool,
    loading_messages: bool,

    composer: String,
    editing: Option<String>,
    pre_edit: Option<String>,
    focus_composer: bool,
    last_typing: Option<Instant>,
    typing: HashMap<String, (String, Instant)>,

    unread: HashSet<String>,
    gateway_status: String,
    connected: bool,
    ping: Option<u64>,
    toast: Option<(String, Instant)>,
    scroll_to_bottom: bool,

    ram_mb: f32,
    last_ram_check: Instant,
    next_local_id: u64,
    images: ImageCache,
}

fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let bytes = include_bytes!("../assets/logo.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ctx.load_texture("phoenix-logo", color, egui::TextureOptions::LINEAR))
}

fn qr_texture(ctx: &egui::Context, data: &str) -> Option<egui::TextureHandle> {
    let code = qrcode::QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::M).ok()?;
    let width = code.width();
    let colors = code.to_colors();

    // Quiet zone de 4 módulos e 1 pixel por módulo. A textura sobe com filtro
    // NEAREST: os módulos ficam nítidos em qualquer escala e o QR continua
    // escaneável (feathering do egui borrava as bordas).
    let quiet = 4usize;
    let total = width + quiet * 2;
    let mut image = egui::ColorImage::new([total, total], Color32::WHITE);
    for y in 0..width {
        for x in 0..width {
            if matches!(colors[y * width + x], qrcode::types::Color::Dark) {
                image[(x + quiet, y + quiet)] = Color32::BLACK;
            }
        }
    }

    Some(ctx.load_texture("phoenix-qr", image, egui::TextureOptions::NEAREST))
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("phoenix-worker")
            .enable_all()
            .build()
            .expect("falha ao criar runtime tokio");

        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let commands = worker::spawn(&runtime, event_tx, cc.egui_ctx.clone());

        let settings = config::load_settings();
        let saved_token = if settings.remember {
            config::load_token()
        } else {
            None
        };

        let mut app = Self {
            runtime,
            commands,
            events: event_rx,
            screen: Screen::Login,
            token: String::new(),
            is_bot: settings.is_bot,
            remember: settings.remember,
            busy: false,
            login_error: None,
            auto_login: false,
            settings,
            login_mode: LoginMode::Password,
            login_email: String::new(),
            login_password: String::new(),
            login_code: String::new(),
            login_mfa: None,
            qr: None,
            qr_error: None,
            force_remember: false,
            logo: load_logo(&cc.egui_ctx),
            me: None,
            demo: false,
            guilds: Vec::new(),
            channels_by_guild: HashMap::new(),
            members_by_guild: HashMap::new(),
            loaded_guild_info: HashSet::new(),
            loaded_members: HashSet::new(),
            member_error: None,
            loading_members: false,
            show_members: true,
            settings_open: false,
            guild_name_edit: String::new(),
            emoji_picker_open: false,
            emoji_query: String::new(),
            emoji_group: None,
            dms: Vec::new(),
            selected_guild: None,
            view: View::Guild,
            active: None,
            messages: Vec::new(),
            demo_messages: HashMap::new(),
            drafts: HashMap::new(),
            loading_channels: false,
            loading_messages: false,
            composer: String::new(),
            editing: None,
            pre_edit: None,
            focus_composer: false,
            last_typing: None,
            typing: HashMap::new(),
            unread: HashSet::new(),
            gateway_status: "Desconectado".to_string(),
            connected: false,
            ping: None,
            toast: None,
            scroll_to_bottom: false,
            ram_mb: config::process_ram_mb(),
            last_ram_check: Instant::now(),
            next_local_id: 1,
            images: ImageCache::new(),
        };

        if std::env::var_os("PHOENIX_DEMO").is_some() {
            app.enter_demo();
        } else if let Some(token) = saved_token {
            app.token = token;
            app.busy = true;
            app.auto_login = true;
            let _ = app.commands.send(Command::Login {
                token: app.token.clone(),
                is_bot: app.is_bot,
            });
        }

        app
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::LoggedIn {
                    token,
                    me,
                    guilds,
                    dms,
                    is_bot,
                } => {
                    self.token = token;
                    if self.force_remember {
                        self.remember = true;
                    }
                    self.force_remember = false;
                    self.login_password.clear();
                    self.login_code.clear();
                    self.login_mfa = None;
                    self.qr = None;
                    self.qr_error = None;
                    self.screen = Screen::Main;
                    self.busy = false;
                    self.auto_login = false;
                    self.me = Some(me);
                    self.guilds = guilds;
                    self.dms = dms;
                    self.is_bot = is_bot;
                    self.demo = false;
                    self.login_error = None;
                    self.gateway_status = "Conectando...".to_string();
                    self.connected = false;
                    self.channels_by_guild.clear();
                    self.members_by_guild.clear();
                    self.loaded_guild_info.clear();
                    self.loaded_members.clear();
                    self.member_error = None;
                    self.loading_members = false;
                    self.settings_open = false;
                    self.messages.clear();
                    self.active = None;
                    self.unread.clear();
                    self.typing.clear();
                    self.images.clear();
                    self.persist_session();

                    if let Some(first) = self.guilds.first().map(|guild| guild.id.clone()) {
                        self.select_guild(first);
                    } else if let Some(dm) = self.dms.first().cloned() {
                        self.view = View::Dm;
                        self.open_channel(dm);
                    }
                }

                Event::LoginFailed(message) => {
                    self.busy = false;
                    self.login_error = Some(message);
                    if self.auto_login {
                        self.auto_login = false;
                        config::clear_token();
                        self.remember = false;
                        self.settings.remember = false;
                        config::save_settings(&self.settings);
                    }
                }

                Event::LoginMfa { ticket } => {
                    self.busy = false;
                    self.login_error = None;
                    self.login_mfa = Some(ticket);
                    self.login_code.clear();
                }

                Event::RemoteAuthQr {
                    url,
                    expires_in_secs,
                } => {
                    self.busy = false;
                    self.qr_error = None;
                    match qr_texture(ctx, &url) {
                        Some(texture) => {
                            self.qr = Some(QrView {
                                texture,
                                expires_at: Instant::now()
                                    + Duration::from_secs(expires_in_secs),
                                scanned: None,
                            });
                        }
                        None => {
                            self.qr = None;
                            self.qr_error = Some(
                                "Não foi possível desenhar o QR Code. Tente gerar outro."
                                    .to_string(),
                            );
                        }
                    }
                }

                Event::RemoteAuthScanned {
                    username,
                    discriminator,
                } => {
                    if let Some(qr) = &mut self.qr {
                        qr.scanned = Some(if discriminator == "0" || discriminator.is_empty() {
                            format!("@{username}")
                        } else {
                            format!("@{username}#{discriminator}")
                        });
                    }
                }

                Event::RemoteAuthExpired => {
                    if let Some(qr) = &mut self.qr {
                        qr.expires_at = Instant::now();
                    }
                }

                Event::RemoteAuthFailed(message) => {
                    self.busy = false;
                    self.qr = None;
                    self.qr_error = Some(message);
                }

                Event::Channels {
                    guild_id,
                    channels,
                } => {
                    let list = channels.clone();
                    self.channels_by_guild.insert(guild_id.clone(), channels);

                    if self.selected_guild.as_deref() == Some(guild_id.as_str()) {
                        self.loading_channels = false;

                        if let Some(active) = self.active.clone() {
                            if let Some(updated) = list
                                .iter()
                                .find(|channel| channel.id == active.id)
                                .cloned()
                            {
                                self.active = Some(updated);
                            } else {
                                self.active = None;
                                self.messages.clear();
                                self.pick_default_channel(&list);
                            }
                        } else {
                            self.pick_default_channel(&list);
                        }
                    }
                }

                Event::Messages {
                    channel_id,
                    messages,
                } => {
                    if self.active.as_ref().map(|channel| channel.id.as_str())
                        == Some(channel_id.as_str())
                    {
                        self.messages = messages;
                        self.loading_messages = false;
                        self.scroll_to_bottom = true;
                    }
                }

                Event::Members {
                    guild_id,
                    mut members,
                    error,
                } => {
                    if self.selected_guild.as_deref() == Some(guild_id.as_str()) {
                        self.loading_members = false;
                        self.member_error = error.clone();
                    }
                    self.loaded_members.insert(guild_id.clone());
                    members.sort_by(|a, b| {
                        a.display_name()
                            .to_lowercase()
                            .cmp(&b.display_name().to_lowercase())
                    });
                    if !members.is_empty() || error.is_none() {
                        self.members_by_guild.insert(guild_id, members);
                    }
                }

                Event::GuildInfo(guild) => {
                    let is_selected =
                        self.selected_guild.as_deref() == Some(guild.id.as_str());
                    self.merge_guild(guild.clone());
                    if is_selected {
                        self.loaded_guild_info.insert(guild.id.clone());
                        if self.editing.is_none() {
                            self.guild_name_edit = guild.name.clone();
                        }
                    }
                }

                Event::GuildUpdated(guild) => {
                    let is_selected =
                        self.selected_guild.as_deref() == Some(guild.id.as_str());
                    self.merge_guild(guild.clone());
                    if is_selected {
                        if self.editing.is_none() {
                            self.guild_name_edit = guild.name.clone();
                        }
                        if !guild.roles.is_empty() {
                            self.loaded_guild_info.insert(guild.id.clone());
                        }
                    }
                }

                Event::GuildRenamed { guild_id, name } => {
                    if let Some(guild) = self.guilds.iter_mut().find(|g| g.id == guild_id) {
                        guild.name = name.clone();
                    }
                    if self.selected_guild.as_deref() == Some(guild_id.as_str()) {
                        self.guild_name_edit = name;
                    }
                }

                Event::MemberUpsert { guild_id, member } => {
                    let list = self.members_by_guild.entry(guild_id).or_default();
                    if let Some(existing) =
                        list.iter_mut().find(|existing| existing.id() == member.id())
                    {
                        *existing = member;
                    } else {
                        list.push(member);
                        list.sort_by(|a, b| {
                            a.display_name()
                                .to_lowercase()
                                .cmp(&b.display_name().to_lowercase())
                        });
                    }
                }

                Event::MemberRemoved { guild_id, user_id } => {
                    if let Some(list) = self.members_by_guild.get_mut(&guild_id) {
                        list.retain(|member| member.id() != user_id);
                    }
                }

                Event::MessageSent(message) => {
                    if self.active.as_ref().map(|channel| channel.id.as_str())
                        == Some(message.channel_id.as_str())
                    {
                        if !self.messages.iter().any(|existing| existing.id == message.id) {
                            self.messages.push(message);
                            self.trim_messages();
                            self.scroll_to_bottom = true;
                        }
                    } else if self
                        .me
                        .as_ref()
                        .map(|me| me.id != message.author.id)
                        .unwrap_or(true)
                    {
                        self.unread.insert(message.channel_id);
                    }
                }

                Event::MessageUpdated(message) => {
                    if self.active.as_ref().map(|channel| channel.id.as_str())
                        == Some(message.channel_id.as_str())
                    {
                        if let Some(existing) = self
                            .messages
                            .iter_mut()
                            .find(|existing| existing.id == message.id)
                        {
                            if !message.content.is_empty() || message.edited_timestamp.is_some() {
                                existing.content = message.content;
                            }
                            if message.edited_timestamp.is_some() {
                                existing.edited_timestamp = message.edited_timestamp;
                            }
                            if !message.embeds.is_empty() {
                                existing.embeds = message.embeds;
                            }
                            if !message.attachments.is_empty() {
                                existing.attachments = message.attachments;
                            }
                        }
                    }
                }

                Event::MessageDeleted {
                    channel_id,
                    message_id,
                } => {
                    if self.active.as_ref().map(|channel| channel.id.as_str())
                        == Some(channel_id.as_str())
                    {
                        self.messages.retain(|message| message.id != message_id);
                    }
                }

                Event::TypingStart {
                    channel_id,
                    name,
                    user_id,
                } => {
                    if self.me.as_ref().map(|me| me.id.as_str()) != Some(user_id.as_str()) {
                        self.typing.insert(channel_id, (name, Instant::now()));
                    }
                }

                Event::Gateway { text, ready } => {
                    self.gateway_status = text;
                    self.connected = ready;
                    if !ready && self.ping.is_some() {
                        self.ping = None;
                    }
                }

                Event::Ping(latency) => {
                    self.ping = Some(latency);
                }

                Event::Error(message) => {
                    self.toast = Some((message, Instant::now()));
                    self.loading_messages = false;
                    self.loading_channels = false;
                }

                Event::LoggedOut => {
                    self.reset_to_login();
                }

                Event::ImageLoaded { url, result } => {
                    match result {
                        Ok(bytes) => self.images.receive(ctx, url, bytes),
                        Err(_) => self.images.mark_failed(url),
                    }
                    // A midia pode aumentar a altura da mensagem depois que o
                    // historico ja foi desenhado; mantem o fim da conversa visivel.
                    self.scroll_to_bottom = true;
                }
            }
        }

        if let Some((_, when)) = &self.toast {
            if when.elapsed() > Duration::from_secs(8) {
                self.toast = None;
            }
        }
        self.typing
            .retain(|_, (_, when)| when.elapsed() < Duration::from_secs(9));
    }

    /// Atualiza somente os campos presentes em eventos parciais do gateway
    /// (GUILD_UPDATE costuma mandar apenas o que mudou).
    fn merge_guild(&mut self, incoming: Guild) {
        if let Some(existing) = self
            .guilds
            .iter_mut()
            .find(|guild| guild.id == incoming.id)
        {
            if !incoming.name.is_empty() {
                existing.name = incoming.name;
            }
            if incoming.icon.is_some() {
                existing.icon = incoming.icon;
            }
            if incoming.owner_id.is_some() {
                existing.owner_id = incoming.owner_id;
            }
            if incoming.member_count.is_some() {
                existing.member_count = incoming.member_count;
            }
            if incoming.description.is_some() {
                existing.description = incoming.description;
            }
            if !incoming.roles.is_empty() {
                existing.roles = incoming.roles;
            }
        } else if !incoming.id.is_empty() {
            self.guilds.push(incoming);
        }
    }

    fn selected_guild_info(&self) -> Option<&Guild> {
        let guild_id = self.selected_guild.as_ref()?;
        self.guilds.iter().find(|guild| &guild.id == guild_id)
    }

    fn persist_session(&mut self) {        if self.remember {
            config::save_token(&self.token);
        } else {
            config::clear_token();
        }
        self.settings.remember = self.remember;
        self.settings.is_bot = self.is_bot;
        config::save_settings(&self.settings);
    }

    fn start_login(&mut self) {
        let mut token = self.token.trim().to_string();
        if token.is_empty() {
            self.login_error = Some("Cole um token de conta ou de bot para entrar.".to_string());
            return;
        }

        let mut is_bot = self.is_bot;
        if let Some(rest) = token.strip_prefix("Bot ") {
            token = rest.trim().to_string();
            is_bot = true;
        }
        if let Some(rest) = token.strip_prefix("Bearer ") {
            token = rest.trim().to_string();
        }

        self.token = token.clone();
        self.is_bot = is_bot;
        self.busy = true;
        self.login_error = None;
        self.force_remember = false;
        let _ = self.commands.send(Command::Login { token, is_bot });
    }

    fn start_password_login(&mut self) {
        let email = self.login_email.trim().to_string();
        if email.is_empty() {
            self.login_error = Some("Digite seu e-mail ou número de telefone.".to_string());
            return;
        }
        if self.login_password.is_empty() {
            self.login_error = Some("Digite sua senha.".to_string());
            return;
        }

        self.busy = true;
        self.login_error = None;
        self.login_mfa = None;
        self.force_remember = true;
        let _ = self.commands.send(Command::LoginPassword {
            email,
            password: self.login_password.clone(),
        });
    }

    fn submit_mfa(&mut self) {
        let Some(ticket) = self.login_mfa.clone() else {
            return;
        };
        let code = self.login_code.trim().to_string();
        if code.len() < 6 {
            self.login_error = Some("Digite o código de 6 dígitos do app autenticador.".to_string());
            return;
        }

        self.busy = true;
        self.login_error = None;
        let _ = self.commands.send(Command::SubmitMfa { ticket, code });
    }

    fn start_qr_login(&mut self) {
        self.qr = None;
        self.qr_error = None;
        self.login_error = None;
        self.login_mfa = None;
        self.busy = true;
        self.force_remember = true;
        let _ = self.commands.send(Command::RemoteAuthStart);
    }

    fn cancel_qr(&mut self) {
        let _ = self.commands.send(Command::RemoteAuthCancel);
        self.qr = None;
        self.qr_error = None;
        self.busy = false;
        self.force_remember = false;
    }

    fn switch_login_mode(&mut self, mode: LoginMode) {
        if self.login_mode == mode || self.busy {
            return;
        }
        if self.login_mode == LoginMode::Qr {
            let _ = self.commands.send(Command::RemoteAuthCancel);
            self.qr = None;
            self.qr_error = None;
        }
        self.login_mode = mode;
        self.login_error = None;
        self.login_mfa = None;
        self.login_code.clear();
        if mode == LoginMode::Qr {
            self.start_qr_login();
        }
    }

    fn enter_demo(&mut self) {
        let data = demo::build();
        self.demo = true;
        self.screen = Screen::Main;
        self.busy = false;
        self.login_error = None;
        self.auto_login = false;
        self.me = Some(data.me);
        self.guilds = data.guilds;
        self.channels_by_guild = data.channels;
        self.members_by_guild = data.members;
        self.loaded_members = self.members_by_guild.keys().cloned().collect();
        self.loaded_guild_info = self.guilds.iter().map(|guild| guild.id.clone()).collect();
        self.member_error = None;
        self.loading_members = false;
        self.settings_open = false;
        self.dms = data.dms;
        self.demo_messages = data.messages;
        self.is_bot = false;
        self.gateway_status = "Modo demonstração".to_string();
        self.connected = true;
        self.ping = None;
        self.selected_guild = None;
        self.active = None;
        self.messages.clear();
        self.unread.clear();
        self.typing.clear();
        self.images.clear();

        if let Some(first) = self.guilds.first().map(|guild| guild.id.clone()) {
            self.select_guild(first);
        }
    }

    fn reset_to_login(&mut self) {
        self.screen = Screen::Login;
        self.busy = false;
        self.auto_login = false;
        self.login_error = None;
        self.me = None;
        self.demo = false;
        self.guilds.clear();
        self.channels_by_guild.clear();
        self.members_by_guild.clear();
        self.loaded_guild_info.clear();
        self.loaded_members.clear();
        self.member_error = None;
        self.loading_members = false;
        self.settings_open = false;
        self.emoji_picker_open = false;
        self.dms.clear();
        self.selected_guild = None;
        self.active = None;
        self.messages.clear();
        self.demo_messages.clear();
        self.drafts.clear();
        self.unread.clear();
        self.typing.clear();
        self.composer.clear();
        self.editing = None;
        self.pre_edit = None;
        self.images.clear();
        self.gateway_status = "Desconectado".to_string();
        self.connected = false;
        self.ping = None;
        self.toast = None;
        self.token.clear();
        self.remember = false;
        self.settings.remember = false;
        self.login_mode = LoginMode::Password;
        self.login_email.clear();
        self.login_password.clear();
        self.login_code.clear();
        self.login_mfa = None;
        self.qr = None;
        self.qr_error = None;
        self.force_remember = false;
        config::clear_token();
        config::save_settings(&self.settings);
    }

    fn logout(&mut self) {
        let _ = self.commands.send(Command::RemoteAuthCancel);
        let _ = self.commands.send(Command::Logout);
        self.reset_to_login();
    }

    fn select_guild(&mut self, guild_id: String) {
        if self.editing.is_some() {
            self.editing = None;
            self.composer = self.pre_edit.take().unwrap_or_default();
        }
        if self.view != View::Guild || self.selected_guild.as_deref() != Some(guild_id.as_str()) {
            if let Some(previous) = self.active.clone() {
                self.drafts
                    .insert(previous.id.clone(), std::mem::take(&mut self.composer));
            }
        }

        self.view = View::Guild;
        self.selected_guild = Some(guild_id.clone());
        self.active = None;
        self.messages.clear();
        self.editing = None;
        self.pre_edit = None;
        self.loading_messages = false;
        self.member_error = None;
        self.guild_name_edit = self
            .guilds
            .iter()
            .find(|guild| guild.id == guild_id)
            .map(|guild| guild.name.clone())
            .unwrap_or_default();

        if !self.demo {
            if !self.loaded_guild_info.contains(&guild_id) {
                let _ = self.commands.send(Command::LoadGuildInfo {
                    guild_id: guild_id.clone(),
                });
            }
            if !self.loaded_members.contains(&guild_id) {
                self.loading_members = true;
                let _ = self.commands.send(Command::LoadGuildMembers {
                    guild_id: guild_id.clone(),
                });
            }
        }

        if let Some(channels) = self.channels_by_guild.get(&guild_id).cloned() {
            self.loading_channels = false;
            self.pick_default_channel(&channels);
        } else if self.demo {
            self.loading_channels = false;
        } else {
            self.loading_channels = true;
            let _ = self
                .commands
                .send(Command::LoadChannels { guild_id });
        }
    }

    fn pick_default_channel(&mut self, channels: &[Channel]) {
        if let Some(active) = &self.active {
            if channels.iter().any(|channel| channel.id == active.id) {
                return;
            }
        }

        let next = channels
            .iter()
            .filter(|channel| channel.is_text())
            .min_by_key(|channel| channel.position)
            .cloned();

        if let Some(channel) = next {
            self.open_channel(channel);
        }
    }

    fn open_channel(&mut self, channel: Channel) {
        if let Some(previous) = &self.active {
            if previous.id == channel.id {
                self.active = Some(channel);
                return;
            }
        }

        if self.editing.is_some() {
            self.editing = None;
            self.composer = self.pre_edit.take().unwrap_or_default();
        }

        if let Some(previous) = self.active.clone() {
            self.drafts
                .insert(previous.id.clone(), std::mem::take(&mut self.composer));
        }

        self.unread.remove(&channel.id);
        self.composer = self.drafts.get(&channel.id).cloned().unwrap_or_default();
        self.focus_composer = true;
        self.messages.clear();
        self.loading_messages = false;
        self.scroll_to_bottom = true;
        self.active = Some(channel.clone());

        if self.demo {
            self.messages = self
                .demo_messages
                .get(&channel.id)
                .cloned()
                .unwrap_or_default();
        } else {
            self.loading_messages = true;
            let _ = self.commands.send(Command::LoadMessages {
                channel_id: channel.id,
            });
        }
    }

    fn open_dm_view(&mut self) {
        self.view = View::Dm;
        self.selected_guild = None;
        self.active = None;
        self.messages.clear();
        self.loading_channels = false;
        self.loading_messages = false;
        self.editing = None;
        self.pre_edit = None;

        if let Some(first) = self.dms.first().cloned() {
            self.open_channel(first);
        }
    }

    fn send_current(&mut self) {
        let Some(active) = self.active.clone() else {
            return;
        };
        let content = emoji::replace_shortcodes(self.composer.trim());
        if content.is_empty() {
            return;
        }

        if let Some(message_id) = self.editing.clone() {
            if self.demo {
                self.apply_demo_edit(&active.id, &message_id, &content);
            } else {
                let _ = self.commands.send(Command::EditMessage {
                    channel_id: active.id.clone(),
                    message_id,
                    content,
                });
            }
            self.editing = None;
            self.pre_edit = None;
            self.composer.clear();
            self.last_typing = None;
            return;
        }

        self.composer.clear();
        self.last_typing = Some(Instant::now());

        if self.demo {
            let id = format!("local-{}", self.next_local_id);
            self.next_local_id += 1;
            let message = Message {
                id,
                channel_id: active.id.clone(),
                author: self.me.clone().unwrap_or_default(),
                content,
                timestamp: Local::now().to_rfc3339(),
                edited_timestamp: None,
                attachments: Vec::new(),
                embeds: Vec::new(),
                pending: false,
            };
            self.messages.push(message.clone());
            self.trim_messages();
            self.demo_messages
                .entry(active.id.clone())
                .or_default()
                .push(message);
            self.scroll_to_bottom = true;
        } else {
            let _ = self.commands.send(Command::SendMessage {
                channel_id: active.id,
                content,
            });
        }
    }

    fn begin_edit(&mut self, message: Message) {
        self.pre_edit = Some(std::mem::take(&mut self.composer));
        self.composer = message.content;
        self.editing = Some(message.id);
        self.focus_composer = true;
    }

    fn cancel_edit(&mut self) {
        self.editing = None;
        self.composer = self.pre_edit.take().unwrap_or_default();
    }

    fn apply_demo_edit(&mut self, channel_id: &str, message_id: &str, content: &str) {
        let edited = Local::now().to_rfc3339();
        for message in &mut self.messages {
            if message.id == message_id {
                message.content = content.to_string();
                message.edited_timestamp = Some(edited.clone());
            }
        }
        if let Some(messages) = self.demo_messages.get_mut(channel_id) {
            for message in messages {
                if message.id == message_id {
                    message.content = content.to_string();
                    message.edited_timestamp = Some(edited.clone());
                }
            }
        }
    }

    fn apply_demo_delete(&mut self, channel_id: &str, message_id: &str) {
        self.messages.retain(|message| message.id != message_id);
        if let Some(messages) = self.demo_messages.get_mut(channel_id) {
            messages.retain(|message| message.id != message_id);
        }
    }

    fn request_typing(&mut self) {
        let Some(active) = &self.active else {
            return;
        };
        if self.demo || self.composer.trim().is_empty() || self.editing.is_some() {
            self.last_typing = None;
            return;
        }

        let should_send = self
            .last_typing
            .map(|last| last.elapsed() > Duration::from_secs(8))
            .unwrap_or(true);

        if should_send {
            self.last_typing = Some(Instant::now());
            let _ = self.commands.send(Command::Typing {
                channel_id: active.id.clone(),
            });
        }
    }

    fn typing_line(&self) -> String {
        let Some(active) = &self.active else {
            return String::new();
        };

        let now = Instant::now();
        let mut names: Vec<String> = self
            .typing
            .iter()
            .filter(|(channel_id, (_, when))| {
                channel_id.as_str() == active.id && now.duration_since(*when) < Duration::from_secs(8)
            })
            .map(|(_, (name, _))| name.clone())
            .collect();

        names.sort();
        names.dedup();

        match names.len() {
            0 => String::new(),
            1 => format!("{} está digitando...", names[0]),
            2 => format!("{} e {} estão digitando...", names[0], names[1]),
            _ => "Várias pessoas estão digitando...".to_string(),
        }
    }

    fn trim_messages(&mut self) {
        const MAX_MESSAGES: usize = 400;
        if self.messages.len() > MAX_MESSAGES {
            let excess = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..excess);
        }
    }

    fn update_ram(&mut self) {
        if self.last_ram_check.elapsed() >= Duration::from_secs(2) {
            self.ram_mb = config::process_ram_mb();
            self.last_ram_check = Instant::now();
        }
    }

    fn flush_image_requests(&mut self) {
        for url in self.images.take_requests() {
            let _ = self.commands.send(Command::FetchImage { url });
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events(ctx);
        self.update_ram();
        self.request_typing();

        // Mantém o app vivo e responsivo mesmo sem eventos de entrada.
        ctx.request_repaint_after(Duration::from_millis(250));

        match self.screen {
            Screen::Login => self.ui_login(ctx),
            Screen::Main => self.ui_main(ctx),
        }

        self.flush_image_requests();
    }
}

impl App {
    fn ui_login(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_DARKEST))
            .show(ctx, |ui| {
                let background = ui.max_rect();
                ui.painter().circle_filled(
                    background.left_top() + egui::vec2(30.0, -30.0),
                    360.0,
                    Color32::from_rgba_unmultiplied(88, 101, 242, 18),
                );
                ui.painter().circle_filled(
                    background.right_bottom() + egui::vec2(-20.0, 70.0),
                    320.0,
                    Color32::from_rgba_unmultiplied(235, 69, 158, 13),
                );

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space((ui.available_height() * 0.035).clamp(12.0, 32.0));
                        ui.vertical_centered(|ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(486.0, 0.0),
                                Layout::top_down(Align::Center),
                                |ui| {
                                    egui::Frame::none()
                                        .fill(Color32::from_rgb(49, 51, 56))
                                        .rounding(14.0)
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            Color32::from_rgb(60, 63, 70),
                                        ))
                                        .inner_margin(Margin::symmetric(30.0, 24.0))
                                        .show(ui, |ui| {
                                            ui.set_width(422.0);
                                            self.ui_login_card(ui);
                                        });
                                },
                            );

                            ui.add_space(14.0);
                            if ui
                                .link(
                                    RichText::new("Explorar no modo demonstração")
                                        .size(13.0)
                                        .color(theme::MUTED),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.enter_demo();
                            }
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!(
                                    "{:.0} MB de RAM • cliente nativo em Rust + egui",
                                    self.ram_mb
                                ))
                                .size(11.0)
                                .color(Color32::from_rgb(110, 116, 126)),
                            );
                            ui.add_space(18.0);
                        });
                    });
            });
    }

    fn ui_login_card(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            if let Some(logo) = &self.logo {
                ui.add(
                    egui::Image::new(egui::load::SizedTexture::new(
                        logo.id(),
                        Vec2::splat(64.0),
                    ))
                    .rounding(32.0),
                );
            } else {
                let (logo_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                let painter = ui.painter();
                painter.circle_filled(logo_rect.center(), 32.0, theme::ACCENT);
                painter.text(
                    logo_rect.center(),
                    Align2::CENTER_CENTER,
                    "DP",
                    FontId::proportional(23.0),
                    Color32::WHITE,
                );
            }
            ui.add_space(12.0);

            let (title, subtitle) = match self.login_mode {
                LoginMode::Password => (
                    "Boas-vindas de volta!",
                    "Estamos felizes em ver você de novo!",
                ),
                LoginMode::Qr => (
                    "Entrar com QR Code",
                    "Escaneie o código com o app do Discord no celular.",
                ),
                LoginMode::Token => (
                    "Entrar com token",
                    "Conta pessoal ou bot. O token fica só no seu computador.",
                ),
            };
            ui.label(RichText::new(title).size(25.0).strong().color(theme::TEXT));
            ui.add_space(3.0);
            ui.add(
                egui::Label::new(
                    RichText::new(subtitle)
                        .size(13.5)
                        .color(theme::MUTED),
                )
                .wrap(),
            );
        });

        ui.add_space(18.0);
        self.ui_login_tabs(ui);
        ui.add_space(18.0);

        match self.login_mode {
            LoginMode::Password => self.ui_login_password(ui),
            LoginMode::Qr => self.ui_login_qr(ui),
            LoginMode::Token => self.ui_login_token(ui),
        }

        ui.add_space(14.0);
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Cliente não oficial. O login usa as APIs do Discord e pode \
                         esbarrar em captcha ou verificação. Use por sua conta e risco.",
                    )
                    .size(10.5)
                    .color(Color32::from_rgb(138, 124, 84)),
                )
                .wrap(),
            );
        });
    }

    fn ui_login_tabs(&mut self, ui: &mut Ui) {
        let tabs = [
            (LoginMode::Password, "E-mail e senha"),
            (LoginMode::Qr, "QR Code"),
            (LoginMode::Token, "Token"),
        ];

        egui::Frame::none()
            .fill(theme::BG_DARKEST)
            .rounding(11.0)
            .inner_margin(Margin::same(4.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 8.0) / 3.0;
                    for (mode, label) in tabs {
                        let selected = self.login_mode == mode;
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
                        if response.clicked() && !selected && !self.busy {
                            self.switch_login_mode(mode);
                        }
                        let background = if selected {
                            theme::ACCENT
                        } else if response.hovered() && !self.busy {
                            theme::BG_HOVER
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(rect, Rounding::same(8.0), background);
                        let color = if selected {
                            Color32::WHITE
                        } else {
                            theme::MUTED
                        };
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(12.5),
                            color,
                        );
                    }
                });
            });
    }

    fn ui_login_password(&mut self, ui: &mut Ui) {
        if self.login_mfa.is_some() {
            egui::Frame::none()
                .fill(Color32::from_rgb(35, 39, 51))
                .rounding(7.0)
                .inner_margin(Margin::symmetric(12.0, 9.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Autenticação em dois fatores")
                            .size(12.5)
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.label(
                        RichText::new(
                            "Abra o app autenticador e digite o código de 6 dígitos.",
                        )
                        .size(11.5)
                        .color(theme::MUTED),
                    );
                });
            ui.add_space(12.0);

            login_field(ui, "CÓDIGO DE AUTENTICAÇÃO", &mut self.login_code, false, "login_mfa_code");
            ui.add_space(12.0);

            if let Some(message) = &self.login_error {
                login_error_box(ui, message);
            }

            let label = if self.busy { "Verificando..." } else { "Verificar código" };
            if ui
                .add_enabled(
                    !self.busy,
                    theme::accent_button(
                        RichText::new(label)
                            .size(15.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .min_size(Vec2::new(ui.available_width(), 42.0)),
                )
                .clicked()
            {
                self.submit_mfa();
            }

            ui.add_space(10.0);
            let mut back = false;
            ui.vertical_centered(|ui| {
                if login_link(ui, "Voltar para o login").clicked() {
                    back = true;
                }
            });
            if back {
                self.login_mfa = None;
                self.login_code.clear();
                self.login_error = None;
            }
            return;
        }

        login_field(
            ui,
            "E-MAIL OU NÚMERO DE TELEFONE",
            &mut self.login_email,
            false,
            "login_email",
        );
        ui.add_space(12.0);
        login_field(ui, "SENHA", &mut self.login_password, true, "login_password");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if login_link(ui, "Esqueceu sua senha?").clicked() {
                open_url("https://discord.com/reset-password");
            }
        });
        ui.add_space(12.0);

        if let Some(message) = &self.login_error {
            login_error_box(ui, message);
        }

        let label = if self.busy { "Entrando..." } else { "Entrar" };
        if ui
            .add_enabled(
                !self.busy,
                theme::accent_button(
                    RichText::new(label)
                        .size(15.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .min_size(Vec2::new(ui.available_width(), 42.0)),
            )
            .clicked()
        {
            self.start_password_login();
        }

        if self.busy {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(14.0));
                ui.label(
                    RichText::new("Falando com o Discord...")
                        .size(11.5)
                        .color(theme::MUTED),
                );
            });
        }

        let enter = ui.input(|input| input.key_pressed(Key::Enter));
        if enter && !self.busy {
            self.start_password_login();
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Precisa de uma conta?")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            if login_link(ui, "Registrar").clicked() {
                open_url("https://discord.com/register");
            }
        });
    }

    fn ui_login_qr(&mut self, ui: &mut Ui) {
        let mut retry = false;
        let mut cancel = false;

        if let Some(qr) = &self.qr {
            let remaining = qr
                .expires_at
                .saturating_duration_since(Instant::now())
                .as_secs();
            let expired = remaining == 0;

            ui.vertical_centered(|ui| {
                egui::Frame::none()
                    .fill(Color32::WHITE)
                    .rounding(12.0)
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        let side = 196.0;
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                        ui.painter().image(
                            qr.texture.id(),
                            rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            Color32::WHITE,
                        );
                    });

                ui.add_space(12.0);

                if let Some(name) = &qr.scanned {
                    ui.label(
                        RichText::new(format!("Escaneado por {name}"))
                            .size(13.5)
                            .strong()
                            .color(theme::GREEN),
                    );
                    ui.label(
                        RichText::new("Confirme o login no seu celular.")
                            .size(12.0)
                            .color(theme::MUTED),
                    );
                } else if expired {
                    ui.label(
                        RichText::new("Código expirado, gerando outro...")
                            .size(13.0)
                            .color(theme::YELLOW),
                    );
                } else {
                    ui.label(
                        RichText::new(format!(
                            "O código expira em {:01}:{:02}",
                            remaining / 60,
                            remaining % 60
                        ))
                        .size(12.5)
                        .color(theme::MUTED),
                    );
                }

                ui.add_space(10.0);
                if login_link(ui, "Gerar outro código").clicked() {
                    retry = true;
                }
                if login_link(ui, "Cancelar").clicked() {
                    cancel = true;
                }
            });

            ui.add_space(12.0);
        } else if let Some(message) = &self.qr_error {
            ui.vertical_centered(|ui| {
                login_error_box(ui, message);
                if ui
                    .add_enabled(
                        !self.busy,
                        theme::accent_button(
                            RichText::new("Gerar QR Code")
                                .size(14.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .min_size(Vec2::new(220.0, 40.0)),
                    )
                    .clicked()
                {
                    retry = true;
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new().size(22.0));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Gerando QR Code...")
                        .size(13.0)
                        .color(theme::MUTED),
                );
                ui.add_space(6.0);
                if login_link(ui, "Cancelar").clicked() {
                    cancel = true;
                }
            });
        }

        if self.qr.is_some() && self.qr_error.is_none() {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(40, 42, 48))
                .rounding(8.0)
                .inner_margin(Margin::symmetric(12.0, 9.0))
                .show(ui, |ui| {
                    for (index, step) in [
                        "Abra o Discord no celular",
                        "Toque em \"Escanear QR Code\"",
                        "Confirme o login",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.label(
                            RichText::new(format!("{}. {step}", index + 1))
                                .size(11.5)
                                .color(theme::MUTED),
                        );
                    }
                });
        }

        if retry {
            self.start_qr_login();
        } else if cancel {
            self.cancel_qr();
            self.switch_login_mode(LoginMode::Password);
        }
    }

    fn ui_login_token(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new("Token")
                .size(11.0)
                .strong()
                .color(theme::MUTED),
        );
        egui::Frame::none()
            .fill(theme::BG_DARKEST)
            .rounding(6.0)
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.add(
                    TextEdit::singleline(&mut self.token)
                        .password(true)
                        .hint_text("Cole o token da conta ou do bot")
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .id_source("login_token"),
                );
            });
        ui.add_space(8.0);
        ui.checkbox(&mut self.is_bot, "Este token é de um bot");
        ui.checkbox(
            &mut self.remember,
            "Lembrar neste computador (DPAPI)",
        );
        ui.add_space(14.0);

        if let Some(message) = &self.login_error {
            login_error_box(ui, message);
        }

        let label = if self.busy { "Conectando..." } else { "Conectar" };
        if ui
            .add_enabled(
                !self.busy,
                theme::accent_button(
                    RichText::new(label)
                        .size(15.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .min_size(Vec2::new(ui.available_width(), 42.0)),
            )
            .clicked()
        {
            self.start_login();
        }

        if self.busy {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(14.0));
                ui.label(
                    RichText::new("Validando token e carregando servidores...")
                        .size(11.5)
                        .color(theme::MUTED),
                );
            });
        }
    }

    fn ui_main(&mut self, ctx: &egui::Context) {
        self.ui_status_bar(ctx);
        self.ui_guild_rail(ctx);
        self.ui_channel_panel(ctx);

        if self.view == View::Guild && self.show_members && self.selected_guild.is_some() {
            self.ui_members_panel(ctx);
        }

        let (send, cancel) = self.ui_composer(ctx);
        self.ui_chat_header(ctx);
        self.ui_chat(ctx);
        self.ui_settings_window(ctx);
        self.ui_emoji_picker(ctx);

        if cancel {
            self.cancel_edit();
        }
        if send {
            self.send_current();
        }
    }

    fn ui_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(26.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 36, 40))
                    .inner_margin(Margin::symmetric(10.0, 0.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let dot_color = if self.demo {
                        theme::ACCENT
                    } else if self.connected {
                        theme::GREEN
                    } else if self.busy {
                        theme::YELLOW
                    } else {
                        theme::RED
                    };
                    status_dot(ui, dot_color);
                    ui.label(
                        RichText::new(&self.gateway_status)
                            .size(11.5)
                            .color(theme::MUTED),
                    );
                    if let Some(ping) = self.ping {
                        ui.label(RichText::new("•").size(11.0).color(theme::MUTED));
                        ui.label(
                            RichText::new(format!("{ping} ms"))
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("RAM {:.0} MB", self.ram_mb))
                                .size(11.5)
                                .color(theme::GREEN),
                        );
                        ui.label(
                            RichText::new("Rust + egui")
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    });
                });
            });
    }

    fn ui_guild_rail(&mut self, ctx: &egui::Context) {
        let mut clicked_guild: Option<String> = None;
        let mut clicked_dm = false;
        let guilds = self.guilds.clone();
        let selected = self.selected_guild.clone();
        let view = self.view;
        let show_dms = !self.is_bot;
        let logo_id = self.logo.as_ref().map(|logo| logo.id());

        egui::SidePanel::left("guild_rail")
            .exact_width(72.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_DARKEST)
                    .inner_margin(Margin {
                        left: 10.0,
                        right: 10.0,
                        top: 10.0,
                        bottom: 10.0,
                    }),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(logo_id) = logo_id {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::splat(46.0),
                                    Sense::hover(),
                                );
                                ui.painter().image(
                                    logo_id,
                                    rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                                response.on_hover_text("Discord Phoenix");
                                rail_separator(ui);
                            }
                            if show_dms && dm_button(ui, view == View::Dm) {
                                clicked_dm = true;
                            }
                            if show_dms {
                                rail_separator(ui);
                            }
                            ui.add_space(2.0);

                            for guild in &guilds {
                                let is_selected = view == View::Guild
                                    && selected.as_deref() == Some(guild.id.as_str());
                                if guild_button(ui, &mut self.images, guild, is_selected) {
                                    clicked_guild = Some(guild.id.clone());
                                }
                                ui.add_space(8.0);
                            }
                        });
                    });
            });

        if clicked_dm {
            self.open_dm_view();
        } else if let Some(guild_id) = clicked_guild {
            self.select_guild(guild_id);
        }
    }

    fn ui_channel_panel(&mut self, ctx: &egui::Context) {
        let mut clicked_channel: Option<Channel> = None;
        let mut logout = false;

        let view = self.view;
        let selected_guild = self.selected_guild.clone();
        let active_id = self.active.as_ref().map(|channel| channel.id.clone());
        let channels = selected_guild
            .as_ref()
            .and_then(|guild_id| self.channels_by_guild.get(guild_id))
            .cloned()
            .unwrap_or_default();
        let dms = self.dms.clone();
        let me = self.me.clone();
        let loading = self.loading_channels;
        let unread = self.unread.clone();
        let guild_name = selected_guild
            .as_ref()
            .and_then(|guild_id| self.guilds.iter().find(|guild| &guild.id == guild_id))
            .map(|guild| guild.name.clone());

        egui::SidePanel::left("channel_panel")
            .exact_width(240.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::BG_SIDEBAR))
            .show(ctx, |ui| {
                let title = if view == View::Guild {
                    guild_name.clone().unwrap_or_else(|| "Servidor".to_string())
                } else {
                    "Mensagens diretas".to_string()
                };

                egui::Frame::none()
                    .inner_margin(Margin {
                        left: 14.0,
                        right: 14.0,
                        top: 14.0,
                        bottom: 10.0,
                    })
                    .show(ui, |ui| {
                        ui.label(RichText::new(title).size(15.0).strong());
                    });
                ui.separator();

                let list_height = (ui.available_height() - 62.0).max(80.0);
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(list_height)
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        if view == View::Guild {
                            if loading && channels.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(10.0);
                                    ui.add(egui::Spinner::new().size(14.0));
                                    ui.label(
                                        RichText::new("Carregando canais...")
                                            .size(12.0)
                                            .color(theme::MUTED),
                                    );
                                });
                            } else if channels.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Nenhum canal de texto disponível.")
                                            .size(12.0)
                                            .color(theme::MUTED),
                                    );
                                });
                            } else if let Some(channel) = render_channel_list(
                                ui,
                                &channels,
                                active_id.as_deref(),
                                &unread,
                            ) {
                                clicked_channel = Some(channel);
                            }
                        } else {
                            for dm in &dms {
                                let selected = active_id.as_deref() == Some(dm.id.as_str());
                                if dm_row(ui, &mut self.images, dm, selected, unread.contains(&dm.id)) {
                                    clicked_channel = Some(dm.clone());
                                }
                                ui.add_space(2.0);
                            }
                            if dms.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Nenhuma mensagem direta.")
                                            .size(12.0)
                                            .color(theme::MUTED),
                                    );
                                });
                            }
                        }
                    });

                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 36, 40))
                    .inner_margin(Margin::symmetric(8.0, 7.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(me) = &me {
                                avatar(ui, &mut self.images, me, 32.0);
                                ui.add_space(2.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(me.display_name())
                                            .size(13.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(me.handle())
                                            .size(10.5)
                                            .color(theme::MUTED),
                                    );
                                });
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Sair").size(12.0).color(theme::RED),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    logout = true;
                                }
                            });
                        });
                    });
            });

        if let Some(channel) = clicked_channel {
            if channel.is_dm() {
                self.view = View::Dm;
                self.selected_guild = None;
            }
            self.open_channel(channel);
        }
        if logout {
            self.logout();
        }
    }
}

fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn stat_chip(ui: &mut Ui, text: String) {
    egui::Frame::none()
        .fill(theme::BG_INPUT)
        .rounding(5.0)
        .inner_margin(Margin::symmetric(7.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.5).color(theme::MUTED));
        });
}

fn member_row(
    ui: &mut Ui,
    images: &mut ImageCache,
    member: &Member,
    roles: &[crate::model::Role],
    is_owner: bool,
    is_me: bool,
) {
    let color = member
        .role_color(roles)
        .map(|(r, g, b)| Color32::from_rgb(r, g, b))
        .unwrap_or(theme::TEXT);

    ui.horizontal(|ui| {
        ui.add_space(4.0);
        avatar(ui, images, &member.user, 28.0);
        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let name = if is_me {
                    format!("{} (você)", member.display_name())
                } else {
                    member.display_name()
                };
                ui.add(
                    Label::new(RichText::new(name).size(12.5).color(color).strong())
                        .truncate(),
                );
                if is_owner {
                    ui.label(
                        RichText::new("DONO")
                            .size(8.0)
                            .strong()
                            .color(theme::YELLOW),
                    );
                } else if member.user.bot {
                    ui.label(
                        RichText::new("BOT")
                            .size(8.0)
                            .strong()
                            .color(theme::ACCENT),
                    );
                }
            });
            if member.nick.as_deref().is_some_and(|nick| !nick.is_empty()) {
                ui.add(
                    Label::new(
                        RichText::new(member.handle())
                            .size(9.5)
                            .color(theme::MUTED),
                    )
                    .truncate(),
                );
            }
        });
    });
    ui.add_space(2.0);
}

fn rail_separator(ui: &mut Ui) {
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(32.0, 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0_f32, Color32::from_rgb(60, 63, 70)),
    );
    ui.add_space(8.0);
}

fn guild_button(ui: &mut Ui, images: &mut ImageCache, guild: &Guild, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    let rounding = if selected { 16.0 } else { 24.0 };

    let fill = if selected {
        theme::color_for(&guild.id)
    } else if response.hovered() {
        theme::color_for(&guild.id).gamma_multiply(0.85)
    } else {
        theme::color_for(&guild.id).gamma_multiply(0.72)
    };
    painter.rect_filled(rect, Rounding::same(rounding), fill);

    let mut has_icon = false;
    if let Some(url) = guild.icon_url() {
        if let Some(draw) = images.get(&ctx, &url) {
            paint_rounded_texture(&painter, rect, draw.texture, Rounding::same(rounding));
            has_icon = true;
        }
    }

    if !has_icon {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            initials(&guild.name),
            FontId::proportional(17.0),
            Color32::WHITE,
        );
    }

    if selected {
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left() - 7.0, rect.center().y - 16.0),
                Vec2::new(4.0, 32.0),
            ),
            Rounding::same(2.0),
            Color32::WHITE,
        );
    }

    response.on_hover_text(guild.name.clone()).clicked()
}

fn dm_button(ui: &mut Ui, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
    let painter = ui.painter();
    let fill = if selected {
        theme::ACCENT
    } else if response.hovered() {
        theme::BG_HOVER
    } else {
        theme::BG_INPUT
    };
    painter.rect_filled(rect, Rounding::same(24.0), fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "DM",
        FontId::proportional(14.0),
        if selected {
            Color32::WHITE
        } else {
            theme::TEXT
        },
    );

    if selected {
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left() - 7.0, rect.center().y - 16.0),
                Vec2::new(4.0, 32.0),
            ),
            Rounding::same(2.0),
            Color32::WHITE,
        );
    }

    response.on_hover_text("Mensagens diretas").clicked()
}

fn render_channel_list(
    ui: &mut Ui,
    channels: &[Channel],
    active_id: Option<&str>,
    unread: &HashSet<String>,
) -> Option<Channel> {
    let mut clicked = None;

    let mut categories: Vec<&Channel> = channels
        .iter()
        .filter(|channel| channel.is_category())
        .collect();
    categories.sort_by_key(|channel| channel.position);

    for category in categories {
        let category_id = category.id.clone();
        let mut children: Vec<&Channel> = channels
            .iter()
            .filter(|channel| {
                channel.parent_id.as_deref() == Some(category_id.as_str())
                    && (channel.is_text() || channel.is_voice())
            })
            .collect();

        if children.is_empty() {
            continue;
        }
        children.sort_by_key(|channel| channel.position);

        ui.add_space(8.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            category
                .name
                .clone()
                .unwrap_or_default()
                .to_uppercase(),
            FontId::proportional(10.5),
            theme::MUTED,
        );

        for channel in children {
            if channel_row(
                ui,
                channel,
                active_id == Some(channel.id.as_str()),
                unread.contains(&channel.id),
                4.0,
            ) {
                clicked = Some(channel.clone());
            }
        }
    }

    let mut uncategorized: Vec<&Channel> = channels
        .iter()
        .filter(|channel| {
            channel.parent_id.is_none() && (channel.is_text() || channel.is_voice())
        })
        .collect();
    uncategorized.sort_by_key(|channel| channel.position);

    if !uncategorized.is_empty() {
        ui.add_space(6.0);
        for channel in uncategorized {
            if channel_row(
                ui,
                channel,
                active_id == Some(channel.id.as_str()),
                unread.contains(&channel.id),
                4.0,
            ) {
                clicked = Some(channel.clone());
            }
        }
    }

    clicked
}

fn channel_row(
    ui: &mut Ui,
    channel: &Channel,
    selected: bool,
    unread: bool,
    indent: f32,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let painter = ui.painter();

    if selected {
        painter.rect_filled(rect, Rounding::same(4.0), theme::BG_SELECTED);
    } else if response.hovered() && channel.is_text() {
        painter.rect_filled(rect, Rounding::same(4.0), theme::BG_HOVER);
    }

    let icon = if channel.is_voice() { "V" } else { "#" };
    let text_color = if selected {
        Color32::WHITE
    } else if channel.is_voice() {
        theme::MUTED
    } else {
        Color32::from_rgb(180, 185, 192)
    };

    painter.text(
        egui::pos2(rect.left() + 8.0 + indent, rect.center().y),
        Align2::LEFT_CENTER,
        format!("{icon}  {}", channel.display_name()),
        FontId::proportional(14.5),
        text_color,
    );

    if unread {
        painter.circle_filled(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            3.0,
            Color32::WHITE,
        );
    }

    let response = if channel.is_voice() {
        response.on_hover_text("Canais de voz ainda não são suportados nesta versão.")
    } else {
        response
    };

    response.clicked() && channel.is_text()
}

fn dm_row(
    ui: &mut Ui,
    images: &mut ImageCache,
    channel: &Channel,
    selected: bool,
    unread: bool,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::click());
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();

    if selected {
        painter.rect_filled(rect, Rounding::same(4.0), theme::BG_SELECTED);
    } else if response.hovered() {
        painter.rect_filled(rect, Rounding::same(4.0), theme::BG_HOVER);
    }

    let user = channel.recipients.first().cloned().unwrap_or_default();
    let avatar_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 23.0, rect.center().y),
        Vec2::splat(30.0),
    );
    draw_avatar(&ctx, &painter, images, avatar_rect, &user, 30.0);

    painter.text(
        egui::pos2(rect.left() + 45.0, rect.center().y),
        Align2::LEFT_CENTER,
        channel.display_name(),
        FontId::proportional(13.5),
        if selected {
            Color32::WHITE
        } else {
            theme::TEXT
        },
    );

    if unread {
        painter.circle_filled(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            3.0,
            Color32::WHITE,
        );
    }

    response.clicked()
}

fn avatar(ui: &mut Ui, images: &mut ImageCache, user: &User, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    draw_avatar(&ctx, &painter, images, rect, user, size);
}

fn draw_avatar(
    ctx: &egui::Context,
    painter: &egui::Painter,
    images: &mut ImageCache,
    rect: egui::Rect,
    user: &User,
    size: f32,
) {
    if let Some(url) = user.avatar_url() {
        if let Some(draw) = images.get(ctx, &url) {
            paint_rounded_texture(painter, rect, draw.texture, Rounding::same(size / 2.0));
            return;
        }
    }

    painter.circle_filled(rect.center(), size / 2.0, theme::color_for(&user.id));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initials(&user.display_name()),
        FontId::proportional(size * 0.38),
        Color32::WHITE,
    );
}

impl App {
    fn ui_chat_header(&mut self, ctx: &egui::Context) {
        let active = self.active.clone();
        let demo = self.demo;
        let guild_view = self.view == View::Guild && self.selected_guild.is_some();
        let members_open = self.show_members;
        let mut toggle_members = false;
        let mut open_settings = false;

        egui::TopBottomPanel::top("chat_header")
            .exact_height(48.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_CHAT)
                    .inner_margin(Margin {
                        left: 16.0,
                        right: 16.0,
                        top: 0.0,
                        bottom: 0.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(channel) = &active {
                        ui.label(
                            RichText::new(format!(
                                "{}{}",
                                channel.prefix(),
                                channel.display_name()
                            ))
                            .size(17.0)
                            .strong()
                            .color(theme::TEXT),
                        );

                        if let Some(topic) = &channel.topic {
                            if !topic.is_empty() {
                                ui.label(RichText::new("│").size(14.0).color(theme::MUTED));
                                ui.add(
                                    Label::new(
                                        RichText::new(topic).size(12.0).color(theme::MUTED),
                                    )
                                    .truncate(),
                                );
                            }
                        }

                        if demo {
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(theme::ACCENT.gamma_multiply(0.22))
                                .rounding(4.0)
                                .inner_margin(Margin::symmetric(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("DEMO")
                                            .size(9.5)
                                            .strong()
                                            .color(theme::ACCENT),
                                    );
                                });
                        }
                    } else {
                        ui.label(
                            RichText::new("Discord Phoenix")
                                .size(17.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                    }

                    if guild_view {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⚙").size(16.0).color(theme::MUTED),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Configurações do servidor")
                                .clicked()
                            {
                                open_settings = true;
                            }
                            ui.add_space(6.0);
                            if ui
                                .selectable_label(
                                    members_open,
                                    RichText::new("Membros").size(12.0),
                                )
                                .on_hover_text("Mostrar ou esconder a lista de membros")
                                .clicked()
                            {
                                toggle_members = true;
                            }
                        });
                    }
                });
            });

        if toggle_members {
            self.show_members = !self.show_members;
        }
        if open_settings {
            self.settings_open = true;
            self.guild_name_edit = self
                .selected_guild_info()
                .map(|guild| guild.name.clone())
                .unwrap_or_default();
        }
    }

    fn ui_members_panel(&mut self, ctx: &egui::Context) {
        let Some(guild_id) = self.selected_guild.clone() else {
            return;
        };

        let guild = self.selected_guild_info().cloned().unwrap_or_default();
        let roles = guild.roles.clone();
        let members = self
            .members_by_guild
            .get(&guild_id)
            .cloned()
            .unwrap_or_default();
        let loading = self.loading_members;
        let error = self.member_error.clone();
        let owner_id = guild.owner_id.clone();
        let total = guild.member_count.unwrap_or(members.len() as u32);
        let me_id = self.me.as_ref().map(|me| me.id.clone()).unwrap_or_default();
        let images = &mut self.images;
        let mut refresh = false;

        egui::SidePanel::right("members_panel")
            .exact_width(228.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_SIDEBAR)
                    .inner_margin(Margin {
                        left: 8.0,
                        right: 8.0,
                        top: 8.0,
                        bottom: 8.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("MEMBROS — {total}"))
                            .size(11.0)
                            .strong()
                            .color(theme::MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("↻").size(13.0).color(theme::MUTED),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Atualizar lista de membros")
                            .clicked()
                        {
                            refresh = true;
                        }
                    });
                });
                ui.add_space(4.0);

                if loading && members.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(13.0));
                        ui.label(
                            RichText::new("Carregando membros...")
                                .size(11.5)
                                .color(theme::MUTED),
                        );
                    });
                } else if let Some(error) = &error {
                    if members.is_empty() {
                        ui.label(
                            RichText::new(error)
                                .size(11.0)
                                .color(Color32::from_rgb(230, 150, 150)),
                        );
                    }
                }

                if !members.is_empty() {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for member in &members {
                                member_row(
                                    ui,
                                    images,
                                    member,
                                    &roles,
                                    owner_id.as_deref() == Some(member.id()),
                                    me_id.as_str() == member.id(),
                                );
                            }
                            ui.add_space(6.0);
                        });
                } else if !loading && error.is_none() {
                    ui.label(
                        RichText::new("Nenhum membro retornado pelo servidor.")
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                }
            });

        if refresh {
            self.refresh_members(&guild_id);
        }
    }

    fn refresh_members(&mut self, guild_id: &str) {
        self.loading_members = true;
        self.member_error = None;
        self.loaded_members.remove(guild_id);
        let _ = self.commands.send(Command::LoadGuildMembers {
            guild_id: guild_id.to_string(),
        });
    }

    fn rename_guild(&mut self, guild_id: String, name: String) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        self.guild_name_edit = name.clone();

        if self.demo {
            if let Some(guild) = self.guilds.iter_mut().find(|guild| guild.id == guild_id) {
                guild.name = name;
            }
        } else {
            let _ = self.commands.send(Command::UpdateGuildName {
                guild_id,
                name,
            });
        }
    }

    fn ui_settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let Some(guild_id) = self.selected_guild.clone() else {
            self.settings_open = false;
            return;
        };

        let guild = self.selected_guild_info().cloned().unwrap_or_default();
        let channels = self
            .channels_by_guild
            .get(&guild_id)
            .cloned()
            .unwrap_or_default();
        let text_channels = channels.iter().filter(|channel| channel.is_text()).count();
        let voice_channels = channels.iter().filter(|channel| channel.is_voice()).count();
        let members_len = self
            .members_by_guild
            .get(&guild_id)
            .map(|members| members.len())
            .unwrap_or(0);
        let member_error = self.member_error.clone();
        let owner_name = guild
            .owner_id
            .as_ref()
            .and_then(|owner_id| {
                self.members_by_guild
                    .get(&guild_id)
                    .and_then(|members| {
                        members
                            .iter()
                            .find(|member| member.id() == owner_id)
                            .map(Member::display_name)
                    })
            })
            .unwrap_or_else(|| {
                guild
                    .owner_id
                    .clone()
                    .unwrap_or_else(|| "desconhecido".to_string())
            });

        let mut open = self.settings_open;
        let mut rename = false;
        let mut refresh_members = false;
        let mut refresh_info = false;
        let mut close = false;

        egui::Window::new("Configurações do servidor")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .max_height(640.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                    ui.painter()
                        .circle_filled(rect.center(), 23.0, theme::color_for(&guild.id));
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        initials(&guild.name),
                        FontId::proportional(17.0),
                        Color32::WHITE,
                    );
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(if guild.name.is_empty() {
                                "Servidor"
                            } else {
                                &guild.name
                            })
                            .size(17.0)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(format!("ID {}", guild.id))
                                .size(10.5)
                                .color(theme::MUTED),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new("Nome do servidor")
                        .size(12.0)
                        .color(theme::MUTED),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.guild_name_edit)
                            .desired_width(300.0)
                            .hint_text("Nome do servidor"),
                    );
                    if ui
                        .add_enabled(
                            !self.guild_name_edit.trim().is_empty(),
                            theme::accent_button(RichText::new("Salvar nome").size(12.5)),
                        )
                        .clicked()
                    {
                        rename = true;
                    }
                });
                if self.demo {
                    ui.label(
                        RichText::new("No modo demonstração a alteração fica só neste app.")
                            .size(10.5)
                            .color(theme::MUTED),
                    );
                }

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    stat_chip(ui, format!("{} membros", guild.member_count.unwrap_or(members_len as u32)));
                    stat_chip(ui, format!("{text_channels} canais de texto"));
                    stat_chip(ui, format!("{voice_channels} canais de voz"));
                    stat_chip(ui, format!("{} cargos", guild.roles.len()));
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("Dono: {owner_name}"))
                        .size(12.0)
                        .color(theme::TEXT),
                );
                if let Some(description) = &guild.description {
                    if !description.trim().is_empty() {
                        ui.add_space(4.0);
                        ui.add(
                            Label::new(
                                RichText::new(description).size(12.0).color(theme::MUTED),
                            )
                            .wrap(),
                        );
                    }
                }

                ui.add_space(8.0);
                ui.checkbox(
                    &mut self.show_members,
                    "Mostrar lista de membros ao lado da conversa",
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(
                            RichText::new("Atualizar membros").size(12.0),
                        ))
                        .clicked()
                    {
                        refresh_members = true;
                    }
                    if ui
                        .add(egui::Button::new(
                            RichText::new("Atualizar dados do servidor").size(12.0),
                        ))
                        .clicked()
                    {
                        refresh_info = true;
                    }
                });

                if let Some(error) = &member_error {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(error)
                            .size(11.0)
                            .color(Color32::from_rgb(230, 150, 150)),
                    );
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    RichText::new("CARGOS")
                        .size(10.5)
                        .strong()
                        .color(theme::MUTED),
                );
                if guild.roles.is_empty() {
                    ui.label(
                        RichText::new(
                            "Nenhum cargo carregado. Clique em \"Atualizar dados do servidor\".",
                        )
                        .size(11.0)
                        .color(theme::MUTED),
                    );
                } else {
                    let mut roles = guild.roles.clone();
                    roles.sort_by_key(|role| std::cmp::Reverse(role.position));
                    for role in roles {
                        ui.horizontal(|ui| {
                            let color = role
                                .rgb()
                                .map(|(r, g, b)| Color32::from_rgb(r, g, b))
                                .unwrap_or(theme::MUTED);
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, color);
                            ui.label(
                                RichText::new(role.name)
                                    .size(12.0)
                                    .color(color),
                            );
                            if role.hoist {
                                ui.label(
                                    RichText::new("exibido separadamente")
                                        .size(9.5)
                                        .color(theme::MUTED),
                                );
                            }
                        });
                    }
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Fechar").clicked() {
                        close = true;
                    }
                });
            });

        if close {
            open = false;
        }
        self.settings_open = open;

        if rename {
            self.rename_guild(guild_id.clone(), self.guild_name_edit.clone());
        }
        if refresh_members {
            self.refresh_members(&guild_id);
        }
        if refresh_info {
            let _ = self.commands.send(Command::LoadGuildInfo {
                guild_id: guild_id.clone(),
            });
            self.loaded_guild_info.remove(&guild_id);
        }
    }

    fn ui_emoji_picker(&mut self, ctx: &egui::Context) {
        if !self.emoji_picker_open {
            return;
        }

        let mut open = self.emoji_picker_open;
        let mut picked: Option<&'static str> = None;

        egui::Window::new("Emojis")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(380.0)
            .anchor(Align2::RIGHT_BOTTOM, [-24.0, -74.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Buscar").size(12.0).color(theme::MUTED));
                    ui.add(
                        TextEdit::singleline(&mut self.emoji_query)
                            .hint_text(":smiley: ou nome")
                            .desired_width(220.0),
                    );
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Limpar").size(11.0).color(theme::MUTED),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.emoji_query.clear();
                    }
                });

                ScrollArea::horizontal()
                    .id_salt("emoji_groups")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.emoji_group.is_none(), "Todos")
                                .clicked()
                            {
                                self.emoji_group = None;
                            }
                            for group in emoji::GROUPS {
                                if ui
                                    .selectable_label(
                                        self.emoji_group == Some(*group),
                                        *group,
                                    )
                                    .clicked()
                                {
                                    self.emoji_group = Some(group);
                                }
                            }
                        });
                    });

                ui.add_space(2.0);
                let items = emoji::search(&self.emoji_query, self.emoji_group);
                ScrollArea::vertical()
                    .id_salt("emoji_scroll")
                    .max_height(220.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if items.is_empty() {
                            ui.label(
                                RichText::new("Nenhum emoji encontrado.")
                                    .size(11.5)
                                    .color(theme::MUTED),
                            );
                        }
                        egui::Grid::new("emoji_grid")
                            .num_columns(8)
                            .spacing([2.0, 2.0])
                            .show(ui, |ui| {
                                for (index, item) in items.iter().enumerate() {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(item.glyph).size(19.0),
                                            )
                                            .frame(false)
                                            .min_size(Vec2::splat(30.0)),
                                        )
                                        .on_hover_text(format!(":{}:", item.code))
                                        .clicked()
                                    {
                                        picked = Some(item.glyph);
                                    }
                                    if index % 8 == 7 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(
                        "Clique para inserir ou digite o atalho :codigo: e envie.",
                    )
                    .size(10.5)
                    .color(theme::MUTED),
                );
            });

        if let Some(glyph) = picked {
            self.insert_emoji(glyph);
        }
        self.emoji_picker_open = open;
    }

    fn insert_emoji(&mut self, glyph: &str) {
        if !self.composer.is_empty()
            && !self.composer.ends_with(char::is_whitespace)
        {
            self.composer.push(' ');
        }
        self.composer.push_str(glyph);
        self.focus_composer = true;
    }

    fn ui_chat(&mut self, ctx: &egui::Context) {
        let mut action: Option<MsgAction> = None;
        let me_id = self.me.as_ref().map(|me| me.id.clone()).unwrap_or_default();
        let active = self.active.clone();
        let toast = self.toast.clone();
        let loading = self.loading_messages;
        let scroll = self.scroll_to_bottom;
        let demo = self.demo;
        let total_messages = self.messages.len();
        let messages = std::mem::take(&mut self.messages);
        let images = &mut self.images;

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_CHAT))
            .show(ctx, |ui| {
                if let Some((text, when)) = &toast {
                    if when.elapsed() < Duration::from_secs(8) {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(78, 32, 35))
                            .rounding(6.0)
                            .inner_margin(Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(text)
                                        .size(12.0)
                                        .color(Color32::from_rgb(255, 190, 190)),
                                );
                            });
                        ui.add_space(6.0);
                    }
                }

                let Some(channel) = &active else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(90.0);
                        ui.label(
                            RichText::new("Selecione um canal")
                                .size(22.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Escolha um servidor à esquerda ou abra suas mensagens diretas.",
                            )
                            .size(13.0)
                            .color(theme::MUTED),
                        );
                    });
                    return;
                };

                if loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(90.0);
                        ui.add(egui::Spinner::new().size(26.0));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Carregando mensagens...")
                                .size(12.0)
                                .color(theme::MUTED),
                        );
                    });
                    return;
                }

                if total_messages == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(70.0);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(58.0), Sense::hover());
                        ui.painter()
                            .circle_filled(rect.center(), 29.0, theme::BG_SIDEBAR);
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            channel.prefix(),
                            FontId::proportional(26.0),
                            theme::MUTED,
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "Bem-vindo(a) a {}{}",
                                channel.prefix(),
                                channel.display_name()
                            ))
                            .size(20.0)
                            .strong()
                            .color(theme::TEXT),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("Este é o começo do canal. Diga olá!")
                                .size(13.0)
                                .color(theme::MUTED),
                        );
                    });
                    return;
                }

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(scroll)
                    .show(ui, |ui| {
                        ui.add_space(10.0);
                        ui.set_width(ui.available_width());
                        for message in &messages {
                            render_message(ui, message, &me_id, &mut action, images);
                        }
                        ui.add_space(12.0);
                    });
            });

        self.messages = messages;
        self.scroll_to_bottom = false;

        if let Some(action) = action {
            match action {
                MsgAction::Edit(message) => self.begin_edit(message),
                MsgAction::Delete(message) => {
                    if demo {
                        self.apply_demo_delete(&message.channel_id, &message.id);
                    } else {
                        let _ = self.commands.send(Command::DeleteMessage {
                            channel_id: message.channel_id,
                            message_id: message.id,
                        });
                    }
                }
            }
        }
    }

    fn ui_composer(&mut self, ctx: &egui::Context) -> (bool, bool) {
        let Some(active) = self.active.clone() else {
            return (false, false);
        };

        let mut send = false;
        let mut cancel = false;
        let typing = self.typing_line();
        let editing = self.editing.is_some();
        let demo = self.demo;

        egui::TopBottomPanel::bottom("composer_panel")
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_CHAT)
                    .inner_margin(Margin {
                        left: 16.0,
                        right: 16.0,
                        top: 6.0,
                        bottom: 12.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());

                if !typing.is_empty() {
                    ui.label(
                        RichText::new(typing)
                            .size(11.5)
                            .color(theme::MUTED),
                    );
                    ui.add_space(2.0);
                }

                if editing {
                    egui::Frame::none()
                        .fill(theme::BG_SIDEBAR)
                        .rounding(6.0)
                        .inner_margin(Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Editando mensagem")
                                        .size(11.5)
                                        .color(theme::YELLOW),
                                );
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Cancelar")
                                                .size(11.5)
                                                .color(theme::MUTED),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    cancel = true;
                                }
                            });
                        });
                    ui.add_space(4.0);
                }

                egui::Frame::none()
                    .fill(theme::BG_INPUT)
                    .rounding(8.0)
                    .inner_margin(Margin::symmetric(12.0, 7.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let hint = if editing {
                                "Salve ou cancele a edição...".to_string()
                            } else if demo {
                                format!(
                                    "Mensagem em {}{} (demonstração)",
                                    active.prefix(),
                                    active.display_name()
                                )
                            } else {
                                format!(
                                    "Mensagem em {}{}",
                                    active.prefix(),
                                    active.display_name()
                                )
                            };

                            let width = (ui.available_width() - 112.0).max(120.0);
                            let response = ui.add(
                                TextEdit::multiline(&mut self.composer)
                                    .frame(false)
                                    .desired_rows(1)
                                    .desired_width(width)
                                    .hint_text(hint),
                            );

                            if self.focus_composer {
                                response.request_focus();
                                self.focus_composer = false;
                            }

                            let enter = response.has_focus()
                                && ui.input(|input| {
                                    input.key_pressed(Key::Enter)
                                        && !input.modifiers.shift
                                        && !input.modifiers.ctrl
                                        && !input.modifiers.alt
                                });

                            if enter {
                                if self.composer.ends_with('\n') {
                                    self.composer.pop();
                                }
                                send = true;
                            }

                            let enabled = !self.composer.trim().is_empty();
                            let label = if editing { "Salvar" } else { "Enviar" };

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("😊").size(16.0),
                                    )
                                    .frame(false)
                                    .min_size(Vec2::new(26.0, 26.0)),
                                )
                                .on_hover_text("Escolher emoji")
                                .clicked()
                            {
                                self.emoji_picker_open = !self.emoji_picker_open;
                            }
                            ui.add_space(2.0);

                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(
                                        RichText::new(label)
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .rounding(6.0)
                                    .min_size(Vec2::new(64.0, 28.0)),
                                )
                                .clicked()
                            {
                                send = true;
                            }
                        });
                    });
            });

        (send, cancel)
    }
}

fn render_message(
    ui: &mut Ui,
    message: &Message,
    me_id: &str,
    action: &mut Option<MsgAction>,
    images: &mut ImageCache,
) {
    let row = ui
        .horizontal_top(|ui| {
            ui.add_space(12.0);
            avatar(ui, images, &message.author, 40.0);
            ui.add_space(2.0);

            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 14.0).max(120.0));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(message.author.display_name())
                            .size(14.5)
                            .strong()
                            .color(theme::name_color(&message.author.id)),
                    );

                    if message.author.bot {
                        egui::Frame::none()
                            .fill(theme::ACCENT)
                            .rounding(3.0)
                            .inner_margin(Margin::symmetric(4.0, 1.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("BOT")
                                        .size(8.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                            });
                    }

                    let time = message.time_label();
                    if !time.is_empty() {
                        ui.label(
                            RichText::new(time)
                                .size(10.5)
                                .color(theme::MUTED),
                        );
                    }
                    if message.edited_timestamp.is_some() {
                        ui.label(
                            RichText::new("(editado)")
                                .size(10.5)
                                .color(theme::MUTED),
                        );
                    }
                });

                let content = clean_content(&message.content);
                if !content.trim().is_empty() {
                    markdown::render(ui, &content);
                }

                render_direct_media(ui, images, &content, message);

                for attachment in &message.attachments {
                    if attachment.is_image() && !attachment.url.is_empty() {
                        render_attachment_image(ui, images, attachment);
                        continue;
                    }

                    let size = if attachment.size > 0 {
                        format!(
                            " ({:.1} MB)",
                            attachment.size as f64 / (1024.0 * 1024.0)
                        )
                    } else {
                        String::new()
                    };
                    let label = format!("[arquivo] {}{}", attachment.filename, size);
                    if attachment.url.is_empty() {
                        ui.label(RichText::new(label).size(12.5).color(theme::LINK));
                    } else {
                        ui.hyperlink_to(
                            RichText::new(label).size(12.5).color(theme::LINK),
                            &attachment.url,
                        );
                    }
                }

                for embed in &message.embeds {
                    render_embed(ui, images, embed);
                }
            });
        })
        .response;

    row.context_menu(|ui| {
        if ui.button("Copiar texto").clicked() {
            ui.ctx().copy_text(message.content.clone());
            ui.close_menu();
        }
        if message.author.id == me_id {
            if ui.button("Editar mensagem").clicked() {
                *action = Some(MsgAction::Edit(message.clone()));
                ui.close_menu();
            }
            if ui.button("Apagar mensagem").clicked() {
                *action = Some(MsgAction::Delete(message.clone()));
                ui.close_menu();
            }
        }
    });

    ui.add_space(6.0);
}

fn render_direct_media(ui: &mut Ui, images: &mut ImageCache, content: &str, message: &Message) {
    if !content.contains("http") {
        return;
    }

    let mut seen: HashSet<String> = HashSet::new();
    for attachment in &message.attachments {
        seen.insert(attachment.url.clone());
    }
    for embed in &message.embeds {
        if let Some(image) = embed.best_image() {
            seen.insert(image.url.clone());
        }
    }

    let mut shown = 0usize;
    for token in content.split_whitespace() {
        if shown >= 3 {
            break;
        }

        let url = token
            .trim_start_matches(['<', '(', '['])
            .trim_end_matches(['>', ')', ']', ',', ';', '!', '"', '\''])
            .trim_end_matches('.');
        if !url.starts_with("http") || !is_media_url(url) {
            continue;
        }
        if !seen.insert(url.to_string()) {
            continue;
        }

        shown += 1;
        if let Some(draw) = images.get(ui.ctx(), url) {
            let size = fit_size(draw.size, Vec2::new(400.0, 300.0));
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            paint_rounded_texture(ui.painter(), rect, draw.texture, Rounding::same(6.0));
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
            ui.add_space(4.0);
        }
    }
}

fn is_media_url(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|extension| path.ends_with(extension))
}

fn render_embed(ui: &mut Ui, images: &mut ImageCache, embed: &Embed) {
    if !embed.has_content() {
        return;
    }

    let color = embed
        .color
        .map(|rgb| {
            Color32::from_rgb(
                ((rgb >> 16) & 0xff) as u8,
                ((rgb >> 8) & 0xff) as u8,
                (rgb & 0xff) as u8,
            )
        })
        .unwrap_or(theme::ACCENT);

    egui::Frame::none()
        .fill(theme::BG_SIDEBAR)
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.75)))
        .rounding(6.0)
        .inner_margin(Margin {
            left: 11.0,
            right: 10.0,
            top: 8.0,
            bottom: 8.0,
        })
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 4.0).max(160.0));

            if let Some(provider) = &embed.provider {
                if !provider.name.trim().is_empty() {
                    ui.label(
                        RichText::new(&provider.name)
                            .size(9.5)
                            .color(theme::MUTED),
                    );
                }
            }

            if let Some(author) = &embed.author {
                if !author.name.trim().is_empty() {
                    ui.horizontal(|ui| {
                        if let Some(icon_url) = author
                            .icon_url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                        {
                            remote_icon(ui, images, icon_url, 16.0);
                        }
                        let name = RichText::new(&author.name)
                            .size(11.5)
                            .strong()
                            .color(theme::TEXT);
                        if let Some(url) = author
                            .url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                        {
                            ui.hyperlink_to(name, url);
                        } else {
                            ui.label(name);
                        }
                    });
                }
            }

            if let Some(title) = &embed.title {
                if !title.trim().is_empty() {
                    let title = RichText::new(title)
                        .size(14.0)
                        .strong()
                        .color(color);
                    if let Some(url) = embed.url.as_deref().filter(|url| !url.is_empty()) {
                        ui.hyperlink_to(title, url);
                    } else {
                        ui.label(title);
                    }
                }
            }

            if let Some(description) = &embed.description {
                if !description.trim().is_empty() {
                    ui.add_space(2.0);
                    markdown::render(ui, &clean_content(description));
                }
            }

            if !embed.fields.is_empty() {
                ui.add_space(4.0);
                let fields = &embed.fields;
                let mut index = 0;
                while index < fields.len() {
                    if fields[index].inline {
                        let mut group = Vec::new();
                        while index < fields.len() && fields[index].inline {
                            group.push(&fields[index]);
                            index += 1;
                        }
                        let columns = group.len().clamp(1, 3);
                        ui.columns(columns, |cols| {
                            for (slot, field) in group.iter().enumerate() {
                                embed_field(&mut cols[slot % columns], field);
                            }
                        });
                    } else {
                        embed_field(ui, &fields[index]);
                        index += 1;
                    }
                }
            }

            if let Some(image) = embed.best_image() {
                ui.add_space(5.0);
                render_embed_image(ui, images, image);
            }

            if let Some(footer) = &embed.footer {
                if !footer.text.trim().is_empty() {
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        if let Some(icon_url) = footer
                            .icon_url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                        {
                            remote_icon(ui, images, icon_url, 14.0);
                        }
                        ui.label(
                            RichText::new(&footer.text)
                                .size(10.0)
                                .color(theme::MUTED),
                        );
                    });
                }
            }
        });

    ui.add_space(4.0);
}

fn embed_field(ui: &mut Ui, field: &crate::model::EmbedField) {
    if !field.name.trim().is_empty() {
        ui.label(
            RichText::new(&field.name)
                .size(11.0)
                .strong()
                .color(theme::TEXT),
        );
    }
    if !field.value.trim().is_empty() {
        markdown::render(ui, &clean_content(&field.value));
    }
}

fn remote_icon(ui: &mut Ui, images: &mut ImageCache, url: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if let Some(draw) = images.get(ui.ctx(), url) {
        paint_rounded_texture(
            ui.painter(),
            rect,
            draw.texture,
            Rounding::same(size / 2.0),
        );
    }
}

fn render_attachment_image(ui: &mut Ui, images: &mut ImageCache, attachment: &Attachment) {
    let Some(draw) = images.get(ui.ctx(), &attachment.url) else {
        ui.label(
            RichText::new(format!("[imagem] {}", attachment.filename))
                .size(12.5)
                .color(theme::MUTED),
        );
        return;
    };

    let size = fit_size(draw.size, Vec2::new(400.0, 300.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    paint_rounded_texture(ui.painter(), rect, draw.texture, Rounding::same(6.0));

    if response.clicked() {
        ui.ctx()
            .open_url(egui::OpenUrl::new_tab(&attachment.url));
    }

    ui.label(
        RichText::new(format!("{}  •  clique para abrir", attachment.filename))
            .size(10.5)
            .color(theme::MUTED),
    );
    ui.add_space(4.0);
}

fn render_embed_image(ui: &mut Ui, images: &mut ImageCache, image: &EmbedImage) {
    let Some(draw) = images.get(ui.ctx(), &image.url) else {
        return;
    };

    let size = fit_size(draw.size, Vec2::new(400.0, 300.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    paint_rounded_texture(ui.painter(), rect, draw.texture, Rounding::same(6.0));

    if response.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(&image.url));
    }
    ui.add_space(4.0);
}

fn fit_size(natural: Vec2, max: Vec2) -> Vec2 {
    if natural.x <= 0.0 || natural.y <= 0.0 {
        return Vec2::new(1.0, 1.0);
    }
    let scale = (max.x / natural.x).min(max.y / natural.y).min(1.0);
    Vec2::new((natural.x * scale).max(1.0), (natural.y * scale).max(1.0))
}

// UI_SECTIONS

fn login_field(ui: &mut Ui, label: &str, value: &mut String, password: bool, id: &str) {
    ui.label(
        RichText::new(label)
            .size(10.5)
            .strong()
            .color(theme::MUTED),
    );
    ui.add_space(2.0);
    egui::Frame::none()
        .fill(theme::BG_DARKEST)
        .rounding(6.0)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 54)))
        .inner_margin(Margin::symmetric(10.0, 9.0))
        .show(ui, |ui| {
            ui.add(
                TextEdit::singleline(value)
                    .password(password)
                    .frame(false)
                    .desired_width(f32::INFINITY)
                    .id_source(id),
            );
        });
}

fn login_error_box(ui: &mut Ui, message: &str) {
    egui::Frame::none()
        .fill(Color32::from_rgb(78, 32, 35))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(message)
                        .size(12.0)
                        .color(Color32::from_rgb(255, 190, 190)),
                )
                .wrap(),
            );
        });
    ui.add_space(10.0);
}

fn login_link(ui: &mut Ui, text: &str) -> egui::Response {
    ui.link(RichText::new(text).size(12.5).color(theme::LINK))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn open_url(url: &str) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

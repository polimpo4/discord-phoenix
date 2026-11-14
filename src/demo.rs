use std::collections::HashMap;

use chrono::{Duration, Local};

use crate::model::{
    Attachment, Channel, Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage, Guild, Member,
    Message, Role, User,
};

pub struct DemoData {
    pub me: User,
    pub guilds: Vec<Guild>,
    pub channels: HashMap<String, Vec<Channel>>,
    pub dms: Vec<Channel>,
    pub messages: HashMap<String, Vec<Message>>,
    pub members: HashMap<String, Vec<Member>>,
}

pub fn build() -> DemoData {
    let me = User {
        id: "u0".to_string(),
        username: "dev".to_string(),
        global_name: Some("Você".to_string()),
        discriminator: Some("0".to_string()),
        avatar: None,
        bot: false,
    };

    let ana = User {
        id: "u1".to_string(),
        username: "ana".to_string(),
        global_name: Some("Ana".to_string()),
        discriminator: Some("0".to_string()),
        avatar: None,
        bot: false,
    };

    let lucas = User {
        id: "u2".to_string(),
        username: "lucas".to_string(),
        global_name: Some("Lucas".to_string()),
        discriminator: Some("0".to_string()),
        avatar: None,
        bot: false,
    };

    let bot = User {
        id: "u3".to_string(),
        username: "phoenix-bot".to_string(),
        global_name: Some("Phoenix Bot".to_string()),
        discriminator: Some("0".to_string()),
        avatar: None,
        bot: true,
    };

    let guilds = vec![
        Guild {
            id: "g1".to_string(),
            name: "Phoenix Lab".to_string(),
            icon: None,
            owner_id: Some("u0".to_string()),
            member_count: Some(4),
            description: Some(
                "Servidor de demonstração do Discord Phoenix: cliente nativo em Rust."
                    .to_string(),
            ),
            roles: vec![
                Role {
                    id: "r1".to_string(),
                    name: "Fundador".to_string(),
                    color: 0xE74C3C,
                    position: 3,
                    hoist: true,
                },
                Role {
                    id: "r2".to_string(),
                    name: "Rustacean".to_string(),
                    color: 0xDEA584,
                    position: 2,
                    hoist: true,
                },
                Role {
                    id: "r3".to_string(),
                    name: "Membro".to_string(),
                    color: 0x57F287,
                    position: 1,
                    hoist: false,
                },
            ],
        },
        Guild {
            id: "g2".to_string(),
            name: "Rust Brasil".to_string(),
            icon: None,
            owner_id: Some("u1".to_string()),
            member_count: Some(2),
            description: Some("Comunidade brasileira de Rust.".to_string()),
            roles: vec![Role {
                id: "r4".to_string(),
                name: "Moderador".to_string(),
                color: 0x5865F2,
                position: 1,
                hoist: true,
            }],
        },
    ];

    let mut channels: HashMap<String, Vec<Channel>> = HashMap::new();
    channels.insert(
        "g1".to_string(),
        vec![
            category("g1c0", "TEXTOS", 0),
            text("g1c1", "geral", 1, "g1c0", "Converse sobre qualquer coisa"),
            text(
                "g1c2",
                "projetos",
                2,
                "g1c0",
                "Mostre o que você está construindo",
            ),
            text("g1c3", "off-topic", 3, "g1c0", "Memes e conversa fiada"),
            category("g1c4", "VOZ", 4),
            voice("g1c5", "Sala principal", 5, "g1c4"),
            voice("g1c6", "Foco", 6, "g1c4"),
        ],
    );
    channels.insert(
        "g2".to_string(),
        vec![
            category("g2c0", "COMUNIDADE", 0),
            text(
                "g2c1",
                "boas-vindas",
                1,
                "g2c0",
                "Apresente-se para a comunidade",
            ),
            text("g2c2", "duvidas", 2, "g2c0", "Pergunte sem medo"),
            text("g2c3", "projetos", 3, "g2c0", "Projetos da comunidade"),
        ],
    );

    let dms = vec![
        Channel {
            id: "dm1".to_string(),
            kind: 1,
            name: None,
            position: 0,
            parent_id: None,
            topic: None,
            recipients: vec![ana.clone()],
            guild_id: None,
        },
        Channel {
            id: "dm2".to_string(),
            kind: 1,
            name: None,
            position: 1,
            parent_id: None,
            topic: None,
            recipients: vec![bot.clone()],
            guild_id: None,
        },
    ];

    let mut messages: HashMap<String, Vec<Message>> = HashMap::new();
    messages.insert(
        "g1c1".to_string(),
        vec![
            message(
                "m1",
                "g1c1",
                &ana,
                "Esse cliente é sério? Abriu usando menos de 100 MB aqui.",
                9,
            ),
            message(
                "m2",
                "g1c1",
                &me,
                "Sim! É Rust + egui, sem Electron e sem Chromium embutido.",
                8,
            ),
            message(
                "m3",
                "g1c1",
                &lucas,
                "A resposta é instantânea. Curti bastante.",
                7,
            ),
            message(
                "m4",
                "g1c1",
                &ana,
                "Testa o modo demonstração e depois conecta com seu token real.",
                6,
            ),
            message(
                "m18",
                "g1c1",
                &lucas,
                "Caracteres: áéíóú ç ãõ âê ô à ü → ← ↑ ↓ ★ ☺ ♥ ⚡ ⚙ ✓ │ ─ █ ▓",
                5,
            ),
            message(
                "m19",
                "g1c1",
                &ana,
                "Emojis: 😀 🎉 🚀 ❤️ 👍🏽 🔥 ✅ 👀 🤖 ✨",
                4,
            ),
            message(
                "m21",
                "g1c1",
                &ana,
                "**Markdown**: **negrito**, *itálico*, __sublinhado__, ~~riscado~~, `código`, ||spoiler||, [link](https://discord.com) e atalhos :smiley: :fire: :heart:",
                3,
            ),
            message(
                "m22",
                "g1c1",
                &lucas,
                "GIF direto na mensagem: https://upload.wikimedia.org/wikipedia/commons/2/2c/Rotating_earth_%28large%29.gif",
                3,
            ),
            embed_message(
                "m23",
                "g1c1",
                &bot,
                "Embed de GIF no estilo Klipy/Tenor, com campos e rodapé.",
                2,
            ),
            image_message(
                "m20",
                "g1c1",
                &bot,
                "Teste de imagens: PNG estático e GIF animado (limitado a 16 quadros).",
                1,
            ),
        ],
    );
    messages.insert(
        "g1c2".to_string(),
        vec![
            message(
                "m5",
                "g1c2",
                &me,
                "Estou portando meu bot para Rust. O consumo caiu de 380 MB para 24 MB.",
                42,
            ),
            message(
                "m6",
                "g1c2",
                &lucas,
                "Se precisar de voz, dá uma olhada no songbird.",
                40,
            ),
            message(
                "m7",
                "g1c2",
                &ana,
                "Depois posta o benchmark completo aqui.",
                38,
            ),
        ],
    );
    messages.insert(
        "g1c3".to_string(),
        vec![message(
            "m8",
            "g1c3",
            &bot,
            "Phoenix online. RAM atual: ridiculamente baixa.",
            20,
        )],
    );
    messages.insert(
        "g2c1".to_string(),
        vec![
            message(
                "m9",
                "g2c1",
                &ana,
                "Bem-vindo(a) ao Rust Brasil! Comece dizendo o que você desenvolve.",
                120,
            ),
            message(
                "m10",
                "g2c1",
                &lucas,
                "Trabalho com back-end em Axum e estou aprendendo egui.",
                115,
            ),
        ],
    );
    messages.insert(
        "g2c2".to_string(),
        vec![
            message(
                "m11",
                "g2c2",
                &lucas,
                "Qual crate vocês usam para WebSocket?",
                90,
            ),
            message("m12", "g2c2", &me, "tokio-tungstenite. Funciona muito bem.", 88),
        ],
    );
    messages.insert(
        "g2c3".to_string(),
        vec![message(
            "m13",
            "g2c3",
            &me,
            "Discord Phoenix nasceu aqui. Código aberto e sem Electron.",
            70,
        )],
    );
    messages.insert(
        "dm1".to_string(),
        vec![
            message("m14", "dm1", &ana, "Oi! Você conseguiu rodar o cliente?", 15),
            message("m15", "dm1", &me, "Consegui. Está muito mais leve.", 14),
            message("m16", "dm1", &ana, "Depois me manda um print!", 13),
        ],
    );
    messages.insert(
        "dm2".to_string(),
        vec![message(
            "m17",
            "dm2",
            &bot,
            "Comandos disponíveis: !ping, !ram, !demo",
            10,
        )],
    );

    let mut members: HashMap<String, Vec<Member>> = HashMap::new();
    members.insert(
        "g1".to_string(),
        vec![
            member(&me, Some("Você"), &["r1"]),
            member(&ana, None, &["r2", "r3"]),
            member(&lucas, Some("Lucas Dev"), &["r3"]),
            member(&bot, None, &[]),
        ],
    );
    members.insert(
        "g2".to_string(),
        vec![
            member(&ana, None, &["r4"]),
            member(&lucas, None, &[]),
        ],
    );

    DemoData {
        me,
        guilds,
        channels,
        dms,
        messages,
        members,
    }
}

fn member(user: &User, nick: Option<&str>, roles: &[&str]) -> Member {
    Member {
        user: user.clone(),
        nick: nick.map(str::to_string),
        roles: roles.iter().map(|role| role.to_string()).collect(),
        joined_at: None,
    }
}

fn category(id: &str, name: &str, position: i32) -> Channel {
    Channel {
        id: id.to_string(),
        kind: 4,
        name: Some(name.to_string()),
        position,
        parent_id: None,
        topic: None,
        recipients: Vec::new(),
        guild_id: None,
    }
}

fn text(id: &str, name: &str, position: i32, parent: &str, topic: &str) -> Channel {
    Channel {
        id: id.to_string(),
        kind: 0,
        name: Some(name.to_string()),
        position,
        parent_id: Some(parent.to_string()),
        topic: Some(topic.to_string()),
        recipients: Vec::new(),
        guild_id: None,
    }
}

fn voice(id: &str, name: &str, position: i32, parent: &str) -> Channel {
    Channel {
        id: id.to_string(),
        kind: 2,
        name: Some(name.to_string()),
        position,
        parent_id: Some(parent.to_string()),
        topic: None,
        recipients: Vec::new(),
        guild_id: None,
    }
}

fn message(
    id: &str,
    channel_id: &str,
    author: &User,
    content: &str,
    minutes_ago: i64,
) -> Message {
    let timestamp = (Local::now() - Duration::minutes(minutes_ago)).to_rfc3339();
    Message {
        id: id.to_string(),
        channel_id: channel_id.to_string(),
        author: author.clone(),
        content: content.to_string(),
        timestamp,
        edited_timestamp: None,
        attachments: Vec::new(),
        embeds: Vec::new(),
        pending: false,
    }
}

fn image_message(
    id: &str,
    channel_id: &str,
    author: &User,
    content: &str,
    minutes_ago: i64,
) -> Message {
    let mut message = message(id, channel_id, author, content, minutes_ago);
    message.attachments = vec![
        Attachment {
            filename: "avatar.png".to_string(),
            size: 0,
            url: "https://cdn.discordapp.com/embed/avatars/0.png".to_string(),
            content_type: Some("image/png".to_string()),
            width: Some(128),
            height: Some(128),
        },
        Attachment {
            filename: "planeta.gif".to_string(),
            size: 0,
            url: "https://upload.wikimedia.org/wikipedia/commons/2/2c/Rotating_earth_%28large%29.gif"
                .to_string(),
            content_type: Some("image/gif".to_string()),
            width: Some(400),
            height: Some(400),
        },
    ];
    message
}

fn embed_message(
    id: &str,
    channel_id: &str,
    author: &User,
    content: &str,
    minutes_ago: i64,
) -> Message {
    let mut message = message(id, channel_id, author, content, minutes_ago);
    let gif = "https://upload.wikimedia.org/wikipedia/commons/2/2c/Rotating_earth_%28large%29.gif"
        .to_string();
    message.embeds = vec![Embed {
        kind: Some("gifv".to_string()),
        title: Some("Planeta em rotação".to_string()),
        description: Some(
            "**GIF** vindo de um embed, no formato usado por Klipy e Tenor.\n\
             A miniatura animada é decodificada e exibida diretamente."
                .to_string(),
        ),
        url: Some("https://klipy.com/gifs/exemplo".to_string()),
        color: Some(0x5865F2),
        author: Some(EmbedAuthor {
            name: "Klipy".to_string(),
            url: Some("https://klipy.com".to_string()),
            icon_url: None,
        }),
        image: Some(EmbedImage {
            url: gif.clone(),
            content_type: Some("image/gif".to_string()),
            width: Some(400),
            height: Some(400),
        }),
        thumbnail: None,
        video: None,
        footer: Some(EmbedFooter {
            text: "Discord Phoenix • mídia renderizada no cliente".to_string(),
            icon_url: None,
        }),
        fields: vec![
            EmbedField {
                name: "Formato".to_string(),
                value: "GIF animado".to_string(),
                inline: true,
            },
            EmbedField {
                name: "Origem".to_string(),
                value: "Embed".to_string(),
                inline: true,
            },
        ],
        provider: None,
    }];
    message
}

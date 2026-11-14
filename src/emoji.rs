//! Emojis Unicode e atalhos no formato tradicional do Discord (`:smiley:`).
//!
//! O Discord não converte esses atalhos no servidor: quem converte é o cliente
//! antes de enviar. Fazemos o mesmo em [`replace_shortcodes`] e também
//! exibimos os atalhos conhecidos como emoji ao renderizar mensagens.

pub struct Emoji {
    pub code: &'static str,
    pub glyph: &'static str,
    pub group: &'static str,
}

pub const GROUPS: &[&str] = &[
    "Carinhas",
    "Gestos",
    "Corações",
    "Animais",
    "Comida",
    "Atividades",
    "Objetos",
    "Símbolos",
];

macro_rules! emoji {
    ($code:literal, $glyph:literal, $group:literal) => {
        Emoji {
            code: $code,
            glyph: $glyph,
            group: $group,
        }
    };
}

pub const EMOJIS: &[Emoji] = &[
    // Carinhas
    emoji!("smiley", "😃", "Carinhas"),
    emoji!("smile", "😄", "Carinhas"),
    emoji!("grin", "😁", "Carinhas"),
    emoji!("grinning", "😀", "Carinhas"),
    emoji!("laughing", "😆", "Carinhas"),
    emoji!("joy", "😂", "Carinhas"),
    emoji!("rofl", "🤣", "Carinhas"),
    emoji!("slightly_smiling_face", "🙂", "Carinhas"),
    emoji!("upside_down_face", "🙃", "Carinhas"),
    emoji!("wink", "😉", "Carinhas"),
    emoji!("blush", "😊", "Carinhas"),
    emoji!("innocent", "😇", "Carinhas"),
    emoji!("smiling_face_with_three_hearts", "🥰", "Carinhas"),
    emoji!("heart_eyes", "😍", "Carinhas"),
    emoji!("star_struck", "🤩", "Carinhas"),
    emoji!("kissing_heart", "😘", "Carinhas"),
    emoji!("kissing", "😗", "Carinhas"),
    emoji!("relaxed", "☺️", "Carinhas"),
    emoji!("yum", "😋", "Carinhas"),
    emoji!("stuck_out_tongue", "😛", "Carinhas"),
    emoji!("stuck_out_tongue_winking_eye", "😜", "Carinhas"),
    emoji!("stuck_out_tongue_closed_eyes", "😝", "Carinhas"),
    emoji!("money_mouth_face", "🤑", "Carinhas"),
    emoji!("hugs", "🤗", "Carinhas"),
    emoji!("thinking", "🤔", "Carinhas"),
    emoji!("face_with_raised_eyebrow", "🤨", "Carinhas"),
    emoji!("neutral_face", "😐", "Carinhas"),
    emoji!("expressionless", "😑", "Carinhas"),
    emoji!("no_mouth", "😶", "Carinhas"),
    emoji!("smirk", "😏", "Carinhas"),
    emoji!("unamused", "😒", "Carinhas"),
    emoji!("roll_eyes", "🙄", "Carinhas"),
    emoji!("grimacing", "😬", "Carinhas"),
    emoji!("lying_face", "🤥", "Carinhas"),
    emoji!("relieved", "😌", "Carinhas"),
    emoji!("pensive", "😔", "Carinhas"),
    emoji!("sleepy", "😪", "Carinhas"),
    emoji!("drooling_face", "🤤", "Carinhas"),
    emoji!("sleeping", "😴", "Carinhas"),
    emoji!("mask", "😷", "Carinhas"),
    emoji!("face_with_thermometer", "🤒", "Carinhas"),
    emoji!("face_with_head_bandage", "🤕", "Carinhas"),
    emoji!("nauseated_face", "🤢", "Carinhas"),
    emoji!("sneezing_face", "🤧", "Carinhas"),
    emoji!("hot_face", "🥵", "Carinhas"),
    emoji!("cold_face", "🥶", "Carinhas"),
    emoji!("woozy_face", "🥴", "Carinhas"),
    emoji!("dizzy_face", "😵", "Carinhas"),
    emoji!("exploding_head", "🤯", "Carinhas"),
    emoji!("cowboy_hat_face", "🤠", "Carinhas"),
    emoji!("partying_face", "🥳", "Carinhas"),
    emoji!("sunglasses", "😎", "Carinhas"),
    emoji!("nerd_face", "🤓", "Carinhas"),
    emoji!("monocle_face", "🧐", "Carinhas"),
    emoji!("confused", "😕", "Carinhas"),
    emoji!("worried", "😟", "Carinhas"),
    emoji!("slightly_frowning_face", "🙁", "Carinhas"),
    emoji!("frowning_face", "☹️", "Carinhas"),
    emoji!("open_mouth", "😮", "Carinhas"),
    emoji!("hushed", "😯", "Carinhas"),
    emoji!("astonished", "😲", "Carinhas"),
    emoji!("flushed", "😳", "Carinhas"),
    emoji!("pleading_face", "🥺", "Carinhas"),
    emoji!("fearful", "😨", "Carinhas"),
    emoji!("cold_sweat", "😰", "Carinhas"),
    emoji!("disappointed_relieved", "😥", "Carinhas"),
    emoji!("cry", "😢", "Carinhas"),
    emoji!("sob", "😭", "Carinhas"),
    emoji!("scream", "😱", "Carinhas"),
    emoji!("confounded", "😖", "Carinhas"),
    emoji!("persevere", "😣", "Carinhas"),
    emoji!("disappointed", "😞", "Carinhas"),
    emoji!("sweat", "😓", "Carinhas"),
    emoji!("weary", "😩", "Carinhas"),
    emoji!("tired_face", "😫", "Carinhas"),
    emoji!("yawning_face", "🥱", "Carinhas"),
    emoji!("triumph", "😤", "Carinhas"),
    emoji!("angry", "😠", "Carinhas"),
    emoji!("rage", "😡", "Carinhas"),
    emoji!("cursing_face", "🤬", "Carinhas"),
    emoji!("smiling_imp", "😈", "Carinhas"),
    emoji!("imp", "👿", "Carinhas"),
    emoji!("skull", "💀", "Carinhas"),
    emoji!("skull_and_crossbones", "☠️", "Carinhas"),
    emoji!("poop", "💩", "Carinhas"),
    emoji!("hankey", "💩", "Carinhas"),
    emoji!("clown_face", "🤡", "Carinhas"),
    emoji!("ghost", "👻", "Carinhas"),
    emoji!("alien", "👽", "Carinhas"),
    emoji!("robot", "🤖", "Carinhas"),
    emoji!("jack_o_lantern", "🎃", "Carinhas"),
    // Gestos
    emoji!("wave", "👋", "Gestos"),
    emoji!("raised_back_of_hand", "🤚", "Gestos"),
    emoji!("raised_hand", "✋", "Gestos"),
    emoji!("vulcan_salute", "🖖", "Gestos"),
    emoji!("ok_hand", "👌", "Gestos"),
    emoji!("pinched_fingers", "🤌", "Gestos"),
    emoji!("v", "✌️", "Gestos"),
    emoji!("crossed_fingers", "🤞", "Gestos"),
    emoji!("love_you_gesture", "🤟", "Gestos"),
    emoji!("metal", "🤘", "Gestos"),
    emoji!("call_me_hand", "🤙", "Gestos"),
    emoji!("point_left", "👈", "Gestos"),
    emoji!("point_right", "👉", "Gestos"),
    emoji!("point_up", "☝️", "Gestos"),
    emoji!("point_up_2", "👆", "Gestos"),
    emoji!("point_down", "👇", "Gestos"),
    emoji!("thumbsup", "👍", "Gestos"),
    emoji!("+1", "👍", "Gestos"),
    emoji!("thumbsdown", "👎", "Gestos"),
    emoji!("-1", "👎", "Gestos"),
    emoji!("fist", "✊", "Gestos"),
    emoji!("facepunch", "👊", "Gestos"),
    emoji!("punch", "👊", "Gestos"),
    emoji!("clap", "👏", "Gestos"),
    emoji!("raised_hands", "🙌", "Gestos"),
    emoji!("open_hands", "👐", "Gestos"),
    emoji!("palms_up_together", "🤲", "Gestos"),
    emoji!("handshake", "🤝", "Gestos"),
    emoji!("pray", "🙏", "Gestos"),
    emoji!("writing_hand", "✍️", "Gestos"),
    emoji!("nail_care", "💅", "Gestos"),
    emoji!("muscle", "💪", "Gestos"),
    emoji!("ear", "👂", "Gestos"),
    emoji!("nose", "👃", "Gestos"),
    emoji!("eyes", "👀", "Gestos"),
    emoji!("eye", "👁️", "Gestos"),
    emoji!("tongue", "👅", "Gestos"),
    emoji!("lips", "👄", "Gestos"),
    emoji!("brain", "🧠", "Gestos"),
    // Corações
    emoji!("heart", "❤️", "Corações"),
    emoji!("red_heart", "❤️", "Corações"),
    emoji!("orange_heart", "🧡", "Corações"),
    emoji!("yellow_heart", "💛", "Corações"),
    emoji!("green_heart", "💚", "Corações"),
    emoji!("blue_heart", "💙", "Corações"),
    emoji!("purple_heart", "💜", "Corações"),
    emoji!("black_heart", "🖤", "Corações"),
    emoji!("white_heart", "🤍", "Corações"),
    emoji!("brown_heart", "🤎", "Corações"),
    emoji!("broken_heart", "💔", "Corações"),
    emoji!("heartpulse", "💗", "Corações"),
    emoji!("sparkling_heart", "💖", "Corações"),
    emoji!("two_hearts", "💕", "Corações"),
    emoji!("revolving_hearts", "💞", "Corações"),
    emoji!("heartbeat", "💓", "Corações"),
    emoji!("heavy_heart_exclamation", "❣️", "Corações"),
    emoji!("cupid", "💘", "Corações"),
    emoji!("gift_heart", "💝", "Corações"),
    // Animais
    emoji!("cat", "🐱", "Animais"),
    emoji!("dog", "🐶", "Animais"),
    emoji!("mouse", "🐭", "Animais"),
    emoji!("hamster", "🐹", "Animais"),
    emoji!("rabbit", "🐰", "Animais"),
    emoji!("fox_face", "🦊", "Animais"),
    emoji!("bear", "🐻", "Animais"),
    emoji!("panda_face", "🐼", "Animais"),
    emoji!("koala", "🐨", "Animais"),
    emoji!("tiger", "🐯", "Animais"),
    emoji!("lion", "🦁", "Animais"),
    emoji!("cow", "🐮", "Animais"),
    emoji!("pig", "🐷", "Animais"),
    emoji!("frog", "🐸", "Animais"),
    emoji!("monkey", "🐵", "Animais"),
    emoji!("chicken", "🐔", "Animais"),
    emoji!("penguin", "🐧", "Animais"),
    emoji!("bird", "🐦", "Animais"),
    emoji!("baby_chick", "🐤", "Animais"),
    emoji!("unicorn", "🦄", "Animais"),
    emoji!("bee", "🐝", "Animais"),
    emoji!("bug", "🐛", "Animais"),
    emoji!("butterfly", "🦋", "Animais"),
    emoji!("snail", "🐌", "Animais"),
    emoji!("snake", "🐍", "Animais"),
    emoji!("turtle", "🐢", "Animais"),
    emoji!("fish", "🐟", "Animais"),
    emoji!("whale", "🐳", "Animais"),
    emoji!("dolphin", "🐬", "Animais"),
    emoji!("octopus", "🐙", "Animais"),
    emoji!("crab", "🦀", "Animais"),
    emoji!("shrimp", "🦐", "Animais"),
    // Comida
    emoji!("sushi", "🍣", "Comida"),
    emoji!("pizza", "🍕", "Comida"),
    emoji!("hamburger", "🍔", "Comida"),
    emoji!("fries", "🍟", "Comida"),
    emoji!("hotdog", "🌭", "Comida"),
    emoji!("taco", "🌮", "Comida"),
    emoji!("burrito", "🌯", "Comida"),
    emoji!("ramen", "🍜", "Comida"),
    emoji!("spaghetti", "🍝", "Comida"),
    emoji!("rice", "🍚", "Comida"),
    emoji!("curry", "🍛", "Comida"),
    emoji!("bento", "🍱", "Comida"),
    emoji!("dumpling", "🥟", "Comida"),
    emoji!("cookie", "🍪", "Comida"),
    emoji!("birthday", "🎂", "Comida"),
    emoji!("cake", "🍰", "Comida"),
    emoji!("cupcake", "🧁", "Comida"),
    emoji!("chocolate_bar", "🍫", "Comida"),
    emoji!("candy", "🍬", "Comida"),
    emoji!("lollipop", "🍭", "Comida"),
    emoji!("icecream", "🍦", "Comida"),
    emoji!("doughnut", "🍩", "Comida"),
    emoji!("coffee", "☕", "Comida"),
    emoji!("tea", "🍵", "Comida"),
    emoji!("beer", "🍺", "Comida"),
    emoji!("beers", "🍻", "Comida"),
    emoji!("wine_glass", "🍷", "Comida"),
    emoji!("cocktail", "🍸", "Comida"),
    emoji!("tropical_drink", "🍹", "Comida"),
    emoji!("apple", "🍎", "Comida"),
    emoji!("green_apple", "🍏", "Comida"),
    emoji!("pear", "🍐", "Comida"),
    emoji!("tangerine", "🍊", "Comida"),
    emoji!("lemon", "🍋", "Comida"),
    emoji!("cherries", "🍒", "Comida"),
    emoji!("grapes", "🍇", "Comida"),
    emoji!("watermelon", "🍉", "Comida"),
    emoji!("strawberry", "🍓", "Comida"),
    emoji!("peach", "🍑", "Comida"),
    emoji!("pineapple", "🍍", "Comida"),
    emoji!("banana", "🍌", "Comida"),
    emoji!("avocado", "🥑", "Comida"),
    emoji!("eggplant", "🍆", "Comida"),
    emoji!("tomato", "🍅", "Comida"),
    emoji!("corn", "🌽", "Comida"),
    emoji!("carrot", "🥕", "Comida"),
    emoji!("broccoli", "🥦", "Comida"),
    emoji!("mushroom", "🍄", "Comida"),
    emoji!("cheese", "🧀", "Comida"),
    emoji!("egg", "🥚", "Comida"),
    emoji!("bread", "🍞", "Comida"),
    emoji!("croissant", "🥐", "Comida"),
    emoji!("pancakes", "🥞", "Comida"),
    // Atividades
    emoji!("soccer", "⚽", "Atividades"),
    emoji!("basketball", "🏀", "Atividades"),
    emoji!("football", "🏈", "Atividades"),
    emoji!("baseball", "⚾", "Atividades"),
    emoji!("tennis", "🎾", "Atividades"),
    emoji!("volleyball", "🏐", "Atividades"),
    emoji!("8ball", "🎱", "Atividades"),
    emoji!("ping_pong", "🏓", "Atividades"),
    emoji!("badminton", "🏸", "Atividades"),
    emoji!("hockey", "🏒", "Atividades"),
    emoji!("cricket", "🏏", "Atividades"),
    emoji!("golf", "⛳", "Atividades"),
    emoji!("dart", "🎯", "Atividades"),
    emoji!("bowling", "🎳", "Atividades"),
    emoji!("video_game", "🎮", "Atividades"),
    emoji!("game_die", "🎲", "Atividades"),
    emoji!("dice", "🎲", "Atividades"),
    emoji!("chess_pawn", "♟️", "Atividades"),
    emoji!("slot_machine", "🎰", "Atividades"),
    emoji!("jigsaw", "🧩", "Atividades"),
    emoji!("trophy", "🏆", "Atividades"),
    emoji!("medal_sports", "🏅", "Atividades"),
    emoji!("first_place", "🥇", "Atividades"),
    emoji!("second_place", "🥈", "Atividades"),
    emoji!("third_place", "🥉", "Atividades"),
    emoji!("checkered_flag", "🏁", "Atividades"),
    emoji!("rocket", "🚀", "Atividades"),
    emoji!("airplane", "✈️", "Atividades"),
    emoji!("car", "🚗", "Atividades"),
    emoji!("taxi", "🚕", "Atividades"),
    emoji!("bus", "🚌", "Atividades"),
    emoji!("train", "🚆", "Atividades"),
    emoji!("ship", "🚢", "Atividades"),
    emoji!("anchor", "⚓", "Atividades"),
    emoji!("bicycle", "🚲", "Atividades"),
    emoji!("motorcycle", "🏍️", "Atividades"),
    emoji!("helicopter", "🚁", "Atividades"),
    emoji!("traffic_light", "🚥", "Atividades"),
    // Objetos
    emoji!("construction", "🚧", "Objetos"),
    emoji!("hammer", "🔨", "Objetos"),
    emoji!("wrench", "🔧", "Objetos"),
    emoji!("gear", "⚙️", "Objetos"),
    emoji!("nut_and_bolt", "🔩", "Objetos"),
    emoji!("link", "🔗", "Objetos"),
    emoji!("paperclip", "📎", "Objetos"),
    emoji!("scissors", "✂️", "Objetos"),
    emoji!("pencil2", "✏️", "Objetos"),
    emoji!("memo", "📝", "Objetos"),
    emoji!("books", "📚", "Objetos"),
    emoji!("bookmark", "🔖", "Objetos"),
    emoji!("book", "📖", "Objetos"),
    emoji!("newspaper", "📰", "Objetos"),
    emoji!("email", "📧", "Objetos"),
    emoji!("envelope", "✉️", "Objetos"),
    emoji!("inbox_tray", "📥", "Objetos"),
    emoji!("package", "📦", "Objetos"),
    emoji!("mailbox", "📫", "Objetos"),
    emoji!("keyboard", "⌨️", "Objetos"),
    emoji!("computer", "💻", "Objetos"),
    emoji!("desktop_computer", "🖥️", "Objetos"),
    emoji!("printer", "🖨️", "Objetos"),
    emoji!("computer_mouse", "🖱️", "Objetos"),
    emoji!("joystick", "🕹️", "Objetos"),
    emoji!("phone", "☎️", "Objetos"),
    emoji!("iphone", "📱", "Objetos"),
    emoji!("camera", "📷", "Objetos"),
    emoji!("video_camera", "📹", "Objetos"),
    emoji!("movie_camera", "🎥", "Objetos"),
    emoji!("television", "📺", "Objetos"),
    emoji!("radio", "📻", "Objetos"),
    emoji!("speaker", "🔈", "Objetos"),
    emoji!("sound", "🔉", "Objetos"),
    emoji!("loud_sound", "🔊", "Objetos"),
    emoji!("mute", "🔇", "Objetos"),
    emoji!("bell", "🔔", "Objetos"),
    emoji!("no_bell", "🔕", "Objetos"),
    emoji!("musical_note", "🎵", "Objetos"),
    emoji!("notes", "🎶", "Objetos"),
    emoji!("headphones", "🎧", "Objetos"),
    emoji!("microphone", "🎤", "Objetos"),
    emoji!("guitar", "🎸", "Objetos"),
    emoji!("violin", "🎻", "Objetos"),
    emoji!("trumpet", "🎺", "Objetos"),
    emoji!("drum", "🥁", "Objetos"),
    emoji!("saxophone", "🎷", "Objetos"),
    emoji!("piano", "🎹", "Objetos"),
    emoji!("lock", "🔒", "Objetos"),
    emoji!("unlock", "🔓", "Objetos"),
    emoji!("key", "🔑", "Objetos"),
    emoji!("old_key", "🗝️", "Objetos"),
    emoji!("shield", "🛡️", "Objetos"),
    emoji!("bulb", "💡", "Objetos"),
    emoji!("flashlight", "🔦", "Objetos"),
    emoji!("candle", "🕯️", "Objetos"),
    emoji!("moneybag", "💰", "Objetos"),
    emoji!("dollar", "💵", "Objetos"),
    emoji!("credit_card", "💳", "Objetos"),
    emoji!("gem", "💎", "Objetos"),
    emoji!("ring", "💍", "Objetos"),
    emoji!("crown", "👑", "Objetos"),
    emoji!("gift", "🎁", "Objetos"),
    emoji!("balloon", "🎈", "Objetos"),
    emoji!("tada", "🎉", "Objetos"),
    emoji!("confetti_ball", "🎊", "Objetos"),
    emoji!("christmas_tree", "🎄", "Objetos"),
    emoji!("snowman", "⛄", "Objetos"),
    // Símbolos
    emoji!("100", "💯", "Símbolos"),
    emoji!("anger", "💢", "Símbolos"),
    emoji!("boom", "💥", "Símbolos"),
    emoji!("collision", "💥", "Símbolos"),
    emoji!("fire", "🔥", "Símbolos"),
    emoji!("sparkles", "✨", "Símbolos"),
    emoji!("star", "⭐", "Símbolos"),
    emoji!("star2", "🌟", "Símbolos"),
    emoji!("dizzy", "💫", "Símbolos"),
    emoji!("zap", "⚡", "Símbolos"),
    emoji!("sun_with_face", "🌞", "Símbolos"),
    emoji!("rainbow", "🌈", "Símbolos"),
    emoji!("cloud", "☁️", "Símbolos"),
    emoji!("snowflake", "❄️", "Símbolos"),
    emoji!("umbrella", "☔", "Símbolos"),
    emoji!("droplet", "💧", "Símbolos"),
    emoji!("ocean", "🌊", "Símbolos"),
    emoji!("warning", "⚠️", "Símbolos"),
    emoji!("no_entry", "⛔", "Símbolos"),
    emoji!("question", "❓", "Símbolos"),
    emoji!("exclamation", "❗", "Símbolos"),
    emoji!("grey_question", "❔", "Símbolos"),
    emoji!("grey_exclamation", "❕", "Símbolos"),
    emoji!("heavy_check_mark", "✔️", "Símbolos"),
    emoji!("white_check_mark", "✅", "Símbolos"),
    emoji!("x", "❌", "Símbolos"),
    emoji!("heavy_multiplication_x", "✖️", "Símbolos"),
    emoji!("plus", "➕", "Símbolos"),
    emoji!("minus", "➖", "Símbolos"),
    emoji!("infinity", "♾️", "Símbolos"),
    emoji!("recycle", "♻️", "Símbolos"),
    emoji!("arrow_right", "➡️", "Símbolos"),
    emoji!("arrow_left", "⬅️", "Símbolos"),
    emoji!("arrow_up", "⬆️", "Símbolos"),
    emoji!("arrow_down", "⬇️", "Símbolos"),
    emoji!("arrows_clockwise", "🔃", "Símbolos"),
    emoji!("clock1", "🕐", "Símbolos"),
    emoji!("hourglass", "⌛", "Símbolos"),
    emoji!("alarm_clock", "⏰", "Símbolos"),
    emoji!("stopwatch", "⏱️", "Símbolos"),
    emoji!("calendar", "📅", "Símbolos"),
    emoji!("pushpin", "📌", "Símbolos"),
    emoji!("round_pushpin", "📍", "Símbolos"),
    emoji!("triangular_flag_on_post", "🚩", "Símbolos"),
    emoji!("white_flag", "🏳️", "Símbolos"),
    emoji!("rainbow_flag", "🏳️‍🌈", "Símbolos"),
    emoji!("pirate_flag", "🏴‍☠️", "Símbolos"),
];

/// Procura um atalho conhecido. Aceita com ou sem os dois-pontos.
pub fn find(code: &str) -> Option<&'static Emoji> {
    let code = code.trim_matches(':');
    EMOJIS.iter().find(|emoji| emoji.code == code)
}

/// Converte atalhos conhecidos (`:smiley:`) em emojis Unicode.
pub fn replace_shortcodes(input: &str) -> String {
    if !input.contains(':') {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find(':') {
        output.push_str(&rest[..start]);
        let tail = &rest[start + 1..];

        let Some(end) = tail.find(':') else {
            output.push(':');
            rest = tail;
            continue;
        };

        let code = &tail[..end];
        if !code.is_empty()
            && !code.chars().any(char::is_whitespace)
            && !code.contains('<')
            && !code.contains('>')
        {
            if let Some(emoji) = find(code) {
                output.push_str(emoji.glyph);
                rest = &tail[end + 1..];
                continue;
            }
        }

        output.push(':');
        rest = tail;
    }

    output.push_str(rest);
    output
}

/// Lista para o seletor de emojis, filtrada por grupo e busca.
pub fn search(query: &str, group: Option<&str>) -> Vec<&'static Emoji> {
    let query = query.trim().trim_matches(':').to_lowercase();
    let query = query.replace(' ', "_");

    EMOJIS
        .iter()
        .filter(|emoji| group.is_none_or(|group| emoji.group == group))
        .filter(|emoji| {
            query.is_empty()
                || emoji.code.contains(&query)
                || emoji.glyph == query
                || emoji.code.replace('_', " ").contains(&query)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_known_shortcodes_only() {
        assert_eq!(
            replace_shortcodes("oi :smiley: e :fire: :desconhecido:"),
            "oi 😃 e 🔥 :desconhecido:"
        );
        assert_eq!(replace_shortcodes("hora 12:30"), "hora 12:30");
    }

    #[test]
    fn searches_by_code_and_group() {
        assert!(search(":smiley:", None)
            .iter()
            .any(|emoji| emoji.code == "smiley"));
        let hearts = search("", Some("Corações"));
        assert!(!hearts.is_empty());
        assert!(hearts.iter().all(|emoji| emoji.group == "Corações"));
    }
}

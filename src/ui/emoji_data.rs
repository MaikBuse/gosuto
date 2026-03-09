#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiCategory {
    Smileys,
    People,
    Nature,
    Food,
    Activities,
    Travel,
    Objects,
    Symbols,
}

impl EmojiCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smileys => "Smileys",
            Self::People => "People",
            Self::Nature => "Nature",
            Self::Food => "Food",
            Self::Activities => "Activities",
            Self::Travel => "Travel",
            Self::Objects => "Objects",
            Self::Symbols => "Symbols",
        }
    }
}

pub struct EmojiEntry {
    pub emoji: &'static str,
    pub name: &'static str,
    pub category: EmojiCategory,
}

use EmojiCategory::*;

pub const EMOJIS: &[EmojiEntry] = &[
    // ── Smileys ──
    EmojiEntry {
        emoji: "\u{1F600}",
        name: "grinning face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F601}",
        name: "beaming face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F602}",
        name: "face with tears of joy",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F603}",
        name: "grinning face big eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F604}",
        name: "grinning face smiling eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F605}",
        name: "grinning face sweat",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F606}",
        name: "squinting face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F609}",
        name: "winking face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60A}",
        name: "smiling face blushing",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60B}",
        name: "face savoring food",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60C}",
        name: "relieved face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60D}",
        name: "smiling face heart eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60E}",
        name: "smiling face sunglasses",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F60F}",
        name: "smirking face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F610}",
        name: "neutral face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F611}",
        name: "expressionless face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F612}",
        name: "unamused face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F613}",
        name: "downcast face sweat",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F614}",
        name: "pensive face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F615}",
        name: "confused face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F616}",
        name: "confounded face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F617}",
        name: "kissing face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F618}",
        name: "face blowing kiss",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F61A}",
        name: "kissing face closed eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F61C}",
        name: "winking tongue face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F61D}",
        name: "squinting tongue face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F61E}",
        name: "disappointed face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F61F}",
        name: "worried face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F620}",
        name: "angry face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F621}",
        name: "pouting face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F622}",
        name: "crying face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F623}",
        name: "persevering face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F624}",
        name: "face with steam",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F625}",
        name: "sad but relieved",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F628}",
        name: "fearful face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F629}",
        name: "weary face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F62A}",
        name: "sleepy face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F62B}",
        name: "tired face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F62D}",
        name: "loudly crying face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F62E}",
        name: "face with open mouth",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F630}",
        name: "anxious face sweat",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F631}",
        name: "face screaming",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F632}",
        name: "astonished face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F633}",
        name: "flushed face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F634}",
        name: "sleeping face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F635}",
        name: "face with crossed eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F637}",
        name: "face with medical mask",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F641}",
        name: "slightly frowning face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F642}",
        name: "slightly smiling face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F643}",
        name: "upside down face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F644}",
        name: "face with rolling eyes",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F910}",
        name: "zipper mouth face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F911}",
        name: "money mouth face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F913}",
        name: "nerd face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F914}",
        name: "thinking face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F915}",
        name: "face with head bandage",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F917}",
        name: "hugging face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F920}",
        name: "cowboy hat face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F921}",
        name: "clown face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F922}",
        name: "nauseated face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F923}",
        name: "rolling on floor laughing",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F924}",
        name: "drooling face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F925}",
        name: "lying face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F929}",
        name: "star struck",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92A}",
        name: "zany face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92B}",
        name: "shushing face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92C}",
        name: "face with symbols mouth",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92D}",
        name: "face with hand over mouth",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92E}",
        name: "face vomiting",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F92F}",
        name: "exploding head",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F970}",
        name: "smiling face with hearts",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F973}",
        name: "partying face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F974}",
        name: "woozy face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F975}",
        name: "hot face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F976}",
        name: "cold face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1F97A}",
        name: "pleading face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1FAE0}",
        name: "melting face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1FAE1}",
        name: "saluting face",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1FAE2}",
        name: "face with open eyes hand over mouth",
        category: Smileys,
    },
    EmojiEntry {
        emoji: "\u{1FAE3}",
        name: "face with peeking eye",
        category: Smileys,
    },
    // ── People ──
    EmojiEntry {
        emoji: "\u{1F44D}",
        name: "thumbs up",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F44E}",
        name: "thumbs down",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F44F}",
        name: "clapping hands",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F44B}",
        name: "waving hand",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F44C}",
        name: "ok hand",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{270C}\u{FE0F}",
        name: "victory hand",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F91E}",
        name: "crossed fingers",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F91F}",
        name: "love you gesture",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F918}",
        name: "sign of the horns",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F919}",
        name: "call me hand",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F448}",
        name: "backhand index pointing left",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F449}",
        name: "backhand index pointing right",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F446}",
        name: "backhand index pointing up",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F447}",
        name: "backhand index pointing down",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{261D}\u{FE0F}",
        name: "index pointing up",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F4AA}",
        name: "flexed biceps",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F64F}",
        name: "folded hands",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F91D}",
        name: "handshake",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{270A}",
        name: "raised fist",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F44A}",
        name: "oncoming fist",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F590}\u{FE0F}",
        name: "hand with fingers splayed",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F440}",
        name: "eyes",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1F9E0}",
        name: "brain",
        category: People,
    },
    EmojiEntry {
        emoji: "\u{1FAF6}",
        name: "heart hands",
        category: People,
    },
    // ── Nature ──
    EmojiEntry {
        emoji: "\u{1F436}",
        name: "dog face",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F431}",
        name: "cat face",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F42D}",
        name: "mouse face",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F439}",
        name: "hamster",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F430}",
        name: "rabbit face",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F98A}",
        name: "fox",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F43B}",
        name: "bear",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F43C}",
        name: "panda",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F428}",
        name: "koala",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F42F}",
        name: "tiger face",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F981}",
        name: "lion",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F984}",
        name: "unicorn",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F41D}",
        name: "honeybee",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F427}",
        name: "penguin",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F433}",
        name: "spouting whale",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F40D}",
        name: "snake",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F339}",
        name: "rose",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F33B}",
        name: "sunflower",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F332}",
        name: "evergreen tree",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F340}",
        name: "four leaf clover",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F335}",
        name: "cactus",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F31F}",
        name: "glowing star",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{2B50}",
        name: "star",
        category: Nature,
    },
    EmojiEntry {
        emoji: "\u{1F308}",
        name: "rainbow",
        category: Nature,
    },
    // ── Food ──
    EmojiEntry {
        emoji: "\u{1F34E}",
        name: "red apple",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F34F}",
        name: "green apple",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F34A}",
        name: "tangerine",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F34B}",
        name: "lemon",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F34C}",
        name: "banana",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F349}",
        name: "watermelon",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F353}",
        name: "strawberry",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F352}",
        name: "cherries",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F351}",
        name: "peach",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F355}",
        name: "pizza",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F354}",
        name: "hamburger",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F35F}",
        name: "french fries",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F32E}",
        name: "taco",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F363}",
        name: "sushi",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F370}",
        name: "shortcake",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F36B}",
        name: "chocolate bar",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F369}",
        name: "doughnut",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F36A}",
        name: "cookie",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{2615}",
        name: "hot beverage",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F37A}",
        name: "beer mug",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F377}",
        name: "wine glass",
        category: Food,
    },
    EmojiEntry {
        emoji: "\u{1F37E}",
        name: "bottle with popping cork",
        category: Food,
    },
    // ── Activities ──
    EmojiEntry {
        emoji: "\u{26BD}",
        name: "soccer ball",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3C0}",
        name: "basketball",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3C8}",
        name: "american football",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{26BE}",
        name: "baseball",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3BE}",
        name: "tennis",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3B3}",
        name: "bowling",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3AE}",
        name: "video game",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3B2}",
        name: "game die",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3AF}",
        name: "direct hit",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3B5}",
        name: "musical note",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3B6}",
        name: "musical notes",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3A4}",
        name: "microphone",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3AC}",
        name: "clapper board",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3A8}",
        name: "artist palette",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F3C6}",
        name: "trophy",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F396}\u{FE0F}",
        name: "military medal",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F389}",
        name: "party popper",
        category: Activities,
    },
    EmojiEntry {
        emoji: "\u{1F38A}",
        name: "confetti ball",
        category: Activities,
    },
    // ── Travel ──
    EmojiEntry {
        emoji: "\u{1F697}",
        name: "automobile",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F695}",
        name: "taxi",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F68C}",
        name: "bus",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F680}",
        name: "rocket",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{2708}\u{FE0F}",
        name: "airplane",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F6F8}",
        name: "flying saucer",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F3E0}",
        name: "house",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F3D4}\u{FE0F}",
        name: "snow capped mountain",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F3D6}\u{FE0F}",
        name: "beach with umbrella",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F30D}",
        name: "globe europe africa",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F30E}",
        name: "globe americas",
        category: Travel,
    },
    EmojiEntry {
        emoji: "\u{1F30F}",
        name: "globe asia australia",
        category: Travel,
    },
    // ── Objects ──
    EmojiEntry {
        emoji: "\u{1F4A1}",
        name: "light bulb",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4BB}",
        name: "laptop",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4F1}",
        name: "mobile phone",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4E7}",
        name: "email",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4DA}",
        name: "books",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4D6}",
        name: "open book",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F512}",
        name: "locked",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F513}",
        name: "unlocked",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F527}",
        name: "wrench",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F528}",
        name: "hammer",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{2699}\u{FE0F}",
        name: "gear",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4B0}",
        name: "money bag",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4A3}",
        name: "bomb",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4A4}",
        name: "zzz",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4A5}",
        name: "collision",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4A8}",
        name: "dashing away",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4AC}",
        name: "speech balloon",
        category: Objects,
    },
    EmojiEntry {
        emoji: "\u{1F4AD}",
        name: "thought balloon",
        category: Objects,
    },
    // ── Symbols ──
    EmojiEntry {
        emoji: "\u{2764}\u{FE0F}",
        name: "red heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F9E1}",
        name: "orange heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F49B}",
        name: "yellow heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F49A}",
        name: "green heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F499}",
        name: "blue heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F49C}",
        name: "purple heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F5A4}",
        name: "black heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F90D}",
        name: "white heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F494}",
        name: "broken heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F495}",
        name: "two hearts",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F496}",
        name: "sparkling heart",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F49E}",
        name: "revolving hearts",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F48B}",
        name: "kiss mark",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F4AF}",
        name: "hundred points",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{2705}",
        name: "check mark",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{274C}",
        name: "cross mark",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{2753}",
        name: "question mark",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{2757}",
        name: "exclamation mark",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F525}",
        name: "fire",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F4A2}",
        name: "anger symbol",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F4AB}",
        name: "dizzy",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{2728}",
        name: "sparkles",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F6AB}",
        name: "prohibited",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{267B}\u{FE0F}",
        name: "recycling symbol",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F198}",
        name: "sos button",
        category: Symbols,
    },
    EmojiEntry {
        emoji: "\u{1F4A9}",
        name: "pile of poo",
        category: Symbols,
    },
];

pub fn filtered_emojis(filter: &str) -> Vec<&'static EmojiEntry> {
    if filter.is_empty() {
        return EMOJIS.iter().collect();
    }
    let lower = filter.to_lowercase();
    EMOJIS.iter().filter(|e| e.name.contains(&lower)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_empty_returns_all() {
        assert_eq!(filtered_emojis("").len(), EMOJIS.len());
    }

    #[test]
    fn filter_matches_substring() {
        let results = filtered_emojis("grin");
        assert!(!results.is_empty());
        for e in &results {
            assert!(e.name.contains("grin"));
        }
    }

    #[test]
    fn filter_case_insensitive() {
        let lower = filtered_emojis("heart");
        let upper = filtered_emojis("Heart");
        assert_eq!(lower.len(), upper.len());
    }

    #[test]
    fn filter_no_match() {
        let results = filtered_emojis("xyznonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn emojis_sorted_by_category() {
        // Verify categories appear in blocks (same category emojis grouped together)
        let mut last_cat = None;
        let mut seen_cats = Vec::new();
        for e in EMOJIS {
            if last_cat != Some(e.category) {
                assert!(
                    !seen_cats.contains(&e.category),
                    "Category {:?} appeared in non-contiguous blocks",
                    e.category
                );
                seen_cats.push(e.category);
                last_cat = Some(e.category);
            }
        }
    }
}

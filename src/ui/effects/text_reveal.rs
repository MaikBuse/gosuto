use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::Xorshift64;

const SCRAMBLE_CHARS: &[char] = &[
    '░', '▒', '▓', '◊', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', '0', '1', 'F', '█', '▌', '╌',
];

const SCRAMBLE_PHASE_MS: u64 = 300;
const REVEAL_MS_PER_CHAR: u64 = 40;
const MAX_DURATION_MS: u64 = 3000;

pub struct TextReveal {
    rng: Xorshift64,
    elapsed_ms: u64,
    scramble_seed: u64,
    active: bool,
}

impl TextReveal {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Xorshift64::new(seed),
            elapsed_ms: 0,
            scramble_seed: 0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.elapsed_ms = 0;
        self.active = true;
    }

    pub fn tick(&mut self, dt_ms: u64) {
        if !self.active {
            return;
        }
        self.elapsed_ms += dt_ms;
        self.scramble_seed = self.rng.next();
        if self.elapsed_ms >= MAX_DURATION_MS {
            self.active = false;
        }
    }

    /// Returns a styled `Line` with per-character scramble/reveal effect.
    /// When inactive, returns a single Span fast-path.
    pub fn render_line(&self, text: &str, style: Style) -> Line<'static> {
        if !self.active {
            return Line::from(Span::styled(text.to_owned(), style));
        }

        let mut scramble_rng = Xorshift64::new(self.scramble_seed);
        let chars: Vec<char> = text.chars().collect();
        let mut char_index: usize = 0;

        let spans: Vec<Span<'static>> = chars
            .into_iter()
            .map(|ch| {
                if ch == ' ' {
                    Span::styled(" ".to_owned(), style)
                } else {
                    let revealed = self.elapsed_ms >= SCRAMBLE_PHASE_MS
                        && (char_index as u64)
                            < (self.elapsed_ms - SCRAMBLE_PHASE_MS) / REVEAL_MS_PER_CHAR;
                    char_index += 1;
                    let display = if revealed {
                        ch
                    } else {
                        SCRAMBLE_CHARS[(scramble_rng.next() % SCRAMBLE_CHARS.len() as u64) as usize]
                    };
                    Span::styled(display.to_string(), style)
                }
            })
            .collect();

        Line::from(spans)
    }

    /// Returns the text after scramble/reveal as a `String` (for gradient title use).
    pub fn revealed_text(&self, text: &str) -> String {
        self.render_chars(text).into_iter().collect()
    }

    /// Returns scrambled/revealed chars for direct buffer writes (used by TransmissionPopup).
    pub fn render_chars(&self, text: &str) -> Vec<char> {
        if !self.active {
            return text.chars().collect();
        }

        let mut scramble_rng = Xorshift64::new(self.scramble_seed);
        let mut char_index: usize = 0;

        text.chars()
            .map(|ch| {
                if ch == ' ' {
                    ' '
                } else {
                    let revealed = self.elapsed_ms >= SCRAMBLE_PHASE_MS
                        && (char_index as u64)
                            < (self.elapsed_ms - SCRAMBLE_PHASE_MS) / REVEAL_MS_PER_CHAR;
                    char_index += 1;
                    if revealed {
                        ch
                    } else {
                        SCRAMBLE_CHARS[(scramble_rng.next() % SCRAMBLE_CHARS.len() as u64) as usize]
                    }
                }
            })
            .collect()
    }
}

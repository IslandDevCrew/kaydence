//! Clean: RawFinal + dial + profile -> CleanFinal.
//!
//! This is the dependency-free floor of the cleanup dial: deterministic Light
//! rules that never call a model, never block injection, and never author new
//! content. The constrained LLM pass and Full profile rewrite plug in later
//! behind this floor; Raw still bypasses the module entirely at the pipeline
//! boundary.

use crate::events::{CleanupDial, SessionEvent, SessionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanedTranscript {
    pub text: String,
    pub dial: CleanupDial,
}

/// Clean transcript text for the selected dial. Raw is returned untouched so
/// callers can use this helper in previews/tests; the runtime pipeline uses
/// [`clean_final_event`] so Raw emits no cleanup event.
pub fn clean_text(raw: &str, dial: CleanupDial) -> CleanedTranscript {
    match dial {
        CleanupDial::Raw => CleanedTranscript {
            text: raw.to_string(),
            dial,
        },
        CleanupDial::Light | CleanupDial::Full => CleanedTranscript {
            text: RuleCleaner.clean(raw),
            dial,
        },
    }
}

/// Convert a `RawFinal` payload into a `CleanFinal` event. Raw mode bypasses
/// cleanup entirely, preserving the event contract and the user's verbatim text.
pub fn clean_final_event(id: SessionId, raw: &str, dial: CleanupDial) -> Option<SessionEvent> {
    if dial == CleanupDial::Raw {
        return None;
    }

    let cleaned = clean_text(raw, dial);
    Some(SessionEvent::CleanFinal {
        id,
        text: cleaned.text,
        dial: cleaned.dial,
    })
}

#[derive(Debug, Default)]
struct RuleCleaner;

impl RuleCleaner {
    fn clean(&self, raw: &str) -> String {
        let words = raw.split_whitespace().collect::<Vec<_>>();
        let mut tokens = Vec::new();
        let mut index = 0;

        while index < words.len() {
            if phrase_at(&words, index, &["you", "know"]) {
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["i", "mean"]) {
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["sort", "of"]) {
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["kind", "of"]) {
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["no", "wait"]) {
                drop_trailing_replaced_span(&mut tokens);
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["scratch", "that"]) {
                drop_current_clause(&mut tokens);
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["new", "paragraph"]) {
                push_line_break(&mut tokens);
                push_line_break(&mut tokens);
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["new", "line"]) {
                push_line_break(&mut tokens);
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["question", "mark"]) {
                push_punctuation(&mut tokens, '?');
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["exclamation", "mark"]) {
                push_punctuation(&mut tokens, '!');
                index += 2;
                continue;
            }
            if phrase_at(&words, index, &["full", "stop"]) {
                push_punctuation(&mut tokens, '.');
                index += 2;
                continue;
            }

            let canonical = canonical_word(words[index]);
            if canonical.is_empty() || is_single_filler(&canonical) {
                index += 1;
                continue;
            }
            if canonical == "actually" {
                drop_trailing_replaced_span(&mut tokens);
                index += 1;
                continue;
            }
            if let Some(punctuation) = punctuation_command(&canonical) {
                push_punctuation(&mut tokens, punctuation);
                index += 1;
                continue;
            }

            push_word(&mut tokens, words[index]);
            index += 1;
        }

        finish_sentence(render_tokens(&tokens))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Punctuation(char),
    LineBreak,
}

fn phrase_at(words: &[&str], index: usize, phrase: &[&str]) -> bool {
    words
        .get(index..index + phrase.len())
        .is_some_and(|candidate| {
            candidate
                .iter()
                .map(|word| canonical_word(word))
                .eq(phrase.iter().map(|word| (*word).to_string()))
        })
}

fn canonical_word(word: &str) -> String {
    word.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'')
        .to_ascii_lowercase()
}

fn display_word(word: &str) -> Option<String> {
    let trimmed = word.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'');
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn is_single_filler(word: &str) -> bool {
    matches!(word, "um" | "uh" | "er" | "erm" | "ah")
}

fn punctuation_command(word: &str) -> Option<char> {
    match word {
        "comma" => Some(','),
        "period" => Some('.'),
        "colon" => Some(':'),
        "semicolon" => Some(';'),
        _ => None,
    }
}

fn push_word(tokens: &mut Vec<Token>, raw: &str) {
    let Some(word) = display_word(raw) else {
        return;
    };
    if last_word(tokens).is_some_and(|last| last.eq_ignore_ascii_case(&word)) {
        return;
    }
    tokens.push(Token::Word(word));
    for punctuation in raw
        .chars()
        .rev()
        .take_while(|ch| matches!(ch, '.' | ',' | '?' | '!' | ':' | ';'))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        push_punctuation(tokens, punctuation);
    }
}

fn last_word(tokens: &[Token]) -> Option<&str> {
    tokens.iter().rev().find_map(|token| match token {
        Token::Word(word) => Some(word.as_str()),
        Token::Punctuation(_) | Token::LineBreak => None,
    })
}

fn push_punctuation(tokens: &mut Vec<Token>, punctuation: char) {
    while matches!(tokens.last(), Some(Token::Punctuation(_))) {
        tokens.pop();
    }
    if matches!(tokens.last(), Some(Token::Word(_))) {
        tokens.push(Token::Punctuation(punctuation));
    }
}

fn push_line_break(tokens: &mut Vec<Token>) {
    while matches!(tokens.last(), Some(Token::Punctuation(',' | ':' | ';'))) {
        tokens.pop();
    }
    if !matches!(tokens.last(), Some(Token::LineBreak) | None) {
        tokens.push(Token::LineBreak);
    }
}

fn drop_current_clause(tokens: &mut Vec<Token>) {
    while let Some(token) = tokens.pop() {
        if matches!(
            token,
            Token::Punctuation('.')
                | Token::Punctuation('!')
                | Token::Punctuation('?')
                | Token::LineBreak
        ) {
            tokens.push(token);
            break;
        }
    }
}

fn drop_trailing_replaced_span(tokens: &mut Vec<Token>) {
    while matches!(tokens.last(), Some(Token::Punctuation(',' | ':' | ';'))) {
        tokens.pop();
    }

    let last_word_index = tokens
        .iter()
        .rposition(|token| matches!(token, Token::Word(_)));
    let Some(last_word_index) = last_word_index else {
        return;
    };

    let anchor_index = tokens[..last_word_index]
        .iter()
        .rposition(|token| match token {
            Token::Word(word) => is_replacement_anchor(&word.to_ascii_lowercase()),
            Token::Punctuation('.')
            | Token::Punctuation('!')
            | Token::Punctuation('?')
            | Token::LineBreak => false,
            Token::Punctuation(_) => false,
        });

    let truncate_at = anchor_index.map_or(last_word_index, |anchor| anchor + 1);
    tokens.truncate(truncate_at);
}

fn is_replacement_anchor(word: &str) -> bool {
    matches!(
        word,
        "to" | "for" | "on" | "at" | "is" | "are" | "was" | "were" | "be" | "as" | "with"
    )
}

fn render_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;

    for token in tokens {
        match token {
            Token::Word(word) => {
                if needs_space_before_word(&out) {
                    out.push(' ');
                }
                if capitalize_next {
                    out.push_str(&capitalize_first(word));
                    capitalize_next = false;
                } else {
                    out.push_str(word);
                }
            }
            Token::Punctuation(punctuation) => {
                trim_trailing_spaces(&mut out);
                out.push(*punctuation);
                capitalize_next = matches!(punctuation, '.' | '!' | '?');
            }
            Token::LineBreak => {
                trim_trailing_spaces(&mut out);
                if !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }
                capitalize_next = true;
            }
        }
    }

    out
}

fn needs_space_before_word(out: &str) -> bool {
    !out.is_empty() && !out.ends_with([' ', '\n'])
}

fn trim_trailing_spaces(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn finish_sentence(mut text: String) -> String {
    trim_trailing_spaces(&mut text);
    if text.is_empty() || text.ends_with(['.', '!', '?', '\n']) {
        text
    } else {
        text.push('.');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    fn sid() -> SessionId {
        SessionId::new(Ulid::new())
    }

    #[test]
    fn raw_mode_returns_untouched_text_and_emits_no_clean_final() {
        let raw = "um leave this exactly as spoken";

        assert_eq!(
            clean_text(raw, CleanupDial::Raw),
            CleanedTranscript {
                text: raw.to_string(),
                dial: CleanupDial::Raw
            }
        );
        assert_eq!(clean_final_event(sid(), raw, CleanupDial::Raw), None);
    }

    #[test]
    fn light_removes_fillers_dedupes_and_formats_punctuation() {
        let cleaned = clean_text(
            "um send send the build comma you know then ship it period",
            CleanupDial::Light,
        );

        assert_eq!(
            cleaned,
            CleanedTranscript {
                text: "Send the build, then ship it.".to_string(),
                dial: CleanupDial::Light
            }
        );
    }

    #[test]
    fn light_collapses_simple_self_correction_markers() {
        let cleaned = clean_text("schedule it for Tuesday no wait Friday", CleanupDial::Light);

        assert_eq!(cleaned.text, "Schedule it for Friday.");
    }

    #[test]
    fn light_handles_spoken_newlines() {
        let cleaned = clean_text("first line new line second line", CleanupDial::Light);

        assert_eq!(cleaned.text, "First line\nSecond line.");
    }

    #[test]
    fn full_currently_uses_the_same_rule_floor_without_authoring() {
        let cleaned = clean_text("uh write write the note period", CleanupDial::Full);

        assert_eq!(
            cleaned,
            CleanedTranscript {
                text: "Write the note.".to_string(),
                dial: CleanupDial::Full
            }
        );
    }

    #[test]
    fn clean_final_event_preserves_session_id_and_dial() {
        let id = sid();

        assert_eq!(
            clean_final_event(id, "hello comma world", CleanupDial::Light),
            Some(SessionEvent::CleanFinal {
                id,
                text: "Hello, world.".to_string(),
                dial: CleanupDial::Light,
            })
        );
    }
}

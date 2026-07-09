//! Personalize: term boosting, post-cleanup replacements, and snippets.
//!
//! This module owns deterministic text personalization after cleanup. It never
//! learns silently; correction learning and import preview/commit flows plug in
//! later on top of the same data model.

use crate::events::SessionEvent;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySource {
    BuiltIn,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryTerm {
    pub spoken_form: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub source: DictionarySource,
}

impl DictionaryTerm {
    pub fn built_in(spoken_form: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            spoken_form: spoken_form.into(),
            replacement: replacement.into(),
            case_sensitive: false,
            source: DictionarySource::BuiltIn,
        }
    }

    pub fn user(spoken_form: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            spoken_form: spoken_form.into(),
            replacement: replacement.into(),
            case_sensitive: false,
            source: DictionarySource::User,
        }
    }

    pub fn case_sensitive(mut self) -> Self {
        self.case_sensitive = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub trigger: String,
    pub expansion: String,
}

impl Snippet {
    pub fn exact(trigger: impl Into<String>, expansion: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            expansion: expansion.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DictionaryPass {
    terms: Vec<DictionaryTerm>,
    snippets: Vec<Snippet>,
}

impl DictionaryPass {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_terms(mut self, terms: Vec<DictionaryTerm>) -> Self {
        self.terms = terms;
        self
    }

    pub fn with_snippets(mut self, snippets: Vec<Snippet>) -> Self {
        self.snippets = snippets;
        self
    }

    pub fn apply_text(&self, text: &str) -> String {
        let candidates = self.candidates();
        if candidates.is_empty() {
            return text.to_string();
        }
        apply_candidates(text, &candidates)
    }

    pub fn apply_event(&self, event: SessionEvent) -> SessionEvent {
        match event {
            SessionEvent::CleanFinal { id, text, dial } => SessionEvent::CleanFinal {
                id,
                text: self.apply_text(&text),
                dial,
            },
            other => other,
        }
    }

    pub fn bias_hints(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut hints = Vec::new();
        for term in &self.terms {
            for candidate in [&term.spoken_form, &term.replacement] {
                let trimmed = candidate.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
                    hints.push(trimmed.to_string());
                }
            }
        }
        hints
    }

    fn candidates(&self) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for snippet in &self.snippets {
            let trigger = snippet.trigger.trim();
            if trigger.is_empty() || snippet.expansion.is_empty() {
                continue;
            }
            candidates.push(Candidate {
                pattern: trigger.to_string(),
                replacement: snippet.expansion.clone(),
                case_sensitive: true,
                source: DictionarySource::User,
                kind: CandidateKind::Snippet,
            });
        }
        for term in &self.terms {
            let spoken_form = term.spoken_form.trim();
            if spoken_form.is_empty() || term.replacement.is_empty() {
                continue;
            }
            candidates.push(Candidate {
                pattern: spoken_form.to_string(),
                replacement: term.replacement.clone(),
                case_sensitive: term.case_sensitive,
                source: term.source,
                kind: CandidateKind::Term,
            });
        }
        candidates.sort_by(candidate_order);
        candidates
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateKind {
    Snippet,
    Term,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    pattern: String,
    replacement: String,
    case_sensitive: bool,
    source: DictionarySource,
    kind: CandidateKind,
}

fn candidate_order(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    right
        .pattern
        .chars()
        .count()
        .cmp(&left.pattern.chars().count())
        .then_with(|| source_rank(left.source).cmp(&source_rank(right.source)))
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
}

fn source_rank(source: DictionarySource) -> u8 {
    match source {
        DictionarySource::User => 0,
        DictionarySource::BuiltIn => 1,
    }
}

fn kind_rank(kind: CandidateKind) -> u8 {
    match kind {
        CandidateKind::Snippet => 0,
        CandidateKind::Term => 1,
    }
}

fn apply_candidates(text: &str, candidates: &[Candidate]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| matches_at(text, index, candidate))
        {
            out.push_str(&candidate.replacement);
            index += candidate.pattern.len();
            continue;
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn matches_at(text: &str, index: usize, candidate: &Candidate) -> bool {
    let end = index + candidate.pattern.len();
    let Some(slice) = text.get(index..end) else {
        return false;
    };
    if !candidate.case_sensitive && !slice.eq_ignore_ascii_case(&candidate.pattern) {
        return false;
    }
    if candidate.case_sensitive && slice != candidate.pattern {
        return false;
    }
    has_left_boundary(text, index) && has_right_boundary(text, end)
}

fn has_left_boundary(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    text[..index]
        .chars()
        .next_back()
        .map_or(true, |ch| !is_word_char(ch))
}

fn has_right_boundary(text: &str, index: usize) -> bool {
    if index >= text.len() {
        return true;
    }
    text[index..]
        .chars()
        .next()
        .map_or(true, |ch| !is_word_char(ch))
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CleanupDial, SessionId};
    use ulid::Ulid;

    fn sid() -> SessionId {
        SessionId::new(Ulid::new())
    }

    #[test]
    fn word_boundaries_prevent_inside_word_rewrites() {
        let pass = DictionaryPass::new().with_terms(vec![DictionaryTerm::user("ai", "AI")]);

        assert_eq!(pass.apply_text("fail ai bail AI."), "fail AI bail AI.");
    }

    #[test]
    fn longest_match_wins_before_shorter_terms() {
        let pass = DictionaryPass::new().with_terms(vec![
            DictionaryTerm::user("new york", "NY"),
            DictionaryTerm::user("new york city", "NYC"),
        ]);

        assert_eq!(pass.apply_text("new york city and new york"), "NYC and NY");
    }

    #[test]
    fn user_terms_beat_built_ins_for_same_phrase() {
        let pass = DictionaryPass::new().with_terms(vec![
            DictionaryTerm::built_in("cadence", "Cadence"),
            DictionaryTerm::user("cadence", "Kaydence"),
        ]);

        assert_eq!(pass.apply_text("cadence"), "Kaydence");
    }

    #[test]
    fn case_sensitive_terms_only_match_exact_case() {
        let pass =
            DictionaryPass::new().with_terms(vec![
                DictionaryTerm::user("IDC", "Island Dev Crew").case_sensitive()
            ]);

        assert_eq!(pass.apply_text("idc IDC"), "idc Island Dev Crew");
    }

    #[test]
    fn snippet_triggers_are_isolated_exact_tokens() {
        let pass =
            DictionaryPass::new().with_snippets(vec![Snippet::exact("/sig", "Regards, Kaydence")]);

        assert_eq!(
            pass.apply_text("sigil /signature /sig."),
            "sigil /signature Regards, Kaydence."
        );
    }

    #[test]
    fn clean_final_events_are_personalized() {
        let id = sid();
        let pass =
            DictionaryPass::new().with_terms(vec![DictionaryTerm::user("kaydence", "Kaydence")]);

        assert_eq!(
            pass.apply_event(SessionEvent::CleanFinal {
                id,
                text: "hello kaydence".to_string(),
                dial: CleanupDial::Light,
            }),
            SessionEvent::CleanFinal {
                id,
                text: "hello Kaydence".to_string(),
                dial: CleanupDial::Light,
            }
        );
    }

    #[test]
    fn bias_hints_export_spoken_and_replacement_terms_once() {
        let pass = DictionaryPass::new().with_terms(vec![
            DictionaryTerm::user("kaydence", "Kaydence"),
            DictionaryTerm::user("Kaydence", "Kaydence"),
            DictionaryTerm::user("IDC", "Island Dev Crew"),
        ]);

        assert_eq!(
            pass.bias_hints(),
            vec!["kaydence", "IDC", "Island Dev Crew"]
        );
    }
}

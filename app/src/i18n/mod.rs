//! Message catalogs. Every user-visible string lives in `app/locales/<lang>.json`
//! (ICU MessageFormat, see [`message`]) and is addressed through [`Msg`] in code.
//!
//! Contract (`docs/web-app-construct.md`, Locale-Stack; `.cursor/rules/i18n-locales.mdc`):
//!
//! - One change = all active languages. The tests in this module fail when a key is missing,
//!   orphaned, unparsable, or reads different arguments in one language than in another.
//!   `cargo test --workspace` is that CI gate.
//! - Catalogs are compiled into the binary; a missing key can therefore only be a programming
//!   error, which the parity test catches before a user sees a raw key.
//! - Messages are text, never HTML. Rendering escapes at the edge.
//! - Product terms follow `docs/glossary.md`.

pub mod message;

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::OnceLock;

use crate::locale::{Language, LocaleStack};
pub use message::{Args, Message};

/// Bundled catalog sources, one per active language.
pub const SOURCES: [(Language, &str); 2] = [
    (Language::De, include_str!("../../locales/de.json")),
    (Language::En, include_str!("../../locales/en.json")),
];

/// Typed message keys. Adding a variant without adding the key to every catalog fails
/// `catalogs_cover_every_message_in_every_language`; adding a catalog key without a variant
/// fails `catalogs_have_no_orphan_keys`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    AppName,
    LanguageDe,
    LanguageEn,
    HelloTitle,
    HelloBody,
    HelloLanguageCurrent,
    HelloLanguagesActive,
    HelloFallbackNote,
    HelloSwitchLabel,
}

impl Msg {
    pub const ALL: [Msg; 9] = [
        Msg::AppName,
        Msg::LanguageDe,
        Msg::LanguageEn,
        Msg::HelloTitle,
        Msg::HelloBody,
        Msg::HelloLanguageCurrent,
        Msg::HelloLanguagesActive,
        Msg::HelloFallbackNote,
        Msg::HelloSwitchLabel,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Msg::AppName => "app.name",
            Msg::LanguageDe => "language.de",
            Msg::LanguageEn => "language.en",
            Msg::HelloTitle => "hello.title",
            Msg::HelloBody => "hello.body",
            Msg::HelloLanguageCurrent => "hello.language_current",
            Msg::HelloLanguagesActive => "hello.languages_active",
            Msg::HelloFallbackNote => "hello.fallback_note",
            Msg::HelloSwitchLabel => "hello.switch_label",
        }
    }

    /// Autonym key of a language, for switchers: shown in that language, whatever the UI speaks.
    pub const fn autonym(language: Language) -> Msg {
        match language {
            Language::De => Msg::LanguageDe,
            Language::En => Msg::LanguageEn,
        }
    }
}

#[derive(Debug)]
pub enum CatalogError {
    Json {
        language: Language,
        error: serde_json::Error,
    },
    Message {
        language: Language,
        key: String,
        error: message::ParseError,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Json { language, error } => {
                write!(f, "catalog {language}: invalid JSON: {error}")
            }
            CatalogError::Message {
                language,
                key,
                error,
            } => write!(f, "catalog {language}, key `{key}`: {error}"),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Parsed messages of one language.
#[derive(Debug)]
pub struct Catalog {
    language: Language,
    messages: HashMap<String, Message>,
}

impl Catalog {
    pub fn parse(language: Language, json: &str) -> Result<Catalog, CatalogError> {
        let raw: BTreeMap<String, String> =
            serde_json::from_str(json).map_err(|error| CatalogError::Json { language, error })?;
        let mut messages = HashMap::with_capacity(raw.len());
        for (key, source) in raw {
            let message = Message::parse(&source).map_err(|error| CatalogError::Message {
                language,
                key: key.clone(),
                error,
            })?;
            messages.insert(key, message);
        }
        Ok(Catalog { language, messages })
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn get(&self, key: &str) -> Option<&Message> {
        self.messages.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.messages.keys().map(String::as_str)
    }
}

/// All active catalogs.
#[derive(Debug)]
pub struct I18n {
    catalogs: Vec<Catalog>,
}

impl I18n {
    pub fn load() -> Result<I18n, CatalogError> {
        let catalogs = SOURCES
            .iter()
            .map(|(language, json)| Catalog::parse(*language, json))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(I18n { catalogs })
    }

    /// Process-wide instance. Catalogs are part of the binary; failing to load them is a build
    /// defect, so this panics loudly instead of serving raw keys.
    pub fn global() -> &'static I18n {
        static GLOBAL: OnceLock<I18n> = OnceLock::new();
        GLOBAL.get_or_init(|| I18n::load().expect("message catalogs must load"))
    }

    pub fn catalog(&self, language: Language) -> &Catalog {
        self.catalogs
            .iter()
            .find(|catalog| catalog.language == language)
            .expect("every active language has a catalog")
    }

    pub fn catalogs(&self) -> &[Catalog] {
        &self.catalogs
    }

    /// Render `msg` for the stack's language and locale. If a key were missing (the tests make
    /// that impossible) the fallback language answers; if that fails too, the key is shown
    /// wrapped in ⟦ ⟧ — visibly broken, never silently empty.
    pub fn t(&self, stack: &LocaleStack, msg: Msg, args: &Args) -> String {
        let key = msg.key();
        let found = self
            .catalog(stack.language)
            .get(key)
            .map(|message| (stack.language, message))
            .or_else(|| {
                self.catalog(Language::FALLBACK)
                    .get(key)
                    .map(|message| (Language::FALLBACK, message))
            });
        match found {
            Some((language, message)) => message.render(language, stack.locale, args),
            None => format!("⟦{key}⟧"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::locale::Locale;

    fn loaded() -> I18n {
        I18n::load().unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn every_active_language_has_a_catalog() {
        let i18n = loaded();
        let present: BTreeSet<Language> = i18n.catalogs().iter().map(Catalog::language).collect();
        let active: BTreeSet<Language> = Language::ACTIVE.into_iter().collect();
        assert_eq!(present, active);
    }

    /// The CI gate for "one change = all languages": a key present in one catalog and missing
    /// in another fails here, naming language and key.
    #[test]
    fn catalogs_cover_every_message_in_every_language() {
        let i18n = loaded();
        let mut missing = Vec::new();
        for catalog in i18n.catalogs() {
            for msg in Msg::ALL {
                if catalog.get(msg.key()).is_none() {
                    missing.push(format!("{}: {}", catalog.language(), msg.key()));
                }
            }
        }
        assert!(missing.is_empty(), "missing catalog keys: {missing:?}");
    }

    #[test]
    fn catalogs_have_no_orphan_keys() {
        let i18n = loaded();
        let known: BTreeSet<&str> = Msg::ALL.iter().map(|msg| msg.key()).collect();
        for catalog in i18n.catalogs() {
            let orphans: Vec<&str> = catalog.keys().filter(|key| !known.contains(key)).collect();
            assert!(
                orphans.is_empty(),
                "{}: keys without a Msg variant: {orphans:?}",
                catalog.language()
            );
        }
    }

    #[test]
    fn translations_of_one_key_read_the_same_arguments() {
        let i18n = loaded();
        let reference = i18n.catalog(Language::FALLBACK);
        for catalog in i18n.catalogs() {
            for msg in Msg::ALL {
                let expected = reference.get(msg.key()).unwrap().arguments();
                let actual = catalog.get(msg.key()).unwrap().arguments();
                assert_eq!(
                    actual,
                    expected,
                    "{}: `{}` uses different arguments than {}",
                    catalog.language(),
                    msg.key(),
                    Language::FALLBACK
                );
            }
        }
    }

    #[test]
    fn catalogs_contain_text_not_markup() {
        for (language, json) in SOURCES {
            let raw: BTreeMap<String, String> = serde_json::from_str(json).unwrap();
            for (key, source) in raw {
                assert!(
                    !source.contains('<') && !source.contains("&#"),
                    "{language}: `{key}` contains markup; catalogs are text"
                );
                assert!(!source.trim().is_empty(), "{language}: `{key}` is empty");
            }
        }
    }

    #[test]
    fn renders_in_both_languages_with_locale_formats() {
        let i18n = loaded();
        let de = LocaleStack::anonymous(Language::De);
        let en = LocaleStack::anonymous(Language::En);
        assert_eq!(i18n.t(&de, Msg::HelloTitle, &Args::new()), "Hallo.");
        assert_eq!(i18n.t(&en, Msg::HelloTitle, &Args::new()), "Hello.");
        assert_eq!(
            i18n.t(&de, Msg::HelloLanguagesActive, &Args::new().int("count", 2)),
            "2 aktive Sprachen, bei jeder Änderung gemeinsam gepflegt."
        );
        assert_eq!(
            i18n.t(&en, Msg::HelloLanguagesActive, &Args::new().int("count", 1)),
            "1 active language, maintained together with every change."
        );
        assert_eq!(
            i18n.t(
                &de,
                Msg::HelloLanguageCurrent,
                &Args::new().str("language", "en")
            ),
            "Aktive Sprache: English"
        );
    }

    #[test]
    fn autonyms_do_not_change_with_the_ui_language() {
        let i18n = loaded();
        for ui in Language::ACTIVE {
            let stack = LocaleStack::anonymous(ui);
            assert_eq!(
                i18n.t(&stack, Msg::autonym(Language::De), &Args::new()),
                "Deutsch"
            );
            assert_eq!(
                i18n.t(&stack, Msg::autonym(Language::En), &Args::new()),
                "English"
            );
        }
    }

    #[test]
    fn a_broken_translation_is_a_load_error_not_a_runtime_surprise() {
        let error = Catalog::parse(Language::En, r#"{"x": "{n, plural, one {#}}"}"#).unwrap_err();
        assert!(
            matches!(error, CatalogError::Message { ref key, .. } if key == "x"),
            "{error}"
        );
        let error = Catalog::parse(Language::En, "not json").unwrap_err();
        assert!(matches!(error, CatalogError::Json { .. }));
    }

    #[test]
    fn locale_stack_for_anonymous_visitors_follows_the_language() {
        assert_eq!(LocaleStack::anonymous(Language::De).locale, Locale::DeDe);
        assert_eq!(LocaleStack::anonymous(Language::En).locale, Locale::EnUs);
    }
}

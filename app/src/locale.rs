//! Locale stack: language, formats, time zones — four axes, kept apart on purpose
//! (`docs/web-app-construct.md`, section Locale-Stack).
//!
//! - [`Language`] picks the words (message catalog).
//! - [`Locale`] picks the formats (numbers, dates). Defaults from the language; user override later (P1.5).
//! - Time zones: two clocks. Display time zone belongs to the user, reporting time zone to the
//!   organisation. Storage is always UTC ([`Instant`]); server local time is never a truth.
//! - Resolution order for the effective language: user preference → org default → URL share hint →
//!   browser `Accept-Language` → product fallback (`de`).

use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Point in time as stored and transported: UTC, ISO-8601 with `Z` on the wire.
pub type Instant = DateTime<Utc>;

/// Active UI languages. Adding a locale means adding a catalog under `app/locales/`
/// and a variant here — the parity tests then demand every key in the new file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    De,
    En,
}

impl Language {
    pub const ACTIVE: [Language; 2] = [Language::De, Language::En];

    /// Product fallback when nothing else is known. Decided 2026-09-16: German.
    pub const FALLBACK: Language = Language::De;

    /// BCP-47 primary subtag, also the value of `html lang` and `Content-Language`.
    pub const fn tag(self) -> &'static str {
        match self {
            Language::De => "de",
            Language::En => "en",
        }
    }

    /// Match a language tag by primary subtag, case-insensitively: `de-AT` → `De`, `en_GB` → `En`.
    pub fn parse(tag: &str) -> Option<Language> {
        let primary = tag
            .trim()
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        Language::ACTIVE
            .into_iter()
            .find(|language| language.tag() == primary)
    }

    /// CLDR plural category for cardinals. German and English share the rule
    /// `one` ⇔ integer 1 without visible fraction digits; further locales bring their own arm.
    pub fn plural_category(self, n: f64) -> PluralCategory {
        match self {
            Language::De | Language::En => {
                if n == 1.0 {
                    PluralCategory::One
                } else {
                    PluralCategory::Other
                }
            }
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// CLDR plural categories. Catalog messages must always provide `other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralCategory {
    pub fn parse(keyword: &str) -> Option<PluralCategory> {
        Some(match keyword {
            "zero" => PluralCategory::Zero,
            "one" => PluralCategory::One,
            "two" => PluralCategory::Two,
            "few" => PluralCategory::Few,
            "many" => PluralCategory::Many,
            "other" => PluralCategory::Other,
            _ => return None,
        })
    }
}

/// Formatting locale. `de` ≠ `de-DE` ≠ `de-AT`; language and locale are separate settings.
/// V1 ships one format set per active language; user overrides arrive with account settings (P1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    DeDe,
    EnUs,
}

impl Locale {
    pub const fn tag(self) -> &'static str {
        match self {
            Locale::DeDe => "de-DE",
            Locale::EnUs => "en-US",
        }
    }

    pub const fn default_for(language: Language) -> Locale {
        match language {
            Language::De => Locale::DeDe,
            Language::En => Locale::EnUs,
        }
    }

    const fn group_separator(self) -> char {
        match self {
            Locale::DeDe => '.',
            Locale::EnUs => ',',
        }
    }

    const fn decimal_separator(self) -> char {
        match self {
            Locale::DeDe => ',',
            Locale::EnUs => '.',
        }
    }

    /// `1234567` → `1.234.567` (de-DE) / `1,234,567` (en-US).
    pub fn format_integer(self, n: i64) -> String {
        let digits = n.unsigned_abs().to_string();
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                grouped.push(self.group_separator());
            }
            grouped.push(ch);
        }
        if n < 0 {
            format!("-{grouped}")
        } else {
            grouped
        }
    }

    /// Up to three fraction digits, trailing zeros trimmed (ICU default for `{x, number}`).
    pub fn format_decimal(self, x: f64) -> String {
        if !x.is_finite() {
            return x.to_string();
        }
        let rounded = format!("{x:.3}");
        let (int_part, frac_part) = rounded.split_once('.').unwrap_or((&rounded, ""));
        let frac = frac_part.trim_end_matches('0');
        let int_value: i64 = int_part.parse().unwrap_or(0);
        let mut out = self.format_integer(int_value);
        if int_value == 0 && int_part.starts_with('-') {
            out.insert(0, '-');
        }
        if !frac.is_empty() {
            out.push(self.decimal_separator());
            out.push_str(frac);
        }
        out
    }

    /// Numeric date and time in the given zone: `16.09.2026, 00:10` (de-DE),
    /// `09/16/2026, 12:10 AM` (en-US). Storage stays UTC; this is display only.
    pub fn format_datetime(self, instant: Instant, zone: Tz) -> String {
        let local = zone.from_utc_datetime(&instant.naive_utc());
        let pattern = match self {
            Locale::DeDe => "%d.%m.%Y, %H:%M",
            Locale::EnUs => "%m/%d/%Y, %-I:%M %p",
        };
        local.format(pattern).to_string()
    }
}

/// Display time zone of a person. `Auto` follows the device when the client reports one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeZonePref {
    Auto,
    Fixed(Tz),
}

/// Locale fields that live on the user and travel into every organisation.
#[derive(Debug, Clone, PartialEq)]
pub struct UserLocalePrefs {
    pub ui_language: Option<Language>,
    pub locale: Option<Locale>,
    pub display_tz: TimeZonePref,
}

impl Default for UserLocalePrefs {
    fn default() -> Self {
        Self {
            ui_language: None,
            locale: None,
            display_tz: TimeZonePref::Auto,
        }
    }
}

/// Locale defaults that live on the organisation. Fiscal calendar joins later (P1.5 / P5).
#[derive(Debug, Clone, PartialEq)]
pub struct OrgLocaleDefaults {
    /// Day, week and month boundaries of every report. Set once by an admin.
    pub reporting_tz: Tz,
    /// Language of invitations and system mail for people without a preference yet.
    pub invite_language: Language,
}

/// Everything known about a request's language before resolution.
#[derive(Debug, Clone, Copy, Default)]
pub struct LanguageSignals<'a> {
    pub user: Option<Language>,
    pub org_default: Option<Language>,
    /// `?lang=` share hint. Never the truth once a stored preference exists.
    pub url_hint: Option<Language>,
    pub accept_language: Option<&'a str>,
}

/// User preference → org default → URL hint → browser → [`Language::FALLBACK`].
pub fn resolve_language(signals: LanguageSignals<'_>) -> Language {
    signals
        .user
        .or(signals.org_default)
        .or(signals.url_hint)
        .or_else(|| signals.accept_language.and_then(parse_accept_language))
        .unwrap_or(Language::FALLBACK)
}

/// Display zone: fixed preference → device hint (when `Auto`) → org reporting zone → UTC.
/// Server local time is deliberately not on this list.
pub fn resolve_display_tz(
    pref: TimeZonePref,
    device_hint: Option<Tz>,
    org: Option<&OrgLocaleDefaults>,
) -> Tz {
    match pref {
        TimeZonePref::Fixed(zone) => zone,
        TimeZonePref::Auto => device_hint
            .or_else(|| org.map(|org| org.reporting_tz))
            .unwrap_or(Tz::UTC),
    }
}

/// First active language in an `Accept-Language` header, by descending quality.
/// `de-AT,de;q=0.9,en;q=0.8` → `De`; `fr-CH,fr;q=0.9` → `None`.
pub fn parse_accept_language(header: &str) -> Option<Language> {
    let mut candidates: Vec<(f64, usize, Language)> = header
        .split(',')
        .enumerate()
        .filter_map(|(index, entry)| {
            let mut parts = entry.split(';');
            let tag = parts.next()?.trim();
            let quality = parts
                .filter_map(|param| param.trim().strip_prefix("q="))
                .find_map(|q| q.trim().parse::<f64>().ok())
                .unwrap_or(1.0);
            if quality <= 0.0 {
                return None;
            }
            Language::parse(tag).map(|language| (quality, index, language))
        })
        .collect();
    candidates.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    candidates.first().map(|(_, _, language)| *language)
}

/// The effective stack of one request or one rendering: words, formats, display clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocaleStack {
    pub language: Language,
    pub locale: Locale,
    pub display_tz: Tz,
}

impl LocaleStack {
    /// Stack for an anonymous visitor: only the language is known, formats follow it, clock is UTC.
    pub fn anonymous(language: Language) -> Self {
        Self {
            language,
            locale: Locale::default_for(language),
            display_tz: Tz::UTC,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_parses_by_primary_subtag() {
        assert_eq!(Language::parse("de"), Some(Language::De));
        assert_eq!(Language::parse("de-AT"), Some(Language::De));
        assert_eq!(Language::parse("EN_gb"), Some(Language::En));
        assert_eq!(Language::parse(" en "), Some(Language::En));
        assert_eq!(Language::parse("fr"), None);
        assert_eq!(Language::parse(""), None);
        assert_eq!(Language::parse("deu"), None);
    }

    #[test]
    fn fallback_is_german() {
        assert_eq!(Language::FALLBACK, Language::De);
        assert_eq!(resolve_language(LanguageSignals::default()), Language::De);
    }

    #[test]
    fn resolution_order_user_org_url_browser_fallback() {
        let browser = Some("en-US,en;q=0.9");
        let all = LanguageSignals {
            user: Some(Language::De),
            org_default: Some(Language::En),
            url_hint: Some(Language::En),
            accept_language: browser,
        };
        assert_eq!(resolve_language(all), Language::De, "user wins");
        assert_eq!(
            resolve_language(LanguageSignals { user: None, ..all }),
            Language::En,
            "org default beats url hint"
        );
        assert_eq!(
            resolve_language(LanguageSignals {
                user: None,
                org_default: Some(Language::De),
                url_hint: Some(Language::En),
                accept_language: browser,
            }),
            Language::De,
            "url hint is a share aid, not the truth"
        );
        assert_eq!(
            resolve_language(LanguageSignals {
                user: None,
                org_default: None,
                url_hint: Some(Language::En),
                accept_language: Some("de"),
            }),
            Language::En,
            "url hint beats browser"
        );
        assert_eq!(
            resolve_language(LanguageSignals {
                accept_language: browser,
                ..LanguageSignals::default()
            }),
            Language::En,
            "browser beats fallback"
        );
    }

    #[test]
    fn accept_language_respects_quality_and_order() {
        assert_eq!(
            parse_accept_language("de-AT,de;q=0.9,en;q=0.8"),
            Some(Language::De)
        );
        assert_eq!(
            parse_accept_language("fr-CH, en;q=0.7, de;q=0.9"),
            Some(Language::De)
        );
        assert_eq!(parse_accept_language("fr-CH,fr;q=0.9"), None);
        assert_eq!(parse_accept_language("en, de"), Some(Language::En));
        assert_eq!(
            parse_accept_language("de;q=0, en;q=0.1"),
            Some(Language::En)
        );
        assert_eq!(parse_accept_language("*"), None);
        assert_eq!(parse_accept_language(""), None);
    }

    #[test]
    fn integers_group_per_locale() {
        assert_eq!(Locale::DeDe.format_integer(0), "0");
        assert_eq!(Locale::DeDe.format_integer(999), "999");
        assert_eq!(Locale::DeDe.format_integer(1234), "1.234");
        assert_eq!(Locale::DeDe.format_integer(1234567), "1.234.567");
        assert_eq!(Locale::EnUs.format_integer(1234567), "1,234,567");
        assert_eq!(Locale::DeDe.format_integer(-1234), "-1.234");
    }

    #[test]
    fn decimals_use_locale_separators() {
        assert_eq!(Locale::DeDe.format_decimal(1234.5), "1.234,5");
        assert_eq!(Locale::EnUs.format_decimal(1234.5), "1,234.5");
        assert_eq!(Locale::DeDe.format_decimal(2.0), "2");
        assert_eq!(Locale::EnUs.format_decimal(0.125), "0.125");
        assert_eq!(Locale::EnUs.format_decimal(0.12345), "0.123");
        assert_eq!(Locale::DeDe.format_decimal(-0.5), "-0,5");
    }

    #[test]
    fn plural_one_only_for_exact_one() {
        for language in Language::ACTIVE {
            assert_eq!(language.plural_category(1.0), PluralCategory::One);
            assert_eq!(language.plural_category(0.0), PluralCategory::Other);
            assert_eq!(language.plural_category(2.0), PluralCategory::Other);
            assert_eq!(language.plural_category(1.5), PluralCategory::Other);
        }
    }

    #[test]
    fn datetime_is_displayed_in_the_user_zone_not_utc() {
        let instant = Utc.with_ymd_and_hms(2026, 9, 15, 22, 10, 0).unwrap();
        assert_eq!(
            Locale::DeDe.format_datetime(instant, chrono_tz::Europe::Berlin),
            "16.09.2026, 00:10"
        );
        assert_eq!(
            Locale::EnUs.format_datetime(instant, chrono_tz::America::New_York),
            "09/15/2026, 6:10 PM"
        );
        assert_eq!(
            Locale::DeDe.format_datetime(instant, chrono_tz::Asia::Tokyo),
            "16.09.2026, 07:10"
        );
        assert_eq!(
            Locale::EnUs.format_datetime(instant, Tz::UTC),
            "09/15/2026, 10:10 PM"
        );
    }

    #[test]
    fn display_zone_never_falls_back_to_server_local_time() {
        let org = OrgLocaleDefaults {
            reporting_tz: chrono_tz::Europe::Berlin,
            invite_language: Language::De,
        };
        assert_eq!(
            resolve_display_tz(
                TimeZonePref::Fixed(chrono_tz::Asia::Tokyo),
                Some(Tz::UTC),
                Some(&org)
            ),
            chrono_tz::Asia::Tokyo
        );
        assert_eq!(
            resolve_display_tz(
                TimeZonePref::Auto,
                Some(chrono_tz::America::New_York),
                Some(&org)
            ),
            chrono_tz::America::New_York
        );
        assert_eq!(
            resolve_display_tz(TimeZonePref::Auto, None, Some(&org)),
            chrono_tz::Europe::Berlin
        );
        assert_eq!(resolve_display_tz(TimeZonePref::Auto, None, None), Tz::UTC);
    }

    #[test]
    fn iana_names_parse_and_garbage_does_not() {
        assert!("Europe/Berlin".parse::<Tz>().is_ok());
        assert!("Asia/Tokyo".parse::<Tz>().is_ok());
        assert!("CEST".parse::<Tz>().is_err());
        assert!("Berlin".parse::<Tz>().is_err());
    }
}

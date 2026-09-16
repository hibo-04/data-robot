//! HTML rendering at the edge. Messages are text; everything interpolated is escaped here.
//!
//! The hello page proves the locale stack (P0.2): `html lang`, catalog copy, plural, select,
//! switcher. It is not the product chrome — that arrives with Phase 2 and replaces this page.

use crate::i18n::{Args, I18n, Msg};
use crate::locale::{Language, LocaleStack};

pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn hello(stack: &LocaleStack) -> String {
    let i18n = I18n::global();
    let text = |msg: Msg, args: &Args| escape_html(&i18n.t(stack, msg, args));
    let app_name = i18n.t(stack, Msg::AppName, &Args::new());

    let switcher = Language::ACTIVE
        .iter()
        .map(|language| {
            let current = if *language == stack.language {
                " aria-current=\"true\""
            } else {
                ""
            };
            format!(
                "<li><a href=\"/?lang={tag}\" hreflang=\"{tag}\" lang=\"{tag}\"{current}>{label}</a></li>",
                tag = language.tag(),
                label = text(Msg::autonym(*language), &Args::new()),
            )
        })
        .collect::<String>();

    format!(
        "<!doctype html>\n\
         <html lang=\"{lang}\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         </head>\n\
         <body>\n\
         <main>\n\
         <h1>{heading}</h1>\n\
         <p>{body}</p>\n\
         <p>{current}</p>\n\
         <p>{active}</p>\n\
         <p>{fallback}</p>\n\
         <nav aria-label=\"{switch_label}\">\n\
         <ul>{switcher}</ul>\n\
         </nav>\n\
         </main>\n\
         </body>\n\
         </html>\n",
        lang = stack.language.tag(),
        title = escape_html(&app_name),
        heading = text(Msg::HelloTitle, &Args::new()),
        body = text(Msg::HelloBody, &Args::new().str("app", app_name.clone())),
        current = text(
            Msg::HelloLanguageCurrent,
            &Args::new().str("language", stack.language.tag()),
        ),
        active = text(
            Msg::HelloLanguagesActive,
            &Args::new().int("count", Language::ACTIVE.len() as i64),
        ),
        fallback = text(Msg::HelloFallbackNote, &Args::new()),
        switch_label = text(Msg::HelloSwitchLabel, &Args::new()),
        switcher = switcher,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_five_html_metacharacters() {
        assert_eq!(
            escape_html(r#"<a href="x">Tom & Jerry's</a>"#),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;"
        );
        assert_eq!(escape_html("Ünïcödé bleibt"), "Ünïcödé bleibt");
    }

    #[test]
    fn hello_page_declares_language_and_marks_the_current_switch() {
        let de = hello(&LocaleStack::anonymous(Language::De));
        assert!(
            de.starts_with("<!doctype html>\n<html lang=\"de\">"),
            "{de}"
        );
        assert!(de.contains("<h1>Hallo.</h1>"));
        assert!(de.contains("2 aktive Sprachen"));
        assert!(de.contains(
            r#"<a href="/?lang=de" hreflang="de" lang="de" aria-current="true">Deutsch</a>"#
        ));
        assert!(de.contains(r#"<a href="/?lang=en" hreflang="en" lang="en">English</a>"#));

        let en = hello(&LocaleStack::anonymous(Language::En));
        assert!(en.contains("<html lang=\"en\">"));
        assert!(en.contains("<h1>Hello.</h1>"));
        assert!(en.contains("2 active languages"));
        assert!(en.contains(r#"lang="en" aria-current="true">English</a>"#));
    }
}

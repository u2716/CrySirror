// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Small system-information plug-ins.
//!
//! getLangName.dll: the user locale as an ISO-639 UI language (`ja`) and a
//! BCP-47-style locale name (`ja-JP`). The adapter's configured `locale`
//! comes first, then the POSIX locale environment, then `en-US`.
use crate::natives::Rt;
use crysirror_tjs2::runtime::{ObjectHandle, Variant};

const FALLBACK_LOCALE_NAME: &str = "en-US";
const FALLBACK_UI_LANG_NAME: &str = "en";

pub(crate) fn install_lang_name(rt: &mut Rt) {
    let Some(system) = rt.global_member("System").object_handle() else { return };
    for (name, ui) in [("getCurrentUILangName", true), ("getCurrentLocaleName", false)] {
        if matches!(rt.object_member(system, name), Variant::Closure(_)) {
            continue;
        }
        let f = move |rt: &mut Rt, _: Option<ObjectHandle>, _: Vec<Variant>| {
            let (locale, language) = user_locale(rt);
            Ok(Variant::String(if ui { language } else { locale }))
        };
        rt.register_object_native(system, name, f);
    }
}

/// `(locale name, UI language)` of the user: configured, environment, `en-US`.
pub(crate) fn user_locale(rt: &Rt) -> (String, String) {
    reference_names(rt.host().locale.clone().or_else(host_locale).as_deref())
}

fn host_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(|name| {
        std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    })
}

/// `(locale name, UI language)`: `ja_JP.UTF-8` and `ja-JP` give `("ja-JP", "ja")`.
fn reference_names(raw: Option<&str>) -> (String, String) {
    match raw.and_then(parse_locale) {
        Some((language, subtag)) => {
            let locale = match subtag {
                Some(subtag) => format!("{language}-{subtag}"),
                None => language.clone(),
            };
            (locale, language)
        }
        None => (FALLBACK_LOCALE_NAME.into(), FALLBACK_UI_LANG_NAME.into()),
    }
}

fn parse_locale(raw: &str) -> Option<(String, Option<String>)> {
    let base = raw.split('@').next()?.split('.').next()?.trim();
    if base.is_empty() || base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    let mut parts = base.split(['_', '-']);
    let language = parts.next()?.trim();
    if !(2..=3).contains(&language.len()) || !language.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let language = language.to_ascii_lowercase();
    let subtag = parts.next().map(str::trim).and_then(|subtag| {
        let alphabetic = subtag.chars().all(|c| c.is_ascii_alphabetic());
        let digits = subtag.chars().all(|c| c.is_ascii_digit());
        match subtag.len() {
            2 if alphabetic => Some(subtag.to_ascii_uppercase()),
            3 if alphabetic => Some(subtag.to_ascii_lowercase()),
            3 if digits => Some(subtag.to_string()),
            4 if alphabetic => Some(format!("{}{}", subtag[..1].to_ascii_uppercase(), subtag[1..].to_ascii_lowercase())),
            _ => None,
        }
    });
    Some((language, subtag))
}

#[cfg(test)]
mod tests {
    use super::reference_names;

    #[test]
    fn locale_spellings_map_to_reference_names() {
        for (raw, locale, ui) in [
            ("ja_JP.UTF-8", "ja-JP", "ja"),
            ("ja-JP", "ja-JP", "ja"),
            ("en_US", "en-US", "en"),
            ("de_DE@euro", "de-DE", "de"),
            ("zh_Hans", "zh-Hans", "zh"),
            ("es_419", "es-419", "es"),
            ("fil_PH", "fil-PH", "fil"),
            ("ja", "ja", "ja"),
        ] {
            assert_eq!(reference_names(Some(raw)), (locale.to_string(), ui.to_string()), "{raw}");
        }
        for raw in [None, Some(""), Some("C"), Some("POSIX"), Some("garbage!"), Some("_JP")] {
            assert_eq!(reference_names(raw), ("en-US".to_string(), "en".to_string()), "{raw:?}");
        }
    }
}

use std::{collections::HashMap, sync::Arc};

use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
pub struct I18n {
	bundles: Arc<HashMap<String, FluentBundle<FluentResource>>>,
	default_locale: Arc<str>,
}

impl I18n {
	pub fn new() -> Self {
		let mut bundles = HashMap::new();
		let en = Self::build_bundle("en-UK", include_str!("langs/en-UK/main.ftl"));
		bundles.insert("en-uk".to_string(), en);
		let de = Self::build_bundle("de-DE", include_str!("langs/de-DE/main.ftl"));
		bundles.insert("de-de".to_string(), de);
		let es = Self::build_bundle("es-ES", include_str!("langs/es-ES/main.ftl"));
		bundles.insert("es-es".to_string(), es);
		let ie = Self::build_bundle("ie-IE", include_str!("langs/ie-IE/main.ftl"));
		bundles.insert("ie-ie".to_string(), ie);
		let el = Self::build_bundle("el-GR", include_str!("langs/el-GR/main.ftl"));
		bundles.insert("el-gr".to_string(), el);
		let tr = Self::build_bundle("tr-TR", include_str!("langs/tr-TR/main.ftl"));
		bundles.insert("tr-tr".to_string(), tr);
		let ru = Self::build_bundle("ru-RU", include_str!("langs/ru-RU/main.ftl"));
		bundles.insert("ru-ru".to_string(), ru);
		let ar = Self::build_bundle("ar", include_str!("langs/ar/main.ftl"));
		bundles.insert("ar".to_string(), ar);
		Self {
			bundles: Arc::new(bundles),
			default_locale: Arc::from("en-uk"),
		}
	}

	fn build_bundle(locale: &str, ftl: &str) -> FluentBundle<FluentResource> {
		let lang: LanguageIdentifier = locale.parse().expect("invalid language id");
		let resource = FluentResource::try_new(ftl.to_string()).expect("invalid FTL");
		let mut bundle = FluentBundle::new_concurrent(vec![lang]);
		bundle
			.add_resource(resource)
			.expect("failed to add FTL resource");
		bundle
	}

	// locale is expected to be normalized lower-case, e.g. "de-de", "de", "en-uk"
	pub fn t(&self, locale: &str, key: &str) -> String {
		// 1) exact match: "de-de"
		// 2) same language: "de" or "de-at" -> the "de-*" bundle ("ar-sa" -> "ar")
		// 3) default locale bundle
		// 4) key itself
		let locale_norm = locale.trim().to_ascii_lowercase();
		let lang = locale_norm.split('-').next().unwrap_or_default();
		let resolved_bundle = self
			.bundles
			.get(&locale_norm)
			.or_else(|| {
				self.bundles
					.iter()
					.find(|(tag, _)| tag.split('-').next() == Some(lang))
					.map(|(_, bundle)| bundle)
			})
			.or_else(|| self.bundles.get(self.default_locale.as_ref()));
		let Some(bundle) = resolved_bundle else {
			return key.to_string();
		};
		let Some(msg) = bundle.get_message(key) else {
			return key.to_string();
		};
		let Some(value) = msg.value() else {
			return key.to_string();
		};
		let mut errors = vec![];
		bundle.format_pattern(value, None, &mut errors).to_string()
	}
	pub fn default_locale(&self) -> &str {
		&self.default_locale
	}
}

#[cfg(test)]
mod tests {
	use super::I18n;

	const LOCALES: [&str; 8] = [
		"en-uk", "de-de", "es-es", "ie-ie", "el-gr", "tr-tr", "ru-ru", "ar",
	];

	fn keys(ftl: &str) -> Vec<&str> {
		ftl.lines()
			.filter_map(|l| l.split_once(" = ").map(|(k, _)| k.trim()))
			.collect()
	}

	#[test]
	fn every_locale_translates_every_message() {
		let i18n = I18n::new();
		for key in keys(include_str!("langs/en-UK/main.ftl")) {
			let english = i18n.t("en-uk", key);
			for locale in LOCALES {
				let text = i18n.t(locale, key);
				assert_ne!(text, key, "{locale} is missing {key}");
				if locale != "en-uk" {
					assert_ne!(text, english, "{locale} falls back to English for {key}");
				}
				assert!(
					text.starts_with(char::is_alphanumeric),
					"{locale} {key} starts with stray punctuation: {text:?}"
				);
			}
		}
	}

	#[test]
	fn locale_resolution_falls_back_by_language_then_to_english() {
		let i18n = I18n::new();
		assert_eq!(i18n.t("DE-de", "note-not-found"), "Notiz nicht gefunden");
		assert_eq!(i18n.t("de", "note-not-found"), "Notiz nicht gefunden");
		assert_eq!(i18n.t("de-AT", "note-not-found"), "Notiz nicht gefunden");
		assert_eq!(
			i18n.t("ar-SA", "note-not-found"),
			i18n.t("ar", "note-not-found")
		);
		assert_eq!(i18n.t("fr-FR", "note-not-found"), "Note not found");
		assert_eq!(i18n.t("en-uk", "no-such-key"), "no-such-key");
	}
}

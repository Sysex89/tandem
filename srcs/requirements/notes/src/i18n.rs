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
		// 2) language-only fallback: "de-de" -> "de"
		// 3) default locale bundle
		// 4) key itself
		let locale_norm = locale.trim().to_ascii_lowercase();
		let resolved_bundle = self
			.bundles
			.get(&locale_norm)
			.or_else(|| {
				locale_norm
					.split('-')
					.next()
					.and_then(|lang| self.bundles.get(lang))
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

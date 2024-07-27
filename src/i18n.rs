use std::collections::HashMap;

use anyhow::anyhow;
use fluent::{FluentArgs, FluentBundle, FluentResource};
use log::error;
use unic_langid::{langid, LanguageIdentifier};

pub struct I18N {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
}

impl I18N {
    pub fn new() -> Self {
        Self {
            bundles: HashMap::from_iter([
                // (langid!("zh-CN"), fluent_bundle_zh_cn()),
                (langid!("en-US"), fluent_bundle_en_us()),
            ]),
        }
    }

    pub fn bundle(&self, lang_id: &LanguageIdentifier) -> &FluentBundle<FluentResource> {
        self.bundles
            .get(lang_id)
            .unwrap_or(&self.bundles[&langid!("en-US")])
    }

    pub fn format_msg(
        &self,
        lang_id: &LanguageIdentifier,
        msg_id: impl AsRef<str>,
        params: Option<Vec<(String, String)>>,
    ) -> anyhow::Result<String> {
        let msg_id = msg_id.as_ref();
        let bundle = self.bundle(lang_id);
        let msg = bundle.get_message(msg_id);
        if let Some(msg) = msg {
            if let Some(pattern) = msg.value() {
                let mut errs = Vec::new();
                let res = bundle.format_pattern(
                    pattern,
                    params.map(FluentArgs::from_iter).as_ref(),
                    &mut errs,
                );
                if !errs.is_empty() {
                    return Err(anyhow!("format error: {:?}", errs));
                }
                Ok(res.to_string())
            } else {
                Err(anyhow!(
                    "key missing: key: {}, lang_id: {}",
                    msg_id,
                    lang_id
                ))
            }
        } else {
            Err(anyhow!(
                "key missing: key: {}, lang_id: {}",
                msg_id,
                lang_id
            ))
        }
    }

    pub fn format_msg_or_log(
        &self,
        lang_id: &LanguageIdentifier,
        msg_id: impl AsRef<str>,
        params: Option<Vec<(String, String)>>,
    ) -> String {
        let msg_id = msg_id.as_ref();
        self.format_msg(lang_id, msg_id, params)
            .unwrap_or_else(|err| {
                error!("{:?}", err);
                msg_id.to_string()
            })
    }
}

//
// fn fluent_bundle_zh_cn() -> FluentBundle<FluentResource> {
//     let mut bundle = FluentBundle::new(vec![langid!("zh-CN"), langid!("en-GB")]);
//     bundle
//         .add_resource(
//             FluentResource::try_new(include_str!("../assets/lang/zh_CN.ftl").to_string())
//                 .expect("Failed to parse an FTL string."),
//         )
//         .expect("Failed to add FTL resources to the bundle.");

//     bundle
// }

fn fluent_bundle_en_us() -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![langid!("en-US")]);
    bundle
        .add_resource(
            FluentResource::try_new(include_str!("../assets/lang/en_US.ftl").to_string())
                .expect("Failed to parse an FTL string."),
        )
        .expect("Failed to add FTL resources to the bundle.");

    bundle
}

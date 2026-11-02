use crate::model::{Attr, Detail};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::sync::OnceLock;

pub fn number(raw: &str) -> Option<f64> {
    static NUM: OnceLock<Regex> = OnceLock::new();
    let normalized = raw.replace([',', '，'], "");
    let text = normalized.rsplit('/').next()?;
    NUM.get_or_init(|| Regex::new(r"-?\d+(?:\.\d+)?").unwrap())
        .find(text)?
        .as_str()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn text(el: ElementRef<'_>) -> String {
    el.text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse(html: &str) -> Result<Detail, String> {
    let doc = Html::parse_document(html);
    let role_sel = Selector::parse(".m-intro-content").unwrap();
    let role = doc
        .select(&role_sel)
        .next()
        .ok_or("详情缺少角色区域，可能已下架或页面结构变化")?;
    let value_sel = Selector::parse(".value").unwrap();
    let mut result = Detail::default();
    for key in role.select(&Selector::parse(".key").unwrap()) {
        let nested = key.select(&value_sel).next();
        let value = nested.or_else(|| {
            let mut sibling = key.next_sibling();
            while let Some(node) = sibling {
                if let Some(el) = ElementRef::wrap(node) {
                    return el
                        .value()
                        .has_class("value", scraper::CaseSensitivity::CaseSensitive)
                        .then_some(el);
                }
                sibling = node.next_sibling();
            }
            None
        });
        if let Some(value) = value {
            let raw = text(value);
            let key_text = text(key);
            let label = key_text
                .split(['：', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('?')
                .trim();
            if !label.is_empty() && !raw.is_empty() {
                result.attrs.entry(label.into()).or_insert_with(|| Attr {
                    num: number(&raw),
                    raw,
                });
            }
        }
    }
    if !result.attrs.contains_key("名称")
        || !result.attrs.get("等级").is_some_and(|v| v.num.is_some())
    {
        return Err("未解析到名称和等级；保留旧详情，请稍后重试".into());
    }
    // Only the group titled 装备 counts; pets and bags can contain identical item tags.
    for group in doc.select(&Selector::parse(".m-group").unwrap()) {
        if group
            .select(&Selector::parse("h4").unwrap())
            .next()
            .map(text)
            .as_deref()
            != Some("装备")
        {
            continue;
        }
        static NAME: OnceLock<Regex> = OnceLock::new();
        static TYPE: OnceLock<Regex> = OnceLock::new();
        let name_re = NAME.get_or_init(|| {
            Regex::new(r#"(?:^|[,\s{])['"]?name['"]?\s*:\s*['"]([^'"]*)['"]"#).unwrap()
        });
        let type_re = TYPE.get_or_init(|| {
            Regex::new(r#"(?:^|[,\s{])['"]?type['"]?\s*:\s*['"]([^'"]*)['"]"#).unwrap()
        });
        for item in group.select(&Selector::parse("[data-info]").unwrap()) {
            let info = item.value().attr("data-info").unwrap_or("");
            let (Some(name), Some(kind)) = (name_re.captures(info), type_re.captures(info)) else {
                continue;
            };
            let slot = match &kind[1] {
                "4" => "武器",
                "3" => "项链",
                "5" => "戒指",
                "1" | "2" | "8" | "9" | "11" => "防具",
                _ => continue,
            };
            let tier = if name[1].contains("至尊") {
                "至尊"
            } else if name[1].contains("神佑") {
                "神佑"
            } else {
                continue;
            };
            *result.equipment.entry(format!("{tier}{slot}")).or_insert(0) += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_nested_values_and_only_role_equipment() {
        let detail = parse(include_str!("../tests/detail.html")).unwrap();
        assert_eq!(detail.attrs["等级"].num, Some(159.0));
        assert_eq!(detail.attrs["生命值"].num, Some(20000.0));
        assert_eq!(detail.attrs["职业"].raw, "剑客");
        assert_eq!(detail.attrs["攻击力"].num, Some(1234.0));
        assert_eq!(detail.equipment["至尊武器"], 1);
        assert_eq!(detail.equipment["神佑防具"], 2);
        assert!(!detail.attrs.contains_key("召唤兽专属"));
        assert_eq!(detail.equipment.len(), 2);
        assert!(parse("<html>维护中</html>").is_err());
        assert_eq!(number("无"), None);
        assert_eq!(number("75%"), Some(75.0));
        assert_eq!(number("LV6"), Some(6.0));
    }
}

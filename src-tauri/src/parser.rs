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

pub fn unavailable(html: &str, code: &str, server: &str, goods_type: u8) -> Result<bool, String> {
    let doc = Html::parse_document(html);
    let info = doc
        .select(&Selector::parse(".m-goods-info").unwrap())
        .next()
        .ok_or("官网详情缺少商品信息，保留缓存")?;
    if info.value().attr("data-goodsid") != Some(code)
        || info.value().attr("data-serverid") != Some(server)
        || info.value().attr("data-gameid") != Some("16")
        || info.value().attr("data-gt") != Some(goods_type.to_string().as_str())
    {
        return Err("官网详情商品不匹配，保留缓存".into());
    }
    let status = info
        .select(&Selector::parse(".goodsStatus").unwrap())
        .next()
        .map(text);
    match status.as_deref() {
        Some("已销售" | "已售出" | "已下架") => Ok(true),
        Some("在售" | "待售中" | "出售中" | "公示中") => Ok(false),
        _ => Err("官网销售状态无法确认，保留缓存".into()),
    }
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

pub fn parse_spirit(html: &str) -> Result<Detail, String> {
    let doc = Html::parse_document(html);
    let mut result = Detail::default();
    for group in doc.select(&Selector::parse(".m-group").unwrap()) {
        let title = group
            .select(&Selector::parse("h4").unwrap())
            .next()
            .map(text);
        if !matches!(title.as_deref(), Some("星灵" | "属性")) {
            continue;
        }
        for item in group.select(&Selector::parse(".info-item").unwrap()) {
            let key = item.select(&Selector::parse(".key").unwrap()).next();
            let value = item.select(&Selector::parse(".value").unwrap()).next();
            if let (Some(key), Some(value)) = (key, value) {
                let label = text(key).trim_end_matches(['：', ':']).to_string();
                let raw = text(value);
                if !label.is_empty() && !raw.is_empty() {
                    result.attrs.insert(
                        label,
                        Attr {
                            num: number(&raw),
                            raw,
                        },
                    );
                }
            }
        }
        if title.as_deref() == Some("星灵") {
            for item in group.select(&Selector::parse("[title]").unwrap()) {
                let title = item.value().attr("title").unwrap_or_default();
                if let Some(raw) = title.strip_prefix("强化等级：") {
                    if let Some(num) = number(raw.split('级').next().unwrap_or_default()) {
                        result.attrs.insert(
                            "强化".into(),
                            Attr {
                                raw: num.to_string(),
                                num: Some(num),
                            },
                        );
                    }
                }
            }
        }
    }
    if [
        "等级",
        "强化",
        "生命值",
        "技能值",
        "攻击力",
        "防御力",
        "精神力",
        "灵巧度",
        "命中率",
        "闪避率",
    ]
    .iter()
    .any(|key| !result.attrs.get(*key).is_some_and(|a| a.num.is_some()))
        || !result.attrs.contains_key("资质")
        || !result.attrs.contains_key("成长")
    {
        return Err("星灵详情字段不完整；保留旧详情，请稍后重试".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spirit_sales_status_is_checked_as_spirit_and_preserves_active_or_unknown_goods() {
        let html = include_str!("../tests/spirit-status.html");
        let code = "1622610181944570349";
        assert!(!unavailable(html, code, "104", 5).unwrap());
        // Terminal variants exercise the real star page structure, not a captured sold star.
        for status in ["已销售", "已售出", "已下架"] {
            assert!(unavailable(&html.replace("待售中", status), code, "104", 5).unwrap());
        }
        assert!(unavailable(html, code, "104", 1).is_err());
        assert!(unavailable(html, code, "615", 5).is_err());
        assert!(unavailable(html, "999", "104", 5).is_err());
        assert!(unavailable(&html.replace("待售中", "未知状态"), code, "104", 5).is_err());
    }
    #[test]
    fn sold_role_status_requires_matching_official_identity_and_explicit_terminal_status() {
        let html = include_str!("../tests/sold.html");
        let code = "1622610310017090018";
        assert_eq!(unavailable(html, code, "104", 1).unwrap(), true);
        for status in ["在售", "出售中", "公示中"] {
            assert!(!unavailable(&html.replace("已销售", status), code, "104", 1).unwrap());
        }
        for status in ["", "维护中", "未知状态"] {
            assert!(unavailable(&html.replace("已销售", status), code, "104", 1).is_err());
        }
        assert!(unavailable(html, "999", "104", 1).is_err());
        assert!(unavailable(html, code, "615", 1).is_err());
        assert!(unavailable(html, code, "104", 5).is_err());
        assert!(unavailable(
            &html.replace("data-gameId=\"16\"", "data-gameId=\"3\""),
            code,
            "104",
            1
        )
        .is_err());
        assert!(unavailable("<html>维护中</html>", code, "104", 1).is_err());
    }
    #[test]
    fn star_detail_reads_current_panel_and_title_stars_without_treasure_contamination() {
        let html = include_str!("../tests/spirit.html");
        let detail = parse_spirit(html).unwrap();
        assert_eq!(detail.attrs["强化"].num, Some(10.0));
        assert_eq!(detail.attrs["慧心"].num, Some(111.0));
        assert_eq!(detail.attrs["敏捷资质"].num, Some(3.2));
        assert_eq!(detail.attrs["灵巧度"].num, Some(2726.0));
        assert_eq!(detail.attrs["生命值"].num, Some(3003.0));
        let html = html.replace("暂无灵宝数据", "<p class='info-item'><span class='key'>生命值：</span><span class='value'>99999</span></p>");
        assert_eq!(
            parse_spirit(&html).unwrap().attrs["生命值"].num,
            Some(3003.0)
        );
        assert!(parse_spirit("<html>维护</html>").is_err());
        assert!(parse_spirit(&html.replace("技能值：", "未知字段：")).is_err());
        assert!(parse_spirit(include_str!("../tests/detail.html")).is_err());
    }
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

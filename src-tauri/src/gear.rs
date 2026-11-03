use crate::model::{Attr, Detail};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Gear {
    pub kind: String,
    pub level: Option<f64>,
    pub score: Option<f64>,
    pub stats: Vec<(String, f64, String)>,
    pub effects: Vec<String>,
    pub set: Vec<String>,
    pub raw: Vec<String>,
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
    let panel = doc
        .select(&Selector::parse("#J_Eqiup").unwrap())
        .next()
        .ok_or("未找到装备详情区域，保留原缓存")?;
    let mut detail = Detail::default();
    for field in panel.select(&Selector::parse(".info-item").unwrap()) {
        if let (Some(key), Some(value)) = (
            field.select(&Selector::parse(".key").unwrap()).next(),
            field.select(&Selector::parse(".value").unwrap()).next(),
        ) {
            let name = text(key).trim_end_matches(['：', ':']).to_string();
            let raw = text(value);
            detail.attrs.insert(
                name,
                Attr {
                    num: crate::parser::number(&raw),
                    raw,
                },
            );
        }
    }
    let kind = detail
        .attrs
        .get("类型")
        .map(|a| a.raw.clone())
        .filter(|s| !s.is_empty() && s != "null")
        .ok_or("未解析到官网装备类型，保留原缓存")?;
    let level = detail
        .attrs
        .get("使用等级")
        .and_then(|a| a.num)
        .ok_or("未解析到使用等级，保留原缓存")?;
    let mut gear = Gear {
        kind,
        level: Some(level),
        ..Default::default()
    };
    static STAT: OnceLock<Regex> = OnceLock::new();
    let stat = STAT.get_or_init(|| {
        Regex::new(r"^([^+\-:：\d,，]{1,35}?)\s*([+-]\s*\d+(?:\.\d+)?)(%)?$").unwrap()
    });
    let mut source = "自身";
    for row in panel.select(&Selector::parse(".row").unwrap()).skip(1) {
        let mut raw = String::new();
        for node in row.descendants() {
            match node.value() {
                scraper::Node::Text(value) => raw.push_str(value),
                scraper::Node::Element(el) if el.name() == "br" => raw.push('\n'),
                _ => {}
            }
        }
        for line in raw.lines().map(str::trim).filter(|s| !s.is_empty()) {
            gear.raw.push(line.into());
            if line.starts_with("装备评分") {
                gear.score = line
                    .split(['：', ':'])
                    .nth(1)
                    .and_then(crate::parser::number);
                continue;
            }
            if line.contains("套装") {
                source = "套装";
            } else if line.contains("宝石效果")
                || line.contains("星石效果")
                || line.contains("强化石效果")
                || line.starts_with("魂技效果")
            {
                source = "附加";
            }
            if source == "套装" {
                gear.set.push(line.into());
                continue;
            }
            // Gem/star/set bonuses stay in their original section, never in equipment's own minima.
            if source == "附加" {
                gear.effects.push(line.into());
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            let normalized: String = chars
                .iter()
                .enumerate()
                .filter_map(|(i, ch)| {
                    let thousands = matches!(ch, ',' | '，')
                        && i > 0
                        && chars[i - 1].is_ascii_digit()
                        && chars
                            .get(i + 1..i + 4)
                            .is_some_and(|digits| digits.iter().all(char::is_ascii_digit))
                        && chars.get(i + 4).is_none_or(|next| !next.is_ascii_digit());
                    (!thousands).then_some(*ch)
                })
                .collect();
            for part in normalized
                .split([',', '，'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if let Some(c) = stat.captures(part) {
                    let value = c[2]
                        .replace(' ', "")
                        .parse::<f64>()
                        .map_err(|e| e.to_string())?;
                    if !value.is_finite() {
                        return Err("装备数值异常".into());
                    }
                    gear.stats.push((
                        c[1].trim().into(),
                        value,
                        c.get(3).map_or("", |m| m.as_str()).into(),
                    ));
                } else {
                    gear.effects.push(part.into());
                }
            }
        }
    }
    if gear.raw.is_empty() || gear.score.is_none() {
        return Err("装备属性或评分缺失，保留原缓存".into());
    }
    detail.gear = Some(gear);
    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_stats_are_preserved_and_gems_are_excluded() {
        let wing = parse(include_str!("../tests/gear-wing.html"))
            .unwrap()
            .gear
            .unwrap();
        assert_eq!(wing.kind, "凤翼");
        assert_eq!(
            wing.stats
                .iter()
                .filter(|s| s.0 == "攻击力" && s.1 == 40.0)
                .count(),
            2
        );
        let crest = parse(include_str!("../tests/gear-gems.html"))
            .unwrap()
            .gear
            .unwrap();
        assert_eq!(crest.stats.iter().filter(|s| s.0 == "力量").count(), 1);
        assert!(crest.effects.iter().any(|s| s.contains("力量+5")));
        assert!(parse("<html>已下架</html>").is_err());
    }
    #[test]
    fn thousands_separators_do_not_truncate_stats() {
        let html = include_str!("../tests/gear-wing.html").replace("攻击力 +40", "攻击力 +2,040");
        let gear = parse(&html).unwrap().gear.unwrap();
        assert_eq!(
            gear.stats
                .iter()
                .filter(|s| s.0 == "攻击力" && s.1 == 2040.0)
                .count(),
            2
        );
        assert!(gear.raw.iter().any(|s| s.contains("2,040")));
    }
    #[test]
    fn set_bonuses_are_not_equipment_stats() {
        let gear = parse(include_str!("../tests/gear-set.html"))
            .unwrap()
            .gear
            .unwrap();
        assert!(!gear.set.is_empty());
        assert_eq!(gear.stats.iter().filter(|s| s.0 == "SP").count(), 1);
    }
    #[test]
    #[ignore = "uses existing local official detail cache; no network"]
    fn all_cached_equipment_parses() {
        let dir = std::env::var("GEAR_DETAIL_CACHE").unwrap();
        let mut count = 0;
        for path in std::fs::read_dir(dir).unwrap() {
            let path = path.unwrap().path();
            if path.extension().is_some_and(|s| s == "html") {
                parse(&std::fs::read_to_string(&path).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                count += 1;
            }
        }
        assert!(count >= 657);
    }
}

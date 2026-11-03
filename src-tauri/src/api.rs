use crate::model::{now, valid_id, Area, Attr, Role};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub const BASE: &str = "http://cyg.changyou.com";

pub fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .connect_timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 CygFinder/0.1 (personal role search)")
        .build()
        .map_err(|e| e.to_string())
}

fn checked(value: Value) -> Result<Value, String> {
    if value["code"].as_i64() != Some(200) {
        return Err(format!("官网接口未成功：{}", value["code"]));
    }
    value
        .get("data")
        .filter(|v| v.is_object())
        .cloned()
        .ok_or("官网响应缺少 data".into())
}

pub async fn areas(client: &Client) -> Result<Vec<Area>, String> {
    let value: Value = client
        .get(format!("{BASE}/gameServer.json?gameId=16"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let data = checked(value)?;
    let areas: Vec<Area> =
        serde_json::from_value(data["server"].clone()).map_err(|e| e.to_string())?;
    if areas.is_empty()
        || areas
            .iter()
            .flat_map(|a| &a.server)
            .any(|s| !valid_id(&s.id))
    {
        return Err("区服数据为空或格式异常".into());
    }
    Ok(areas)
}

pub struct ListPage {
    pub roles: Vec<Role>,
    pub pages: usize,
    pub count: usize,
}

pub async fn list(
    client: &Client,
    server: &str,
    status: u8,
    page: usize,
    size: usize,
    goods_type: u8,
) -> Result<ListPage, String> {
    if ![1, 5].contains(&goods_type) {
        return Err("不支持的商品类型".into());
    }
    let value: Value = client.post(format!("{BASE}/list/filterSortGoods.json"))
        .header("Referer", format!("{BASE}/list"))
        .json(&json!({"filterList":[{"field":"gameId","value":"16","type":1},{"field":"goodsType","value":goods_type.to_string(),"type":1},{"field":"serverId","value":server},{"field":"status","value":status.to_string(),"type":1}],"gameServer":{"gameId":"16","serverId":server},"pageNumber":page,"pageSize":size}))
        .send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    decode_page(checked(value)?, server, goods_type)
}

fn decode_page(data: Value, server: &str, goods_type: u8) -> Result<ListPage, String> {
    let pages = data["totalPage"].as_u64().ok_or("列表缺少页数")? as usize;
    let count = data["totalCount"].as_u64().ok_or("列表缺少总数")? as usize;
    if pages > 1000 || count > 20000 {
        return Err("官网分页数量异常，停止以保护缓存".into());
    }
    let items = data["list"].as_array().ok_or("列表字段不是数组")?;
    let mut roles = Vec::new();
    for item in items {
        let g = &item["goods"];
        let string = |k: &str| {
            g[k].as_str()
                .map(String::from)
                .or_else(|| g[k].as_u64().map(|n| n.to_string()))
                .unwrap_or_default()
        };
        let code = string("goodsCode");
        let sid = string("serverId");
        if !valid_id(&code)
            || sid != server
            || g["gameId"].as_u64() != Some(16)
            || g["goodsType"].as_u64() != Some(goods_type.into())
        {
            return Err("官网返回了不属于当前区服或分类的商品，停止更新".into());
        }
        let price = g["price"].as_f64().filter(|v| v.is_finite() && *v >= 0.0);
        roles.push(Role {
            goods_type,
            list_attrs: g["attrs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|attr| {
                    let name = attr["name"].as_str()?;
                    let raw = attr["value"].as_str()?.to_string();
                    Some((
                        name.to_string(),
                        Attr {
                            num: crate::parser::number(&raw),
                            raw,
                        },
                    ))
                })
                .collect(),
            code,
            server_id: sid,
            name: string("goodsName"),
            title: string("title"),
            job: string("job"),
            price,
            status: g["webStatus"].as_u64().unwrap_or(0),
            listed_at: string("saleTime"),
            offline_at: string("offlineTime"),
            listing_updated_at: string("updateTime"),
            detail: None,
            detail_at: None,
            checked_at: now(),
            detail_error: None,
            needs_refresh: false,
        });
    }
    Ok(ListPage {
        roles,
        pages,
        count,
    })
}

pub async fn detail(
    client: &Client,
    code: &str,
    goods_type: u8,
) -> Result<crate::model::Detail, String> {
    if !valid_id(code) {
        return Err("商品编号格式错误".into());
    }
    // shortcut: the official detail host has an invalid HTTPS certificate; use its public HTTP endpoint until it supports valid TLS.
    let html = client
        .get(format!(
            "http://public.npt.changyou.com/details/16/{code}.html"
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    match goods_type {
        1 => crate::parser::parse(&html),
        5 => crate::parser::parse_spirit(&html),
        _ => Err("不支持的商品类型".into()),
    }
}

pub async fn unavailable(client: &Client, role: &Role) -> Result<bool, String> {
    if !valid_id(&role.code) || !valid_id(&role.server_id) || ![1, 5].contains(&role.goods_type) {
        return Err("商品编号或分类格式错误".into());
    }
    let html = client
        .get(format!("{BASE}/details?goodsCode={}", role.code))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    crate::parser::unavailable(&html, &role.code, &role.server_id, role.goods_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spirit_list_keeps_attributes_and_rejects_other_categories() {
        let response: Value = serde_json::from_str(include_str!("../tests/spirits.json")).unwrap();
        let data = checked(response).unwrap();
        let page = decode_page(data.clone(), "104", 5).unwrap();
        assert_eq!(page.roles.len(), 2);
        assert_eq!(page.roles[0].goods_type, 5);
        assert_eq!(page.roles[0].list_attrs["力量资质"].num, Some(2.73));
        assert_eq!(page.roles[1].list_attrs["慧心"].num, Some(111.0));
        assert!(decode_page(data.clone(), "104", 1).is_err());
        assert!(decode_page(data, "615", 5).is_err());
    }
    #[test]
    fn listing_timestamp_is_preserved_for_incremental_matching() {
        let mut response: Value =
            serde_json::from_str(include_str!("../tests/spirits.json")).unwrap();
        response["data"]["list"][0]["goods"]["updateTime"] = json!("2026-10-21 19:45:00");
        let page = decode_page(checked(response).unwrap(), "104", 5).unwrap();
        assert_eq!(page.roles[0].listing_updated_at, "2026-10-21 19:45:00");
        assert!(page.roles[1].listing_updated_at.is_empty());
    }
    #[tokio::test]
    #[ignore = "one public star list and detail request; run explicitly"]
    async fn live_spirit_sample() {
        let c = client().unwrap();
        let page = list(&c, "104", 2, 1, 2, 5).await.unwrap();
        assert!(!page.roles.is_empty());
        let detail = detail(&c, &page.roles[0].code, 5).await.unwrap();
        assert!(detail.attrs["攻击力"].num.is_some());
        println!("Live star sample OK: {} fields", detail.attrs.len());
    }
    #[test]
    fn invalid_responses_cannot_be_treated_as_empty_lists() {
        assert!(checked(json!({"code":500,"data":{}})).is_err());
        assert!(decode_page(json!({"totalPage":0,"totalCount":0}), "104", 1).is_err());
        assert!(decode_page(json!({"totalPage":1,"totalCount":1,"list":[{"goods":{"goodsCode":"123","serverId":615,"gameId":16,"goodsType":1}}]}),"104", 1).is_err());
        assert!(decode_page(json!({"totalPage":0,"totalCount":0,"list":[]}), "104", 1).is_ok());
    }
    #[tokio::test]
    #[ignore = "one public list page and one detail request; run explicitly"]
    async fn live_sample() {
        let c = client().unwrap();
        assert!(!areas(&c).await.unwrap().is_empty());
        let page = list(&c, "104", 2, 1, 2, 1).await.unwrap();
        assert!(!page.roles.is_empty());
        let d = detail(&c, &page.roles[0].code, 1).await.unwrap();
        assert!(d.attrs.len() > 10);
        println!(
            "Live sample OK: {} attributes; no full crawl",
            d.attrs.len()
        );
    }
}

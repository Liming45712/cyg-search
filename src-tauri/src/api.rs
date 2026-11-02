use crate::model::{now, valid_id, Area, Role};
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
) -> Result<ListPage, String> {
    let value: Value = client.post(format!("{BASE}/list/filterSortGoods.json"))
        .header("Referer", format!("{BASE}/list"))
        .json(&json!({"filterList":[{"field":"gameId","value":"16","type":1},{"field":"goodsType","value":"1","type":1},{"field":"serverId","value":server},{"field":"status","value":status.to_string(),"type":1}],"gameServer":{"gameId":"16","serverId":server},"pageNumber":page,"pageSize":size}))
        .send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    decode_page(checked(value)?, server)
}

fn decode_page(data: Value, server: &str) -> Result<ListPage, String> {
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
            || g["goodsType"].as_u64() != Some(1)
        {
            return Err("官网返回了不属于当前区服的角色或格式异常，停止更新".into());
        }
        let price = g["price"].as_f64().filter(|v| v.is_finite() && *v >= 0.0);
        roles.push(Role {
            code,
            server_id: sid,
            name: string("goodsName"),
            title: string("title"),
            job: string("job"),
            price,
            status: g["webStatus"].as_u64().unwrap_or(0),
            listed_at: string("saleTime"),
            offline_at: string("offlineTime"),
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

pub async fn detail(client: &Client, code: &str) -> Result<crate::model::Detail, String> {
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
    crate::parser::parse(&html)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_responses_cannot_be_treated_as_empty_lists() {
        assert!(checked(json!({"code":500,"data":{}})).is_err());
        assert!(decode_page(json!({"totalPage":0,"totalCount":0}), "104").is_err());
        assert!(decode_page(json!({"totalPage":1,"totalCount":1,"list":[{"goods":{"goodsCode":"123","serverId":615,"gameId":16,"goodsType":1}}]}),"104").is_err());
        assert!(decode_page(json!({"totalPage":0,"totalCount":0,"list":[]}), "104").is_ok());
    }
    #[tokio::test]
    #[ignore = "one public list page and one detail request; run explicitly"]
    async fn live_sample() {
        let c = client().unwrap();
        assert!(!areas(&c).await.unwrap().is_empty());
        let page = list(&c, "104", 2, 1, 2).await.unwrap();
        assert!(!page.roles.is_empty());
        let d = detail(&c, &page.roles[0].code).await.unwrap();
        assert!(d.attrs.len() > 10);
        println!(
            "Live sample OK: {} attributes; no full crawl",
            d.attrs.len()
        );
    }
}

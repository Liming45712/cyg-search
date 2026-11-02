# 畅易阁角色筛选
<!-- impeccable:product-schema 1 -->

## Platform
web

Tauri 桌面应用内的 Web 界面；当前交付平台为 macOS。

## Stack
Tauri 2 + Vue 3 + Element Plus + TypeScript + Rust + SQLite。用户明确要求使用 Element Plus 改进界面。

## Product Purpose
为新水浒Q传买家提供角色属性区间筛选、可排序的多列紧凑角色卡片、详情侧栏和增量缓存。

## Operating Context
从畅易阁公开接口获取数据，在本地筛选，购买时打开官网。原项目是机制参考，不复制其源代码。

## Capabilities and Constraints
首版先支持单个区服，默认侠客岛。拉取可取消、缓存可续抓；详情抓取失败保留旧值；未完整获取列表不删除旧数据。
服务端属性筛选口径未经验证，首版只按区服获取列表。收藏、通知、价格历史、Windows 打包不在首版范围。

---
name: db-schema-sync-rule
description: 数据库表结构变更时必须同时更新 db/tables.sql 与 db/tables_pgsql.sql 两个文件
metadata: 
  node_type: memory
  type: project
  originSessionId: 293e1827-f8b1-4a12-b8ab-f194730307a2
---

数据库表结构（schema）发生任何变更时，必须同时更新仓库中的两个 SQL 参考文件：
- `db/tables.sql` -- SQLite 版本，是项目本地数据库的 schema 来源真值（对应 `src-tauri/src/migration/*.rs` 全部迁移跑完后的最终态）
- `db/tables_pgsql.sql` -- PostgreSQL 镜像版本，需与 `tables.sql` 保持结构一致（列、索引、表），仅类型按 PostgreSQL 语法改写

**Why:** 项目运行时数据库仅用 SQLite（所有迁移均为 `DbBackend::Sqlite`，见 `src-tauri/src/migration/mod.rs`），但 `db/` 下额外维护了一份 PostgreSQL 镜像 schema 供参考/部署使用。两份文件无自动同步机制，历史上曾出现分歧（pgsql 落后或残留旧列），需人工保持一致。

**How to apply:** 当新增/修改 SeaORM 迁移文件（`src-tauri/src/migration/m*.rs`）或 `create_local_schema.rs` 导致表结构变化时，除迁移代码外，必须同步修改 `db/tables.sql` 与 `db/tables_pgsql.sql`。两个文件结构必须一一对应；pgsql 版本用 bigserial/varchar(N)/timestamp/text/boolean/smallint 等类型表达，索引名与组合方式与 SQLite 版本保持一致。相关后端结构见 [[tech-stack-backend]]。

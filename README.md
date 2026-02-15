<div align="center">
	<a href="https://github.com/kodaskills/diffly">
	    <img src="https://cdn-icons-png.flaticon.com/512/16465/16465057.png" alt="Diffly — Quickly compare your SQL data with clarity and style." />
	</a>

<h1>Diffly</h1>
<h2>Quickly compare your SQL data with clarity and style.</h2>

[![docs.rs](https://img.shields.io/docsrs/diffly)](https://docs.rs/diffly)
[![GitHub issues](https://img.shields.io/github/issues/kodaskills/diffly)](https://github.com/kodaskills/diffly/issues)
[![GitHub pull requests](https://img.shields.io/github/issues-pr/kodaskills/diffly)](https://github.com/kodaskills/diffly/pulls)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.0-4baaaa.svg)](CODE_OF_CONDUCT.md)

</div>

<h3 align="center">
⭐ Like what we're doing? Give us a star ⬆️
</h3>

</div>

**Diffly** is a Rust CLI and library for generating row-level data diffs between SQL databases such as PostgreSQL, MySQL, MariaDB, and SQLite. It produces human-readable tables with color-coded inserts, updates, deletes, and totals, as well as machine-readable JSON output for CI/CD pipelines, automated testing, or reporting. Perfect for QA, migrations, ETL verification, or auditing database changes.

## 👤 Who is Diffly for?

Diffly is built for developers, data engineers, and teams who need to **trust their data**, not just their schema.

It is especially useful for:

- Backend and platform engineers validating database migrations  
- Data engineers verifying ETL and replication pipelines  
- QA teams performing data-level regression testing  
- DevOps teams automating checks in CI/CD  
- Auditors and compliance teams tracking record-level changes  

If your system moves or transforms data between databases, Diffly helps ensure nothing was lost, altered, or corrupted.

## ⏱️ When should you use Diffly?

Use Diffly whenever you need to answer:

- *Did my data change?*  
- *Which rows changed?*  
- *Were rows inserted, updated, or deleted?*  
- *Did my migration or sync behave as expected?*

Typical scenarios include:

- After running a migration script  
- After syncing or replicating data  
- Before deploying to production  
- After importing or transforming datasets  
- When debugging inconsistencies between environments  

Diffly compares **actual row data**, not SQL dumps or schemas, and reports differences in both **human-readable tables** and **machine-readable JSON**.

## 🎯 What problem does Diffly solve?

Most tools focus on:

- schema diffs  
- SQL files  
- structure  

But production issues usually come from:

- missing rows  
- wrong values  
- duplicated data  
- partial imports  

Diffly focuses on what really matters:  
**the data itself.**

It acts as a safety net between:

```
source database → transformation / migration → target database
```

by validating the result, not just the process.

## 🧭 Positioning

> Diffly is to databases what `git diff` is to source code — but for data, not schema.


## 🧬 3-Way Merge and Conflict Detection

> Diffly can perform a 3-way merge to not only see the difference between a source and a target, but to understand how both have changed relative to a common ancestor, making it easy to detect conflicting changes.

This process works in two steps:

### Step 1: Create a Snapshot

First, create a snapshot of your database when it's in a known "base" or "common ancestor" state. This command inspects the database defined in your config and saves its state to a snapshot directory.

```bash
# Create a snapshot from the database configured as `target`
diffly snapshot --config ./diffly.toml
```

### Step 2: Check for Conflicts

Later, after changes have been made to your databases, run the `check-conflicts` command. This compares your current `source` and `target` databases against the snapshot to identify divergent changes.

```bash
# Compare the current source and target databases against the snapshot
diffly check-conflicts --config ./diffly.toml --snapshot ./my-snapshot
```

The result is a rich diff that shows you not just the final state, but how you got there, clearly identifying rows that were changed in the source, the target, or even conflicting changes made to both.

🚨 **IMPORTANT — SNAPSHOT CONSISTENCY REQUIRED**

Diffly’s conflict detection is only valid if the snapshot reflects the **exact state** of the target database at snapshot time.

If the target database changes after the snapshot is created:

- conflicts may be missed  
- valid changes may be flagged incorrectly  
- generated diffs may be unsafe to apply  

**Always ensure one of the following:**

✅ Snapshot is taken immediately before diff  
✅ Target database is locked or frozen during snapshot  
✅ No writes occur between snapshot and diff

This constraint is fundamental to safe 3-way merge behavior.

## 🏛️ Architecture

```mermaid
---
config:
  theme: neutral
  look: handDrawn
---
flowchart LR
    SDB[(Source DB)]
    TDB[(Target DB)]
    Diffly[Diffly<br/>Diff Engine]
    
    subgraph DIFFLY_ENGINE["Diffly Engine"]
      direction TB
      DIFF[Diff<br/>Command]
      JSON[Diff JSON Format]
      SQL[Migration Atomic SQL]
      HTML[Diff Report HTML]
      subgraph MERGE["3 Way Merge"]
          SNAP[Snapshot<br/>Command JSON]
          SNAPJSON[Snapshot JSON]
          CONFLICTS[Check-Conflicts<br/>Command]
      end
    end

    SDB -->|Fetch Info| Diffly
    TDB -->|Fetch Info| Diffly

    Diffly -->|diff| DIFF
    DIFF -->|generates| JSON
    DIFF -->|generates| SQL
    DIFF -->|generates| HTML

    Diffly -->|snapshot| SNAP
    SNAP -->|generates| SNAPJSON
    Diffly -->|check-conflicts| CONFLICTS
    CONFLICTS -->|Uses| SNAPJSON

    style SDB fill:#e1f5ff
    style TDB fill:#f3e5f5
    style Diffly fill:#fff3e0
    style DIFF fill:#fffde7
    style JSON fill:#e8f5e9
    style SQL fill:#fce4ec
    style HTML fill:#f1f8e9
    style SNAP fill:#e3fcec
    style CONFLICTS fill:#ffe0e0
    style SNAPJSON fill:#dcedc8
    style DIFFLY_ENGINE fill:#f7fafc,stroke:#aaa,stroke-width:2px
    style MERGE fill:#e1f0fa,stroke:#7dc5f3,stroke-width:2px
```

## 🌿 Dialects

Today, **Diffly** is implements these dialects:

- <img src="https://www.vectorlogo.zone/logos/postgresql/postgresql-icon.svg"  alt="Postgresql"  width="20"  height="20"> Postgresql
- <img src="https://www.vectorlogo.zone/logos/mysql/mysql-icon.svg"  alt="Mysql"  width="20"  height="20"> Mysql
- <img src="https://www.vectorlogo.zone/logos/mariadb/mariadb-icon.svg"  alt="MariaDB"  width="20"  height="20"> MariaDB
- <img src="https://www.vectorlogo.zone/logos/sqlite/sqlite-icon.svg"  alt="Sqlite"  width="20"  height="20"> Sqlite

Please feel free to fix or add new dialect by forking this repository and use a pull request, we ❤️ having feedbacks and [contributions](CODE_OF_CONDUCT.md)!

## ⚡ Quick Start

### 🐳 Run all once as cli for a diff in Docker

```bash
# Start PostgreSQL + fixtures import + diff execution
docker compose -f examples/postgresql/docker-compose.yml up --build

# Results are exported into the ./output/ root directory
ls ./output/
#  cs_20260211_*.json   ← Strucutred changeset
#  cs_20260211_*.sql    ← Atomic migration SQL (BEGIN/COMMIT)
#  cs_20260211_*.html   ← Visual report (open in a browser)

# To reset
docker compose -f examples/postgresql/docker-compose.yml down -v
```

### 📍 Run as cli in local (with cargo)

```bash
# 1. Run PostgreSQL alone if you do not have a local base (optional)
docker compose -f examples/postgresql/docker-compose.yml up postgres -d

# 2. Copy and paste postgresql config example and change the settings you need
# For example as is you must change host of 2 the sources to localhost (or your own)
cp ./examples/postgresql/config.toml ./my-config.toml

# 3. Build + run
cargo run --features cli -- --config ./my-config diff

# 4. Dry run (cli summary only, pas de fichiers)
cargo run --features cli -- --config ./my-config --dry-run diff

# 5. Only one format (json, html, sql)
cargo run --features cli -- --config ./my-config --format html diff 
```

### 📚 Run as library

```bash
# 1. Run PostgreSQL alone if you do not have a local base (optional)
docker compose -f examples/postgresql/docker-compose.yml up postgres -d

# 2. Copy and paste postgresql config example and change the settings you need
# For example as is you must change host of 2 the sources to localhost (or your own)
cp ./examples/postgresql/config.toml ./my-config.toml

# 3. Run
cargo run --example diff_as_lib -- ./my-config.toml
```

## 🔧 Base configuration (config.toml)

```toml
[source]
host = "localhost"
port = 5432
dbname = "diffly"
user = "diffly"
password = "diffly"
schema = "sandbox_admin1"    # Sandbox schema

[target]
host = "localhost"
port = 5432
dbname = "diffly"
user = "diffly"
password = "diffly"
schema = "staging"           # Target schema

[diff]
[[diff.tables]]
name = "pricing_rules"       # Table to compare
primary_key = ["id"]         # Simple PK

[[diff.tables]]
name = "tax_rules"
primary_key = ["region_code", "product_category"]  # Composite PK

[output]
dir = "./output"
```

**Note**: SQLite has no schema concept. Source and target must be **separate database files**. The `schema` config field is ignored.

## ➡️ Outputs

Actually **Diffly** generates 3 formats:

### JSON
Complete Changeset  with `before`/`after` for each modification, PK, modified columns, resume and the sql query to apply for each modification (be careful to run them inside an **atomic transaction**).

### SQL
Atomic transaction  `BEGIN`/`COMMIT` with `DELETE` → `UPDATE` → `INSERT` (secure order), useful for data migration.

The generated `.sql` diff file adapts to the target driver automatically:

| Driver | Identifier quoting | JSON literal |
|---|---|---|
| `postgres` | `"double_quotes"` | `'...'::jsonb` |
| `mysql` / `mariadb` | `` `backticks` `` | `'...'` |
| `sqlite` | `"double_quotes"` | `'...'` |


### HTML
Visual report with dark/light-mode made for humans.

We use [Sailfish](https://rust-sailfish.github.io/sailfish/) templating internally for better isolation. You can change the way the template engine works by modifying the `sailfish.toml` file on root directory if necessary  ([see configuration](https://rust-sailfish.github.io/sailfish/options/) for more informations).

## 🈂️ Examples

For more examples, visit our [examples repository](https://github.com/kodaskills/diffly/tree/main/examples).

## 🤝 Contribution

We welcome contributions! Please refer to the contribution guidelines (link needed if available). Join our [Github Discussions](https://github.com/kodaskills/diffly)  for questions and discussions.

## ♥️ Contributor Thanks

Big thanks to everyone who's been part of the **Diffly** journey, whether you've built a plugin, opened an issue, dropped a pull request, or just helped someone out on GitHub Discussions.

**Diffly** is a community effort, and it keeps getting better because of people like you.

<!--![Contributors](https://contrib.rocks/image?repo=kodaskills/diffly&max=500&columns=20&anon=1)-->

## 📄 License

Licensed under the MIT License, Copyright © 2026-present **Diffly**.

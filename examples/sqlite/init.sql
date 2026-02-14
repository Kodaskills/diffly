-- ============================================================
-- DIFFLY — SQLite Test Fixtures
-- ============================================================
-- SQLite has no schemas or databases; diffly uses the `schema`
-- config field as a table prefix: `{schema}_{table}`.
-- We simulate two "schemas" by prefixing table names:
--   source_* = source tables (admin modifications)
--   target_* = target tables (current state)
--
-- Diffly's SQLite support: schema is ignored (empty prefix),
-- so source and target must be separate database FILES.
-- This fixture is split into two files:
--   init_sqlite_source.sql  → loaded into source.db
--   init_sqlite_target.sql  → loaded into target.db
--
-- Run with:
--   sqlite3 source.db < init_sqlite_source.sql
--   sqlite3 target.db < init_sqlite_target.sql
-- ============================================================
-- This file is the TARGET database (current state).
-- See init_sqlite_source.sql for the source (admin modifications).
-- ============================================================

-- ============================================================
-- TABLE 1: pricing_rules
-- Scenarios: INSERT, UPDATE, DELETE, unchanged rows
-- ============================================================

CREATE TABLE IF NOT EXISTS pricing_rules (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_name     TEXT    NOT NULL,
    product_type  TEXT    NOT NULL,
    -- SQLite stores DECIMAL as REAL; TEXT preserves exact precision
    discount_rate REAL    NOT NULL,
    min_quantity  INTEGER NOT NULL DEFAULT 1,
    max_quantity  INTEGER,
    -- SQLite has no BOOLEAN: 1=TRUE, 0=FALSE
    is_active     INTEGER NOT NULL DEFAULT 1,
    -- JSON stored as TEXT in SQLite
    metadata      TEXT
);

INSERT INTO pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    (1, 'Early Bird Discount',  'electronics', 0.1,  1,  NULL, 1, '{"campaign":"spring_2026"}'),
    (2, 'Bulk Purchase Silver', 'electronics', 0.15, 10, 50,   1, '{"tier":"silver"}'),
    (3, 'Bulk Purchase Gold',   'electronics', 0.20, 51, 200,  1, '{"tier":"gold"}'),
    (4, 'Seasonal Clearance',   'clothing',    0.30, 1,  NULL, 1, '{"season":"winter"}'),
    (5, 'VIP Member Discount',  'all',         0.05, 1,  NULL, 1, '{"membership":"vip"}'),
    -- Row 6: will be DELETED (exists only in target)
    (6, 'Flash Sale Weekend',   'accessories', 0.25, 1,  NULL, 0, '{"expired":true}'),
    (7, 'Student Discount',     'books',       0.12, 1,  NULL, 1, '{"requires_id":true}');

-- ============================================================
-- TABLE 2: discount_tiers
-- Scenario: UPDATE only
-- ============================================================

CREATE TABLE IF NOT EXISTS discount_tiers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    tier_name    TEXT NOT NULL,
    min_spend    REAL NOT NULL,
    max_spend    REAL,
    discount_pct REAL NOT NULL,
    applies_to   TEXT NOT NULL DEFAULT 'all',
    is_active    INTEGER NOT NULL DEFAULT 1
);

INSERT INTO discount_tiers (id, tier_name, min_spend, max_spend, discount_pct, applies_to) VALUES
    (1, 'Bronze',    50.0,   199.99,  2.0,  'all'),
    (2, 'Silver',   200.0,   499.99,  5.0,  'all'),
    (3, 'Gold',     500.0,   999.99,  8.0,  'all'),
    (4, 'Platinum', 1000.0,  NULL,    12.0, 'all');

-- ============================================================
-- TABLE 3: shipping_rules
-- Scenario: INSERT only
-- ============================================================

CREATE TABLE IF NOT EXISTS shipping_rules (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    zone_name     TEXT    NOT NULL,
    country_code  TEXT    NOT NULL,
    base_cost     REAL    NOT NULL,
    free_above    REAL,
    delivery_days INTEGER NOT NULL,
    carrier       TEXT    NOT NULL,
    is_active     INTEGER NOT NULL DEFAULT 1
);

INSERT INTO shipping_rules (id, zone_name, country_code, base_cost, free_above, delivery_days, carrier) VALUES
    (1, 'France Metro',     'FR', 4.99, 50.0,  2, 'Colissimo'),
    (2, 'Germany Standard', 'DE', 6.99, 75.0,  3, 'DHL'),
    (3, 'UK Express',       'GB', 9.99, 100.0, 2, 'Royal Mail');

-- ============================================================
-- TABLE 4: tax_rules
-- Scenario: DELETE only — composite primary key
-- ============================================================

CREATE TABLE IF NOT EXISTS tax_rules (
    region_code      TEXT NOT NULL,
    product_category TEXT NOT NULL,
    tax_rate         REAL NOT NULL,
    tax_name         TEXT NOT NULL,
    effective_from   TEXT NOT NULL,  -- ISO 8601 date as TEXT
    effective_to     TEXT,
    is_active        INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (region_code, product_category)
);

INSERT INTO tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR',    'electronics',  0.20,  'TVA Standard',           '2024-01-01', NULL,         1),
    ('FR',    'food',         0.055, 'TVA Réduit',             '2024-01-01', NULL,         1),
    ('FR',    'books',        0.055, 'TVA Réduit Livres',      '2024-01-01', NULL,         1),
    -- These 3 will be DELETED:
    ('FR-CO', 'electronics',  0.17,  'TVA Corse Ancien',       '2020-01-01', '2023-12-31', 0),
    ('FR-CO', 'food',         0.021, 'TVA Corse Réduit Ancien','2020-01-01', '2023-12-31', 0),
    ('FR-GP', 'all',          0.085, 'Octroi de Mer Ancien',   '2019-01-01', '2023-06-30', 0);

-- ============================================================
-- TABLE 5: no_change_rules
-- Scenario: identical — diff must be empty
-- ============================================================

CREATE TABLE IF NOT EXISTS no_change_rules (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_name TEXT NOT NULL,
    value     REAL NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1
);

INSERT INTO no_change_rules (id, rule_name, value, is_active) VALUES
    (1, 'Identical Rule A', 10.0, 1),
    (2, 'Identical Rule B', 20.0, 0),
    (3, 'Identical Rule C', 30.0, 1);

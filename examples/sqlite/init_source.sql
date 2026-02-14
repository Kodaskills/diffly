-- ============================================================
-- DIFFLY — SQLite Source Fixtures (admin modifications)
-- ============================================================
-- Load into source.db:
--   sqlite3 source.db < init_sqlite_source.sql
-- ============================================================

-- ============================================================
-- TABLE 1: pricing_rules (source — admin modifications)
-- ============================================================

CREATE TABLE IF NOT EXISTS pricing_rules (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_name     TEXT    NOT NULL,
    product_type  TEXT    NOT NULL,
    discount_rate REAL    NOT NULL,
    min_quantity  INTEGER NOT NULL DEFAULT 1,
    max_quantity  INTEGER,
    is_active     INTEGER NOT NULL DEFAULT 1,
    metadata      TEXT
);

INSERT INTO pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    -- Row 1: UNCHANGED
    (1, 'Early Bird Discount',     'electronics', 0.1,  1,  NULL, 1, '{"campaign":"spring_2026"}'),
    -- Row 2: UPDATE — discount_rate 0.15→0.18, min_quantity 10→5
    (2, 'Bulk Purchase Silver',    'electronics', 0.18, 5,  50,   1, '{"tier":"silver"}'),
    -- Row 3: UPDATE — max_quantity 200→500, metadata updated
    (3, 'Bulk Purchase Gold',      'electronics', 0.20, 51, 500,  1, '{"tier":"gold","note":"expanded range"}'),
    -- Row 4: UPDATE — is_active 1→0
    (4, 'Seasonal Clearance',      'clothing',    0.30, 1,  NULL, 0, '{"season":"winter"}'),
    -- Row 5: UNCHANGED
    (5, 'VIP Member Discount',     'all',         0.05, 1,  NULL, 1, '{"membership":"vip"}'),
    -- Row 6: DELETED (absent from source)
    -- Row 7: UNCHANGED
    (7, 'Student Discount',        'books',       0.12, 1,  NULL, 1, '{"requires_id":true}'),
    -- Row 8: INSERT
    (8, 'New Customer Welcome',    'all',         0.10, 1,  NULL, 1, '{"first_order_only":true}'),
    -- Row 9: INSERT
    (9, 'Bundle Deal Electronics', 'electronics', 0.22, 3,  10,   1, '{"requires_bundle":true}');

-- ============================================================
-- TABLE 2: discount_tiers (source — admin modifications)
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
    (1, 'Bronze',    50.0,   199.99,  2.0,  'all'),  -- UNCHANGED
    (2, 'Silver',   200.0,   499.99,  5.0,  'all'),  -- UNCHANGED
    (3, 'Gold',     500.0,   999.99,  10.0, 'all'),  -- UPDATE: discount 8→10
    (4, 'Platinum', 1000.0,  NULL,    15.0, 'all');  -- UPDATE: discount 12→15

-- ============================================================
-- TABLE 3: shipping_rules (source — admin adds 3 new zones)
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
    (1, 'France Metro',     'FR', 4.99, 50.0,  2, 'Colissimo'),   -- UNCHANGED
    (2, 'Germany Standard', 'DE', 6.99, 75.0,  3, 'DHL'),         -- UNCHANGED
    (3, 'UK Express',       'GB', 9.99, 100.0, 2, 'Royal Mail'),  -- UNCHANGED
    -- INSERTs:
    (4, 'Spain Standard',   'ES', 7.49, 60.0,  4, 'Correos'),
    (5, 'Italy Express',    'IT', 8.99, 70.0,  3, 'Poste Italiane'),
    (6, 'Belgium Quick',    'BE', 5.49, 45.0,  1, 'bpost');

-- ============================================================
-- TABLE 4: tax_rules (source — admin removes 3 deprecated rules)
-- ============================================================

CREATE TABLE IF NOT EXISTS tax_rules (
    region_code      TEXT NOT NULL,
    product_category TEXT NOT NULL,
    tax_rate         REAL NOT NULL,
    tax_name         TEXT NOT NULL,
    effective_from   TEXT NOT NULL,
    effective_to     TEXT,
    is_active        INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (region_code, product_category)
);

INSERT INTO tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR', 'electronics', 0.20,  'TVA Standard',      '2024-01-01', NULL, 1),
    ('FR', 'food',        0.055, 'TVA Réduit',        '2024-01-01', NULL, 1),
    ('FR', 'books',       0.055, 'TVA Réduit Livres', '2024-01-01', NULL, 1);

-- ============================================================
-- TABLE 5: no_change_rules (source — identical to target)
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

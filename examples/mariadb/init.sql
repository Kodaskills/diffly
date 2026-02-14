-- ============================================================
-- DIFFLY — MySQL / MariaDB Test Fixtures
-- ============================================================
-- Creates two databases (source_db + target_db) with controlled
-- differences to exercise every diff scenario.
--
-- Differences vs PostgreSQL fixtures:
--   - Uses databases instead of schemas
--   - INT AUTO_INCREMENT instead of SERIAL
--   - DECIMAL instead of NUMERIC
--   - JSON instead of JSONB (MySQL 5.7+ / MariaDB 10.2+)
--   - TINYINT(1) for BOOLEAN (MySQL convention)
--   - VARCHAR(2) instead of CHAR(2)
--   - No ::TEXT casts needed (diffly handles MySQL natively)
-- ============================================================

CREATE DATABASE IF NOT EXISTS source_db CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE IF NOT EXISTS target_db CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- Grant the diffly user access to both databases
-- (MYSQL_USER only gets access to MYSQL_DATABASE by default, which we don't set)
GRANT ALL PRIVILEGES ON source_db.* TO 'diffly'@'%';
GRANT ALL PRIVILEGES ON target_db.* TO 'diffly'@'%';
FLUSH PRIVILEGES;

-- ============================================================
-- TABLE 1: pricing_rules
-- Scenarios: INSERT, UPDATE, DELETE, and unchanged rows
-- ============================================================

CREATE TABLE target_db.pricing_rules (
    id            INT AUTO_INCREMENT PRIMARY KEY,
    rule_name     VARCHAR(100) NOT NULL,
    product_type  VARCHAR(50)  NOT NULL,
    discount_rate DECIMAL(5,4) NOT NULL,
    min_quantity  INT          NOT NULL DEFAULT 1,
    max_quantity  INT,
    is_active     TINYINT(1)   NOT NULL DEFAULT 1,
    metadata      JSON
);

CREATE TABLE source_db.pricing_rules (
    id            INT AUTO_INCREMENT PRIMARY KEY,
    rule_name     VARCHAR(100) NOT NULL,
    product_type  VARCHAR(50)  NOT NULL,
    discount_rate DECIMAL(5,4) NOT NULL,
    min_quantity  INT          NOT NULL DEFAULT 1,
    max_quantity  INT,
    is_active     TINYINT(1)   NOT NULL DEFAULT 1,
    metadata      JSON
);

-- Target data (current state)
INSERT INTO target_db.pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    (1, 'Early Bird Discount',  'electronics', 0.1000, 1,  NULL, 1, '{"campaign": "spring_2026"}'),
    (2, 'Bulk Purchase Silver', 'electronics', 0.1500, 10, 50,   1, '{"tier": "silver"}'),
    (3, 'Bulk Purchase Gold',   'electronics', 0.2000, 51, 200,  1, '{"tier": "gold"}'),
    (4, 'Seasonal Clearance',   'clothing',    0.3000, 1,  NULL, 1, '{"season": "winter"}'),
    (5, 'VIP Member Discount',  'all',         0.0500, 1,  NULL, 1, '{"membership": "vip"}'),
    -- Row 6: will be DELETED (exists only in target)
    (6, 'Flash Sale Weekend',   'accessories', 0.2500, 1,  NULL, 0, '{"expired": true}'),
    (7, 'Student Discount',     'books',       0.1200, 1,  NULL, 1, '{"requires_id": true}');

-- Source data (admin modifications)
INSERT INTO source_db.pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    -- Row 1: UNCHANGED
    (1, 'Early Bird Discount',     'electronics', 0.1000, 1,  NULL, 1, '{"campaign": "spring_2026"}'),
    -- Row 2: UPDATE — discount_rate 0.15→0.18, min_quantity 10→5
    (2, 'Bulk Purchase Silver',    'electronics', 0.1800, 5,  50,   1, '{"tier": "silver"}'),
    -- Row 3: UPDATE — max_quantity 200→500, metadata updated
    (3, 'Bulk Purchase Gold',      'electronics', 0.2000, 51, 500,  1, '{"tier": "gold", "note": "expanded range"}'),
    -- Row 4: UPDATE — is_active flipped 1→0
    (4, 'Seasonal Clearance',      'clothing',    0.3000, 1,  NULL, 0, '{"season": "winter"}'),
    -- Row 5: UNCHANGED
    (5, 'VIP Member Discount',     'all',         0.0500, 1,  NULL, 1, '{"membership": "vip"}'),
    -- Row 6: DELETED (absent from source)
    -- Row 7: UNCHANGED
    (7, 'Student Discount',        'books',       0.1200, 1,  NULL, 1, '{"requires_id": true}'),
    -- Row 8: INSERT
    (8, 'New Customer Welcome',    'all',         0.1000, 1,  NULL, 1, '{"first_order_only": true}'),
    -- Row 9: INSERT
    (9, 'Bundle Deal Electronics', 'electronics', 0.2200, 3,  10,   1, '{"requires_bundle": true}');

-- ============================================================
-- TABLE 2: discount_tiers
-- Scenario: UPDATE only
-- ============================================================

CREATE TABLE target_db.discount_tiers (
    id           INT AUTO_INCREMENT PRIMARY KEY,
    tier_name    VARCHAR(50)   NOT NULL,
    min_spend    DECIMAL(10,2) NOT NULL,
    max_spend    DECIMAL(10,2),
    discount_pct DECIMAL(5,2)  NOT NULL,
    applies_to   VARCHAR(50)   NOT NULL DEFAULT 'all',
    is_active    TINYINT(1)    NOT NULL DEFAULT 1
);

CREATE TABLE source_db.discount_tiers LIKE target_db.discount_tiers;

INSERT INTO target_db.discount_tiers (id, tier_name, min_spend, max_spend, discount_pct, applies_to) VALUES
    (1, 'Bronze',   50.00,   199.99,  2.00,  'all'),
    (2, 'Silver',   200.00,  499.99,  5.00,  'all'),
    (3, 'Gold',     500.00,  999.99,  8.00,  'all'),
    (4, 'Platinum', 1000.00, NULL,    12.00, 'all');

INSERT INTO source_db.discount_tiers (id, tier_name, min_spend, max_spend, discount_pct, applies_to) VALUES
    (1, 'Bronze',   50.00,   199.99,  2.00,  'all'),  -- UNCHANGED
    (2, 'Silver',   200.00,  499.99,  5.00,  'all'),  -- UNCHANGED
    (3, 'Gold',     500.00,  999.99,  10.00, 'all'),  -- UPDATE: discount 8→10
    (4, 'Platinum', 1000.00, NULL,    15.00, 'all');  -- UPDATE: discount 12→15

-- ============================================================
-- TABLE 3: shipping_rules
-- Scenario: INSERT only
-- ============================================================

CREATE TABLE target_db.shipping_rules (
    id             INT AUTO_INCREMENT PRIMARY KEY,
    zone_name      VARCHAR(100)  NOT NULL,
    country_code   VARCHAR(2)    NOT NULL,
    base_cost      DECIMAL(8,2)  NOT NULL,
    free_above     DECIMAL(8,2),
    delivery_days  INT           NOT NULL,
    carrier        VARCHAR(50)   NOT NULL,
    is_active      TINYINT(1)    NOT NULL DEFAULT 1
);

CREATE TABLE source_db.shipping_rules LIKE target_db.shipping_rules;

INSERT INTO target_db.shipping_rules (id, zone_name, country_code, base_cost, free_above, delivery_days, carrier) VALUES
    (1, 'France Metro',     'FR', 4.99, 50.00,  2, 'Colissimo'),
    (2, 'Germany Standard', 'DE', 6.99, 75.00,  3, 'DHL'),
    (3, 'UK Express',       'GB', 9.99, 100.00, 2, 'Royal Mail');

INSERT INTO source_db.shipping_rules (id, zone_name, country_code, base_cost, free_above, delivery_days, carrier) VALUES
    (1, 'France Metro',     'FR', 4.99, 50.00,  2, 'Colissimo'),   -- UNCHANGED
    (2, 'Germany Standard', 'DE', 6.99, 75.00,  3, 'DHL'),         -- UNCHANGED
    (3, 'UK Express',       'GB', 9.99, 100.00, 2, 'Royal Mail'),  -- UNCHANGED
    -- INSERTs:
    (4, 'Spain Standard',   'ES', 7.49, 60.00,  4, 'Correos'),
    (5, 'Italy Express',    'IT', 8.99, 70.00,  3, 'Poste Italiane'),
    (6, 'Belgium Quick',    'BE', 5.49, 45.00,  1, 'bpost');

-- ============================================================
-- TABLE 4: tax_rules
-- Scenario: DELETE only — composite primary key
-- ============================================================

CREATE TABLE target_db.tax_rules (
    region_code      VARCHAR(10)  NOT NULL,
    product_category VARCHAR(50)  NOT NULL,
    tax_rate         DECIMAL(5,4) NOT NULL,
    tax_name         VARCHAR(100) NOT NULL,
    effective_from   DATE         NOT NULL,
    effective_to     DATE,
    is_active        TINYINT(1)   NOT NULL DEFAULT 1,
    PRIMARY KEY (region_code, product_category)
);

CREATE TABLE source_db.tax_rules LIKE target_db.tax_rules;

INSERT INTO target_db.tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR',    'electronics',  0.2000, 'TVA Standard',           '2024-01-01', NULL,         1),
    ('FR',    'food',         0.0550, 'TVA Réduit',             '2024-01-01', NULL,         1),
    ('FR',    'books',        0.0550, 'TVA Réduit Livres',      '2024-01-01', NULL,         1),
    -- These 3 will be DELETED:
    ('FR-CO', 'electronics',  0.1700, 'TVA Corse Ancien',       '2020-01-01', '2023-12-31', 0),
    ('FR-CO', 'food',         0.0210, 'TVA Corse Réduit Ancien','2020-01-01', '2023-12-31', 0),
    ('FR-GP', 'all',          0.0850, 'Octroi de Mer Ancien',   '2019-01-01', '2023-06-30', 0);

INSERT INTO source_db.tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR', 'electronics', 0.2000, 'TVA Standard',      '2024-01-01', NULL, 1),
    ('FR', 'food',        0.0550, 'TVA Réduit',        '2024-01-01', NULL, 1),
    ('FR', 'books',       0.0550, 'TVA Réduit Livres', '2024-01-01', NULL, 1);

-- ============================================================
-- TABLE 5: no_change_rules
-- Scenario: identical — diff must be empty
-- ============================================================

CREATE TABLE target_db.no_change_rules (
    id        INT AUTO_INCREMENT PRIMARY KEY,
    rule_name VARCHAR(100)  NOT NULL,
    value     DECIMAL(10,2) NOT NULL,
    is_active TINYINT(1)    NOT NULL DEFAULT 1
);

CREATE TABLE source_db.no_change_rules LIKE target_db.no_change_rules;

INSERT INTO target_db.no_change_rules (id, rule_name, value, is_active) VALUES
    (1, 'Identical Rule A', 10.00, 1),
    (2, 'Identical Rule B', 20.00, 0),
    (3, 'Identical Rule C', 30.00, 1);

INSERT INTO source_db.no_change_rules (id, rule_name, value, is_active) VALUES
    (1, 'Identical Rule A', 10.00, 1),
    (2, 'Identical Rule B', 20.00, 0),
    (3, 'Identical Rule C', 30.00, 1);

-- ============================================================
-- TABLE 6: nullable_rules
-- Scenario: NULL value transitions
-- ============================================================

CREATE TABLE target_db.nullable_rules (
    id         INT AUTO_INCREMENT PRIMARY KEY,
    rule_name  VARCHAR(100) NOT NULL,
    optional_a VARCHAR(100),
    optional_b DECIMAL(10,2),
    notes      TEXT
);

CREATE TABLE source_db.nullable_rules LIKE target_db.nullable_rules;

INSERT INTO target_db.nullable_rules (id, rule_name, optional_a, optional_b, notes) VALUES
    (1, 'Nullable Test A', 'foo', NULL, 'original'),
    (2, 'Nullable Test B', NULL,  5.00, 'some text');

INSERT INTO source_db.nullable_rules (id, rule_name, optional_a, optional_b, notes) VALUES
    -- Row 1: optional_a→NULL, optional_b→99
    (1, 'Nullable Test A', NULL, 99.00, 'original'),
    -- Row 2: notes→NULL
    (2, 'Nullable Test B', NULL,  5.00, NULL);

-- ============================================================
-- TABLE 7: quoted_rules
-- Scenario: special characters — SQL escaping
-- ============================================================

CREATE TABLE target_db.quoted_rules (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    rule_name   VARCHAR(200) NOT NULL,
    description TEXT
);

CREATE TABLE source_db.quoted_rules LIKE target_db.quoted_rules;

INSERT INTO target_db.quoted_rules (id, rule_name, description) VALUES
    (1, "Rule with 'single' quotes", "It's a test with apostrophe"),
    (2, 'Rule unchanged',            'No special chars');

INSERT INTO source_db.quoted_rules (id, rule_name, description) VALUES
    -- Row 1: UPDATE — description changes
    (1, "Rule with 'single' quotes", 'New desc with\nnewline and "double" quotes & <html>'),
    -- Row 2: UNCHANGED
    (2, 'Rule unchanged',            'No special chars'),
    -- Row 3: INSERT
    (3, 'Rule with backslash',       'Path: C:\\Users\\admin');

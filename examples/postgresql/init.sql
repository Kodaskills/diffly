-- ============================================================
-- DIFFLY DIFF ENGINE — Test Fixtures
-- ============================================================
-- Creates two schemas (staging + sandbox) with controlled
-- differences to exercise every diff scenario.
-- ============================================================

-- ─── Setup Schemas ───
CREATE SCHEMA IF NOT EXISTS staging;
CREATE SCHEMA IF NOT EXISTS sandbox_admin1;

-- ============================================================
-- TABLE 1: pricing_rules
-- Scenarios: INSERT, UPDATE, DELETE, and unchanged rows
-- ============================================================

CREATE TABLE staging.pricing_rules (
    id              SERIAL PRIMARY KEY,
    rule_name       VARCHAR(100) NOT NULL,
    product_type    VARCHAR(50) NOT NULL,
    discount_rate   NUMERIC(5,4) NOT NULL,
    min_quantity    INTEGER NOT NULL DEFAULT 1,
    max_quantity    INTEGER,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    metadata        JSONB,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE sandbox_admin1.pricing_rules (LIKE staging.pricing_rules INCLUDING ALL);

-- Staging data (the "target" — current state)
INSERT INTO staging.pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    (1, 'Early Bird Discount',     'electronics', 0.1000, 1,   NULL, TRUE,  '{"campaign": "spring_2026"}'),
    (2, 'Bulk Purchase Silver',    'electronics', 0.1500, 10,  50,   TRUE,  '{"tier": "silver"}'),
    (3, 'Bulk Purchase Gold',      'electronics', 0.2000, 51,  200,  TRUE,  '{"tier": "gold"}'),
    (4, 'Seasonal Clearance',      'clothing',    0.3000, 1,   NULL, TRUE,  '{"season": "winter"}'),
    (5, 'VIP Member Discount',     'all',         0.0500, 1,   NULL, TRUE,  '{"membership": "vip"}'),
    -- Row 6 will be DELETED in sandbox (exists only in staging)
    (6, 'Flash Sale Weekend',      'accessories', 0.2500, 1,   NULL, FALSE, '{"expired": true}'),
    (7, 'Student Discount',        'books',       0.1200, 1,   NULL, TRUE,  '{"requires_id": true}');

-- Sandbox data (the "source" — admin's modifications)
INSERT INTO sandbox_admin1.pricing_rules (id, rule_name, product_type, discount_rate, min_quantity, max_quantity, is_active, metadata) VALUES
    -- Row 1: UNCHANGED
    (1, 'Early Bird Discount',     'electronics', 0.1000, 1,   NULL, TRUE,  '{"campaign": "spring_2026"}'),
    -- Row 2: UPDATE — discount_rate changed 0.15 → 0.18, min_quantity 10 → 5
    (2, 'Bulk Purchase Silver',    'electronics', 0.1800, 5,   50,   TRUE,  '{"tier": "silver"}'),
    -- Row 3: UPDATE — max_quantity changed 200 → 500, metadata updated
    (3, 'Bulk Purchase Gold',      'electronics', 0.2000, 51,  500,  TRUE,  '{"tier": "gold", "note": "expanded range"}'),
    -- Row 4: UPDATE — is_active flipped TRUE → FALSE
    (4, 'Seasonal Clearance',      'clothing',    0.3000, 1,   NULL, FALSE, '{"season": "winter"}'),
    -- Row 5: UNCHANGED
    (5, 'VIP Member Discount',     'all',         0.0500, 1,   NULL, TRUE,  '{"membership": "vip"}'),
    -- Row 6: DELETED (not present in sandbox)
    -- Row 7: UNCHANGED
    (7, 'Student Discount',        'books',       0.1200, 1,   NULL, TRUE,  '{"requires_id": true}'),
    -- Row 8: INSERT — new rule added by admin
    (8, 'New Customer Welcome',    'all',         0.1000, 1,   NULL, TRUE,  '{"first_order_only": true}'),
    -- Row 9: INSERT — another new rule
    (9, 'Bundle Deal Electronics', 'electronics', 0.2200, 3,   10,   TRUE,  '{"requires_bundle": true}');

-- ============================================================
-- TABLE 2: discount_tiers
-- Scenario: UPDATE only (admin adjusts tier thresholds)
-- ============================================================

CREATE TABLE staging.discount_tiers (
    id              SERIAL PRIMARY KEY,
    tier_name       VARCHAR(50) NOT NULL,
    min_spend       NUMERIC(10,2) NOT NULL,
    max_spend       NUMERIC(10,2),
    discount_pct    NUMERIC(5,2) NOT NULL,
    applies_to      VARCHAR(50) NOT NULL DEFAULT 'all',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE sandbox_admin1.discount_tiers (LIKE staging.discount_tiers INCLUDING ALL);

-- Staging
INSERT INTO staging.discount_tiers (id, tier_name, min_spend, max_spend, discount_pct, applies_to) VALUES
    (1, 'Bronze',   50.00,   199.99,  2.00, 'all'),
    (2, 'Silver',   200.00,  499.99,  5.00, 'all'),
    (3, 'Gold',     500.00,  999.99,  8.00, 'all'),
    (4, 'Platinum', 1000.00, NULL,    12.00, 'all');

-- Sandbox — admin raised all thresholds and discount_pct for Gold/Platinum
INSERT INTO sandbox_admin1.discount_tiers (id, tier_name, min_spend, max_spend, discount_pct, applies_to) VALUES
    (1, 'Bronze',   50.00,   199.99,  2.00, 'all'),    -- UNCHANGED
    (2, 'Silver',   200.00,  499.99,  5.00, 'all'),    -- UNCHANGED
    (3, 'Gold',     500.00,  999.99,  10.00, 'all'),   -- UPDATE: discount 8 → 10
    (4, 'Platinum', 1000.00, NULL,    15.00, 'all');    -- UPDATE: discount 12 → 15

-- ============================================================
-- TABLE 3: shipping_rules
-- Scenario: INSERT only (admin adds new shipping zones)
-- ============================================================

CREATE TABLE staging.shipping_rules (
    id              SERIAL PRIMARY KEY,
    zone_name       VARCHAR(100) NOT NULL,
    country_code    CHAR(2) NOT NULL,
    base_cost       NUMERIC(8,2) NOT NULL,
    free_above      NUMERIC(8,2),
    delivery_days   INTEGER NOT NULL,
    carrier         VARCHAR(50) NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE sandbox_admin1.shipping_rules (LIKE staging.shipping_rules INCLUDING ALL);

-- Staging (existing rules)
INSERT INTO staging.shipping_rules (id, zone_name, country_code, base_cost, free_above, delivery_days, carrier) VALUES
    (1, 'France Metro',     'FR', 4.99,  50.00,  2, 'Colissimo'),
    (2, 'Germany Standard', 'DE', 6.99,  75.00,  3, 'DHL'),
    (3, 'UK Express',       'GB', 9.99,  100.00, 2, 'Royal Mail');

-- Sandbox — admin adds 3 new zones, keeps existing ones intact
INSERT INTO sandbox_admin1.shipping_rules (id, zone_name, country_code, base_cost, free_above, delivery_days, carrier) VALUES
    (1, 'France Metro',     'FR', 4.99,  50.00,  2, 'Colissimo'),  -- UNCHANGED
    (2, 'Germany Standard', 'DE', 6.99,  75.00,  3, 'DHL'),        -- UNCHANGED
    (3, 'UK Express',       'GB', 9.99,  100.00, 2, 'Royal Mail'), -- UNCHANGED
    -- INSERTs:
    (4, 'Spain Standard',   'ES', 7.49,  60.00,  4, 'Correos'),
    (5, 'Italy Express',    'IT', 8.99,  70.00,  3, 'Poste Italiane'),
    (6, 'Belgium Quick',    'BE', 5.49,  45.00,  1, 'bpost');

-- ============================================================
-- TABLE 4: tax_rules
-- Scenario: DELETE only (admin removes deprecated tax rules)
-- Composite primary key: (region_code, product_category)
-- ============================================================

CREATE TABLE staging.tax_rules (
    region_code      VARCHAR(10) NOT NULL,
    product_category VARCHAR(50) NOT NULL,
    tax_rate         NUMERIC(5,4) NOT NULL,
    tax_name         VARCHAR(100) NOT NULL,
    effective_from   DATE NOT NULL,
    effective_to     DATE,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    PRIMARY KEY (region_code, product_category)
);

CREATE TABLE sandbox_admin1.tax_rules (LIKE staging.tax_rules INCLUDING ALL);

-- Staging (6 rules, admin will remove 3 deprecated ones)
INSERT INTO staging.tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR',    'electronics',  0.2000, 'TVA Standard',           '2024-01-01', NULL,         TRUE),
    ('FR',    'food',         0.0550, 'TVA Réduit',             '2024-01-01', NULL,         TRUE),
    ('FR',    'books',        0.0550, 'TVA Réduit Livres',      '2024-01-01', NULL,         TRUE),
    -- These 3 will be DELETED by admin:
    ('FR-CO', 'electronics',  0.1700, 'TVA Corse Ancien',       '2020-01-01', '2023-12-31', FALSE),
    ('FR-CO', 'food',         0.0210, 'TVA Corse Réduit Ancien','2020-01-01', '2023-12-31', FALSE),
    ('FR-GP', 'all',          0.0850, 'Octroi de Mer Ancien',   '2019-01-01', '2023-06-30', FALSE);

-- Sandbox — admin keeps only the 3 active rules
INSERT INTO sandbox_admin1.tax_rules (region_code, product_category, tax_rate, tax_name, effective_from, effective_to, is_active) VALUES
    ('FR',    'electronics',  0.2000, 'TVA Standard',           '2024-01-01', NULL,         TRUE),
    ('FR',    'food',         0.0550, 'TVA Réduit',             '2024-01-01', NULL,         TRUE),
    ('FR',    'books',        0.0550, 'TVA Réduit Livres',      '2024-01-01', NULL,         TRUE);

-- ============================================================
-- TABLE 5: no_change_rules
-- Scenario: aucun changement — diff doit être vide
-- ============================================================

CREATE TABLE staging.no_change_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(100) NOT NULL,
    value       NUMERIC(10,2) NOT NULL,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE sandbox_admin1.no_change_rules (LIKE staging.no_change_rules INCLUDING ALL);

INSERT INTO staging.no_change_rules (id, rule_name, value, is_active) VALUES
    (1, 'Identical Rule A', 10.00, TRUE),
    (2, 'Identical Rule B', 20.00, FALSE),
    (3, 'Identical Rule C', 30.00, TRUE);

INSERT INTO sandbox_admin1.no_change_rules (id, rule_name, value, is_active) VALUES
    (1, 'Identical Rule A', 10.00, TRUE),
    (2, 'Identical Rule B', 20.00, FALSE),
    (3, 'Identical Rule C', 30.00, TRUE);

-- ============================================================
-- TABLE 6: nullable_rules
-- Scenario: valeurs NULL — UPDATE d'une colonne non-NULL vers NULL et vice-versa
-- ============================================================

CREATE TABLE staging.nullable_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(100) NOT NULL,
    optional_a  VARCHAR(100),
    optional_b  NUMERIC(10,2),
    notes       TEXT
);

CREATE TABLE sandbox_admin1.nullable_rules (LIKE staging.nullable_rules INCLUDING ALL);

INSERT INTO staging.nullable_rules (id, rule_name, optional_a, optional_b, notes) VALUES
    -- Row 1: admin sets optional_a to NULL (was 'foo'), optional_b to 99 (was NULL)
    (1, 'Nullable Test A', 'foo',  NULL,  'original'),
    -- Row 2: admin sets notes to NULL (was 'some text')
    (2, 'Nullable Test B', NULL,   5.00,  'some text');

INSERT INTO sandbox_admin1.nullable_rules (id, rule_name, optional_a, optional_b, notes) VALUES
    -- Row 1: optional_a → NULL, optional_b → 99.00
    (1, 'Nullable Test A', NULL,   99.00, 'original'),
    -- Row 2: notes → NULL
    (2, 'Nullable Test B', NULL,   5.00,  NULL);

-- ============================================================
-- TABLE 7: jsonb_rules
-- Scenario: JSONB nested — modification d'un champ imbriqué
-- ============================================================

CREATE TABLE staging.jsonb_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(100) NOT NULL,
    config      JSONB NOT NULL
);

CREATE TABLE sandbox_admin1.jsonb_rules (LIKE staging.jsonb_rules INCLUDING ALL);

INSERT INTO staging.jsonb_rules (id, rule_name, config) VALUES
    (1, 'JSONB Nested Test', '{"threshold": 100, "nested": {"enabled": true, "level": 1}, "tags": ["a", "b"]}'),
    (2, 'JSONB Unchanged',   '{"simple": "value"}');

INSERT INTO sandbox_admin1.jsonb_rules (id, rule_name, config) VALUES
    -- Row 1: nested.level changed 1→2, tags appended 'c'
    (1, 'JSONB Nested Test', '{"threshold": 100, "nested": {"enabled": true, "level": 2}, "tags": ["a", "b", "c"]}'),
    -- Row 2: unchanged
    (2, 'JSONB Unchanged',   '{"simple": "value"}');

-- ============================================================
-- TABLE 8: empty_staging_rules
-- Scenario: table vide côté staging → tout est INSERT
-- ============================================================

CREATE TABLE staging.empty_staging_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(100) NOT NULL,
    value       NUMERIC(10,2) NOT NULL
);

CREATE TABLE sandbox_admin1.empty_staging_rules (LIKE staging.empty_staging_rules INCLUDING ALL);

-- Staging: vide (0 lignes)

-- Sandbox: 3 lignes — toutes seront des INSERTs
INSERT INTO sandbox_admin1.empty_staging_rules (id, rule_name, value) VALUES
    (1, 'New Rule Alpha',   10.00),
    (2, 'New Rule Beta',    20.00),
    (3, 'New Rule Gamma',   30.00);

-- ============================================================
-- TABLE 9: empty_sandbox_rules
-- Scenario: table vide côté sandbox → tout est DELETE
-- ============================================================

CREATE TABLE staging.empty_sandbox_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(100) NOT NULL,
    value       NUMERIC(10,2) NOT NULL
);

CREATE TABLE sandbox_admin1.empty_sandbox_rules (LIKE staging.empty_sandbox_rules INCLUDING ALL);

-- Staging: 3 lignes — toutes seront des DELETEs
INSERT INTO staging.empty_sandbox_rules (id, rule_name, value) VALUES
    (1, 'Deprecated Rule X', 11.00),
    (2, 'Deprecated Rule Y', 22.00),
    (3, 'Deprecated Rule Z', 33.00);

-- Sandbox: vide (0 lignes)

-- ============================================================
-- TABLE 10: quoted_rules
-- Scenario: texte avec quotes — vérifier l'échappement SQL et HTML
-- ============================================================

CREATE TABLE staging.quoted_rules (
    id          SERIAL PRIMARY KEY,
    rule_name   VARCHAR(200) NOT NULL,
    description TEXT
);

CREATE TABLE sandbox_admin1.quoted_rules (LIKE staging.quoted_rules INCLUDING ALL);

INSERT INTO staging.quoted_rules (id, rule_name, description) VALUES
    (1, 'Rule with ''single'' quotes', 'It''s a test with apostrophe'),
    (2, 'Rule unchanged',              'No special chars');

INSERT INTO sandbox_admin1.quoted_rules (id, rule_name, description) VALUES
    -- Row 1: UPDATE — description changes, contains special chars
    (1, 'Rule with ''single'' quotes', E'New desc with\nnewline and "double" quotes & <html>'),
    -- Row 2: INSERT — new rule with backslash
    (2, 'Rule unchanged',              'No special chars'),
    (3, 'Rule with backslash',         E'Path: C:\\Users\\admin');

-- ============================================================
-- Grant access
-- ============================================================
GRANT USAGE ON SCHEMA staging TO diffly;
GRANT USAGE ON SCHEMA sandbox_admin1 TO diffly;
GRANT SELECT ON ALL TABLES IN SCHEMA staging TO diffly;
GRANT SELECT ON ALL TABLES IN SCHEMA sandbox_admin1 TO diffly;

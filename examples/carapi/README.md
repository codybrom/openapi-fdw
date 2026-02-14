# CarAPI Example

Query the [CarAPI](https://carapi.app/) vehicle database using SQL. This example demonstrates the OpenAPI FDW against a free, no-auth API with **page-based pagination**, auto-detected `data` wrapper key, and **query parameter pushdown** for filtering by year, make, and model.

## Quick Start

**Prerequisites:** Docker, Rust 1.88+, `cargo-component` v0.21.1, `wasm32-unknown-unknown` target.

```bash
# Start everything (builds WASM, starts Postgres, copies binary)
./examples/carapi/setup.sh

# Connect
psql postgresql://postgres:postgres@localhost:54326/postgres

# When done
./examples/carapi/teardown.sh
```

> All queries below hit the live CarAPI. No API key or authentication is needed. Free demo data covers model years **2015-2020** only.

---

## 1. Makes

Fetches all car manufacturers. Demonstrates **page-based pagination** with auto-detected `data` wrapper key. The CarAPI wraps responses in `{"collection": {...}, "data": [...]}` and the FDW auto-detects the `data` key.

```sql
SELECT id, name
FROM makes
LIMIT 5;
```

| id | name |
| --- | --- |
| 1 | Acura |
| 24 | Alfa Romeo |
| 44 | Aston Martin |
| 2 | Audi |
| 25 | Bentley |

## 2. Models

Car models filtered by make and year. Demonstrates **query parameter pushdown** — the WHERE clause values are sent as query parameters to the API, so only matching data is returned.

```sql
SELECT id, name, make
FROM models
WHERE make = 'Toyota' AND year = '2020'
LIMIT 5;
```

| id | name | make |
| --- | --- | --- |
| 4841 | 4Runner | Toyota |
| 7245 | 86 | Toyota |
| 5689 | Avalon | Toyota |
| 7308 | C-HR | Toyota |
| 4779 | Camry | Toyota |

## 3. Trims

Trim levels with MSRP pricing. Combines query pushdown (year, make, model) with integer type coercion for pricing fields.

```sql
SELECT trim, msrp, description
FROM trims
WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry'
LIMIT 3;
```

| trim | msrp | description |
| --- | --- | --- |
| LE | 28430 | LE 4dr Sedan (2.5L 4cyl gas/electric hybrid CVT) |
| SE | 30130 | SE 4dr Sedan (2.5L 4cyl gas/electric hybrid CVT) |
| XLE | 32730 | XLE 4dr Sedan (2.5L 4cyl gas/electric hybrid CVT) |

## 4. Bodies

Vehicle body dimensions. Demonstrates mixed types — integer for counts/weights, text for decimal measurements.

```sql
SELECT type, doors, length, curb_weight
FROM bodies
WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry'
LIMIT 3;
```

| type | doors | length | curb_weight |
| --- | --- | --- | --- |
| Sedan | 4 | 192.1 | 3472 |
| Sedan | 4 | 192.7 | 3549 |
| Sedan | 4 | 192.1 | 3572 |

## 5. Engines

Engine specifications and performance data.

```sql
SELECT engine_type, horsepower_hp, cylinders, transmission
FROM engines
WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry'
LIMIT 3;
```

| engine_type | horsepower_hp | cylinders | transmission |
| --- | --- | --- | --- |
| hybrid | 208 | I4 | continuously variable-speed automatic |
| hybrid | 208 | I4 | continuously variable-speed automatic |
| hybrid | 208 | I4 | continuously variable-speed automatic |

## 6. Mileages

Fuel economy and range data (EPA ratings).

```sql
SELECT combined_mpg, epa_city_mpg, epa_highway_mpg, range_city
FROM mileages
WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry'
LIMIT 3;
```

| combined_mpg | epa_city_mpg | epa_highway_mpg | range_city |
| --- | --- | --- | --- |
| 52 | 51 | 53 | 673 |
| 46 | 44 | 47 | 581 |
| 46 | 44 | 47 | 581 |

## 7. Exterior Colors

Paint colors with RGB values.

```sql
SELECT color, rgb
FROM exterior_colors
WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry'
LIMIT 5;
```

| color | rgb |
| --- | --- |
| Blue Streak Metallic | 0,62,155 |
| Brownstone | 95,85,71 |
| Celestial Silver Metallic | 151,156,160 |
| Galactic Aqua Mica | 37,54,65 |
| Midnight Black Metallic | 23,23,23 |

## 8. OBD Codes

OBD-II diagnostic trouble codes. A small dataset available on the free tier.

```sql
SELECT code, description
FROM obd_codes
LIMIT 5;
```

| code | description |
| --- | --- |
| P0100 | Mass or Volume Air Flow Sensor A Circuit |
| U1000 | Manufacturer Controlled DTC |

## 9. Debug Mode

The `makes_debug` table uses the `carapi_debug` server which has `debug 'true'`. This emits HTTP request details and scan statistics as PostgreSQL INFO messages.

```sql
SELECT id FROM makes_debug LIMIT 1;
```

Look for INFO output like:

```log
INFO:  [openapi_fdw] HTTP GET https://carapi.app/api/makes/v2 -> 200 (1404 bytes)
INFO:  [openapi_fdw] Scan complete: 1 rows, 1 columns
```

## 10. The `attrs` Column

Every table includes an `attrs jsonb` column that captures **all fields not mapped to named columns**. This is useful for exploring what data the API returns without defining every column upfront.

```sql
SELECT name, attrs
FROM makes
LIMIT 1;
```

## Features Demonstrated

| Feature | Table(s) |
| --- | --- |
| Page-based pagination (auto-followed) | `makes`, `models`, `trims`, `bodies`, `engines`, `mileages`, `exterior_colors` |
| Auto-detected `data` wrapper key | All tables |
| Query parameter pushdown | `models`, `trims`, `bodies`, `engines`, `mileages`, `exterior_colors` |
| Integer type coercion | `trims` (msrp), `bodies` (curb_weight), `engines` (horsepower), `mileages` (mpg) |
| `timestamptz` coercion | `trims` (created, modified) |
| LIMIT pushdown | Any table with `LIMIT` |
| Debug mode (`debug`) | `makes_debug` |
| `attrs` catch-all column | All tables |
| `rowid_column` | All tables |
| No authentication required | All servers |

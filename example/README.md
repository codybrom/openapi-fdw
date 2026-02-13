# NWS Weather API Example

Query the [National Weather Service API](https://www.weather.gov/documentation/services-web-api) using SQL. This example exercises all major features of the OpenAPI FDW against a real, free, no-auth API.

## Quick Start

**Prerequisites:** Docker, Rust 1.88+, `cargo-component` v0.21.1, `wasm32-unknown-unknown` target.

```bash
# Start everything (builds WASM, starts Postgres, copies binary)
./example/setup.sh

# Connect
psql postgresql://postgres:postgres@localhost:54322/postgres

# When done
./example/teardown.sh
```

> All queries below hit the live NWS API. Results will reflect real-time weather data.

---

## 1. Weather Stations

Fetches the full list of US weather stations. Demonstrates **GeoJSON extraction** (`response_path` + `object_path`), **cursor-based pagination** (`cursor_path`), and **camelCase-to-snake_case** column matching (`stationIdentifier` → `station_identifier`).

```sql
SELECT station_identifier, name, time_zone
FROM stations
LIMIT 10;
```

The `stations` table paginates automatically — the FDW follows `/pagination/next` cursors. Try fetching more:

```sql
SELECT count(*) FROM stations;
```

The `elevation` column is `jsonb` because the API returns a structured object with value and unit:

```sql
SELECT station_identifier, name, elevation
FROM stations
LIMIT 3;
```

## 2. Active Alerts

Different GeoJSON shape with **timestamptz coercion** for `onset` and `expires` columns.

```sql
SELECT event, severity, headline, onset, expires
FROM active_alerts
LIMIT 5;
```

Filter in SQL after fetching:

```sql
SELECT event, severity, headline
FROM active_alerts
WHERE severity IN ('Severe', 'Extreme')
LIMIT 10;
```

## 3. Query Param Pushdown (severity filter)

When a WHERE clause references a column that isn't a path parameter, the FDW sends it as a **query parameter** to the API. The NWS alerts endpoint supports a `severity` filter — and because it echoes `severity` back in every response object, the column is populated naturally:

```sql
-- Pushes down to: GET /alerts/active?severity=Severe
SELECT event, severity, headline
FROM active_alerts
WHERE severity = 'Severe'
LIMIT 5;
```

Try other severity values: `Extreme`, `Moderate`, `Minor`, `Unknown`.

## 4. Station Observations

**Path parameter substitution**: the `{station_id}` placeholder in the endpoint is replaced with the value from your WHERE clause.

```sql
-- Pushes down to: GET /stations/KDEN/observations
SELECT timestamp, text_description, temperature
FROM station_observations
WHERE station_id = 'KDEN'
LIMIT 5;
```

`KDEN` is Denver International Airport. Try other station IDs: `KJFK` (New York), `KLAX` (Los Angeles), `KORD` (Chicago).

Temperature and wind values are `jsonb` because the NWS returns them as objects with unit and value:

```sql
SELECT timestamp,
       temperature->>'value' AS temp_c,
       wind_speed->>'value' AS wind_mps,
       text_description
FROM station_observations
WHERE station_id = 'KDEN'
LIMIT 5;
```

## 5. Current Conditions

**Single object response** — the `/observations/latest` endpoint returns one GeoJSON Feature (not a FeatureCollection). The FDW auto-detects this and returns a single row.

```sql
SELECT text_description,
       temperature->>'value' AS temp_c,
       wind_speed->>'value' AS wind_mps,
       wind_direction->>'value' AS wind_deg,
       barometric_pressure->>'value' AS pressure_pa,
       relative_humidity->>'value' AS humidity_pct
FROM latest_observation
WHERE station_id = 'KDEN';
```

## 6. Point Metadata & Forecast

This two-step flow demonstrates **composite path parameters** and **nested response extraction**.

**Step 1:** Look up grid coordinates for a location (Denver: 39.7456,-104.9887):

```sql
SELECT grid_id, grid_x, grid_y, forecast
FROM point_metadata
WHERE point = '39.7456,-104.9887';
```

**Step 2:** Use those grid coordinates to fetch the forecast. This exercises **multiple path parameters** (`wfo`, `x`, `y`) and **nested `response_path`** (`/properties/periods` digs two levels into the response):

```sql
-- Replace wfo/x/y with values from Step 1
SELECT name, temperature, temperature_unit,
       is_daytime, wind_speed, short_forecast
FROM forecast_periods
WHERE wfo = 'BOU' AND x = '63' AND y = '62';
```

> Grid coordinates vary by location. Always use Step 1 to find the right values for your area.

## 7. IMPORT FOREIGN SCHEMA

Auto-generate table definitions from the NWS OpenAPI spec. The `nws_import` server has a `spec_url` configured.

```sql
CREATE SCHEMA IF NOT EXISTS nws_auto;

IMPORT FOREIGN SCHEMA "unused"
FROM SERVER nws_import
INTO nws_auto;
```

See what was generated:

```sql
SELECT foreign_table_name FROM information_schema.foreign_tables
WHERE foreign_table_schema = 'nws_auto';
```

Pick a generated table and query it:

```sql
SELECT * FROM nws_auto.alerts LIMIT 3;
```

## 8. Debug Mode

The `stations_debug` table uses the `nws_debug` server which has `debug 'true'`. This emits HTTP request details (method, URL, status, response size) and scan statistics (row/column counts) as PostgreSQL INFO messages.

```sql
SELECT station_identifier, name
FROM stations_debug
LIMIT 5;
```

Look for INFO output like:

```
INFO:  [openapi_fdw] HTTP GET https://api.weather.gov/stations?limit=50 -> 200 (51639 bytes)
INFO:  [openapi_fdw] Scan complete: 5 rows, 2 columns
```

## 9. The `attrs` Column

Every table includes an `attrs jsonb` column that captures **all fields not mapped to named columns**. This is useful for exploring what data the API returns without defining every column.

```sql
SELECT station_identifier, jsonb_object_keys(attrs) AS extra_field
FROM stations
LIMIT 20;
```

Dig into a specific field that isn't in the table definition:

```sql
SELECT station_identifier, attrs->>'county' AS county
FROM stations
LIMIT 10;
```

## Features Demonstrated

| Feature | Table(s) |
|---------|----------|
| GeoJSON extraction (`response_path` + `object_path`) | `stations`, `active_alerts`, `station_observations` |
| Cursor-based pagination (`cursor_path`) | `stations` |
| Path parameter substitution | `station_observations`, `latest_observation`, `point_metadata`, `forecast_periods` |
| Query parameter pushdown | `active_alerts` (with `WHERE severity = ...`) |
| camelCase → snake_case matching | All tables |
| Custom headers (`user_agent`, `accept`) | All servers |
| LIMIT pushdown | Any table with `LIMIT` |
| Debug mode (`debug`) | `stations_debug` |
| IMPORT FOREIGN SCHEMA | `nws_import` server |
| Single object response | `latest_observation`, `point_metadata` |
| Type coercion (timestamptz, jsonb, boolean, integer) | `active_alerts`, `forecast_periods` |
| `attrs` catch-all column | All tables |
| Multiple path parameters | `forecast_periods` |
| Nested response extraction (JSON pointer) | `forecast_periods` |
| `rowid_column` | `stations`, `active_alerts` |

Library and HTTP service, implementing analytics collection and various utilities.



## Library

The library provides utilities for [axum](https://github.com/tokio-rs/axum).

It requires the following environment variables:
- `GATEWAY_URL`: the URL of the endpoint to push analytics to
- `GATEWAY_PROPERTY`: the property's UUID
- `GATEWAY_SECRET`: the property's secret



## HTTP service

The HTTP service collects analytics and saves them to PostgreSQL.

It requires the following environment variables:
- `PORT`: the port the HTTP API is exposed to
- `DB`: the PostgreSQL URL
- `UAPARSER_URL`: the URL to download the `uaparser.yml` file, allowing to parse the `User-Agent` header
- `GEOIP_URL`: the URL to download the GeoIP database
- `GEOIP_USER`: the GeoIP account ID
- `GEOIP_PASSWORD`: the GeoIP license key

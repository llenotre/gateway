Library and HTTP service, implementing analytics collection.



## Library

The library provides a middleware for [axum](https://github.com/tokio-rs/axum).

It requires the following environment variables:
- `ANALYTICS_URL`: the URL of the endpoint to push analytics to
- `ANALYTICS_TOKEN`: the access token of the service



## HTTP service

The HTTP service collects analytics and saves them to PostgreSQL.

It requires the following environment variables:
- `PORT`: the port the HTTP API is exposed to
- `DB`: the PostgreSQL URL
- `UAPARSER_URL`: the URL to download the `uaparser.yml` file, allowing to parse the `User-Agent` header
- `GEOIP_URL`: the URL to download the GeoIP database
- `GEOIP_USER`: the GeoIP account ID
- `GEOIP_PASSWORD`: the GeoIP license key

use chrono::{DateTime, Utc};
use failure::format_err;
use models::node::{TokenLike, TokenPrice};
use models::primitives::UnsignedRatioSerializeAsDecimal;
use num::rational::Ratio;
use num::BigUint;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) async fn fetch_coimarketcap_data(
    client: &reqwest::Client,
    base_url: &Url,
    token_symbol: &str,
) -> Result<TokenPrice, failure::Error> {
    let request_url = base_url
        .join(&format!(
            "cryptocurrency/quotes/latest?symbol={}",
            token_symbol
        ))
        .expect("failed to join url path");
    let mut api_response = client
        .get(request_url)
        .send()
        .await?
        .json::<CoinmarketCapResponse>()
        .await?;
    let mut token_info = api_response
        .data
        .remove(&TokenLike::Symbol(token_symbol.to_string()))
        .ok_or_else(|| format_err!("Could not found token in response"))?;
    let usd_quote = token_info
        .quote
        .remove(&TokenLike::Symbol("USD".to_string()))
        .ok_or_else(|| format_err!("Could not found usd quote in response"))?;
    Ok(TokenPrice {
        usd_price: usd_quote.price,
        last_updated: usd_quote.last_updated,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub(super) struct CoinmarketcapQuote {
    #[serde(with = "UnsignedRatioSerializeAsDecimal")]
    pub price: Ratio<BigUint>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub(super) struct CoinmarketcapTokenInfo {
    pub quote: HashMap<TokenLike, CoinmarketcapQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CoinmarketCapResponse {
    pub data: HashMap<TokenLike, CoinmarketcapTokenInfo>,
}

#[cfg(test)]
mod test {
    use super::*;
    use models::config_options::parse_env;
    use std::str::FromStr;

    #[test]
    // Should be run in the dev environment
    fn test_fetch_coinmarketcap_data() {
        let mut runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let ticker_url = parse_env("TICKER_URL");
        let client = reqwest::Client::new();
        runtime
            .block_on(fetch_coimarketcap_data(&client, &ticker_url, "ETH"))
            .expect("Failed to get data from ticker");
    }

    #[test]
    fn parse_coinmarket_cap_responce() {
        let example = r#"{
    "status": {
        "timestamp": "2020-04-17T04:51:12.012Z",
        "error_code": 0,
        "error_message": null,
        "elapsed": 9,
        "credit_count": 1,
        "notice": null
    },
    "data": {
        "ETH": {
            "id": 1027,
            "name": "Ethereum",
            "symbol": "ETH",
            "slug": "ethereum",
            "num_market_pairs": 5153,
            "date_added": "2015-08-07T00:00:00.000Z",
            "tags": [
                "mineable"
            ],
            "max_supply": null,
            "circulating_supply": 110550929.1865,
            "total_supply": 110550929.1865,
            "platform": null,
            "cmc_rank": 2,
            "last_updated": "2020-04-17T04:50:41.000Z",
            "quote": {
                "USD": {
                    "price": 170.692214992,
                    "volume_24h": 22515583743.3856,
                    "percent_change_1h": -0.380817,
                    "percent_change_24h": 11.5718,
                    "percent_change_7d": 3.6317,
                    "market_cap": 18870182972.267426,
                    "last_updated": "2020-04-17T04:50:41.000Z"
                }
            }
        }
    }
}"#;
        let resp =
            serde_json::from_str::<CoinmarketCapResponse>(example).expect("serialization failed");
        let token_data = resp
            .data
            .get(&TokenLike::Symbol("ETH".to_string()))
            .expect("ETH data not found");
        let quote = token_data
            .quote
            .get(&TokenLike::Symbol("USD".to_string()))
            .expect("USD not found");
        assert_eq!(
            quote.price,
            UnsignedRatioSerializeAsDecimal::deserialize_from_str_with_dot("170.692214992")
                .unwrap()
        );
        assert_eq!(
            quote.last_updated,
            DateTime::<Utc>::from_str("2020-04-17T04:50:41.000Z").unwrap()
        );
    }
}

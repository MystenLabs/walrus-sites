use anyhow::Result;
use serde::Deserialize;
use sui_sdk::{SuiClient, SuiClientBuilder};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Localnet,
    Devnet,
    Testnet,
    Mainnet,
}

impl Network {
    pub async fn get_sui_client(&self) -> Result<SuiClient> {
        match self {
            Network::Localnet => Ok(SuiClientBuilder::default().build_localnet().await?),
            Network::Devnet => Ok(SuiClientBuilder::default().build_devnet().await?),
            Network::Testnet => Ok(SuiClientBuilder::default().build_testnet().await?),
            Network::Mainnet => panic!("No mainnet?"),
        }
    }
}

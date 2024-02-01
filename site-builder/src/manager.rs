use std::mem;

use anyhow::Result;
use sui_keys::keystore::{FileBasedKeystore, Keystore};
use sui_sdk::{rpc_types::SuiTransactionBlockResponse, SuiClient};
use sui_types::transaction::{
    Argument,
    CallArg::{self},
};

use crate::{
    calls::CallBuilder,
    page::{Page, Site},
    util::{get_object_ref_from_id, sign_and_send_ptb},
    Config,
};

// const MAX_TX_LEN: usize = 131_072;
const MAX_ARG_LEN: usize = 16_300;
// const MAX_ARG_LEN: usize = 1000;

pub struct SuiManager {
    pub calls: CallBuilder,
    pub client: SuiClient,
    pub config: Config,
    pub keystore: Keystore,
}

impl SuiManager {
    pub async fn new(config: Config) -> Result<Self> {
        let builder = CallBuilder::new(config.package, config.module.clone());
        let client = config.network.get_sui_client().await?;
        let keystore = Keystore::File(FileBasedKeystore::new(&config.keystore)?);
        Ok(SuiManager {
            calls: builder,
            client,
            config,
            keystore,
        })
    }

    /// Terminates the construction of the transaction, signs it, and sends it
    /// The call builder is reset after this.
    pub async fn sign_and_send(&mut self) -> Result<SuiTransactionBlockResponse> {
        let builder = mem::replace(
            &mut self.calls,
            CallBuilder::new(self.config.package, self.config.module.clone()),
        );
        sign_and_send_ptb(
            &self.client,
            &self.keystore,
            self.config.address,
            builder.finish(),
            get_object_ref_from_id(&self.client, self.config.gas_coin).await?,
            self.config.gas_budget,
        )
        .await
    }

    // Convenience functions to the undelying builder
    // TODO: Logic for bigger pages with [add_piece].

    /// PTB to create a BlockPage in the from a [Page]
    /// In case the contents of the page are too large to fit in a single argument input,
    /// it is split up and added to the transaction in multiple calls to [add_piece]
    pub fn create_page(&mut self, page: &Page) -> Result<Argument> {
        println!("Preparing page: {}", page.name);
        self.create_page_in_chunks(page)
    }

    pub fn create_site(&mut self, site: &Site) -> Result<Argument> {
        let clock = self.calls.pt_builder.input(CallArg::CLOCK_IMM)?;
        let name = self.calls.pt_builder.pure(site.name.clone())?;
        self.calls.new_site(name, clock)
    }

    pub fn create_and_add_page(&mut self, site: Argument, page: &Page) -> Result<()> {
        let page_arg = self.create_page(page)?;
        self.calls.add_page(site, page_arg)?;
        Ok(())
    }

    /// Create the site and all the pages in it, add these to the site, and execute the transaction
    pub async fn publish_site(
        &mut self,
        site: &Site,
        pages: &[Page],
    ) -> Result<SuiTransactionBlockResponse> {
        let site_arg = self.create_site(site)?;
        for page in pages {
            self.create_and_add_page(site_arg, page)?;
        }
        self.calls.transfer_arg(self.config.address, site_arg);
        self.sign_and_send().await
    }

    fn create_page_in_chunks(&mut self, page: &Page) -> Result<Argument> {
        let clock = self.calls.pt_builder.input(CallArg::CLOCK_IMM)?;
        let name = self.calls.pt_builder.pure(page.name.clone())?;
        let content_type = self.calls.pt_builder.pure(page.content_type.to_string())?;
        let content_encoding = self
            .calls
            .pt_builder
            .pure(page.content_encoding.to_string())?;

        // Create separate chunks of at most MAX_ARG_LEN
        let mut chunks = page.content.chunks(MAX_ARG_LEN);
        let first_chunk_arg = self.calls.pt_builder.pure(chunks.next().unwrap())?;
        let page_arg =
            self.calls
                .new_page(name, content_type, content_encoding, first_chunk_arg, clock)?;
        for chunk in chunks {
            let chunk_arg = self.calls.pt_builder.pure(chunk)?;
            self.calls.add_piece(page_arg, chunk_arg, clock)?;
        }
        Ok(page_arg)
    }
}

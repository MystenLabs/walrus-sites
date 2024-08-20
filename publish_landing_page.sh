#!/bin/bash
# Publishes the landing page to walrus sites.
echo "Building site builder..." && \
cargo build --release && \
echo "Creating temporary landing page directory..." && \
mkdir temp-landing-page && \
cp -r portal/common/static/* temp-landing-page && \
rm temp-landing-page/index.html temp-landing-page/404-page.template.html temp-landing-page/sw.js && \
echo "Publishing landing page to walrus sites..."
./target/release/site-builder --config site-builder/assets/builder-example.yaml publish temp-landing-page/ > publish-result.log
echo "Cleaning up..."
rm -rf temp-landing-page
echo "Done."

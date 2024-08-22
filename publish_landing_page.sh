# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash
# Publishes the landing page to walrus sites.
echo "Building site builder..." && \
cargo build --release && \
echo "Creating temporary landing page directory..." && \
mkdir temp-landing-page && \
cp -r portal/common/static/* temp-landing-page && \
rm temp-landing-page/index.html && \
mv temp-landing-page/index-sw-enabled.html temp-landing-page/index.html && \
rm temp-landing-page/{404-page.template.html,sw.js,walrus-sites-portal-register-sw.js} && \
echo "Publishing landing page to walrus sites..."
./target/release/site-builder --config \
site-builder/assets/builder-example.yaml publish temp-landing-page/ \
> publish-result.log
echo "Cleaning up..."
rm -rf temp-landing-page
echo "Done."

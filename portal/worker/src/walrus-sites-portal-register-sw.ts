// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { getSubdomainAndPath } from "@lib/domain_parsing";
import { FALLBACK_PORTAL } from "@lib/constants";

function main() {
    if ("serviceWorker" in navigator) {
        navigator.serviceWorker
            .register("/walrus-sites-sw.js")
            .then((reg) => {
                console.log("SW registered");
                if (reg.installing) {
                    const sw = reg.installing || reg.waiting;
                    sw.onstatechange = function () {
                        if (sw.state === "installed") {
                            console.log("SW installed");
                            // SW installed. Refresh page so SW can respond with SW-enabled page.
                            window.location.reload();
                        }
                    };
                } else if (reg.active) {
                    console.log("SW active, error?");
                    // Previously-installed SW should have redirected this request to different page
                    handleError();
                }
            })
            .catch(handleError);
    } else {
        const currentUrl = new URL(window.location.href);
        console.warn(
            "This browser does not yet support Walrus Sites 💔, redirecting to blob.store",
        );
        const domainDetails = getSubdomainAndPath(currentUrl);
        window.location.href = new URL(
            `${currentUrl.pathname}${currentUrl.search}${currentUrl.hash}`,
            `https://${
                domainDetails.subdomain ? domainDetails.subdomain + "." : ""
            }${FALLBACK_PORTAL}`,
        ).toString();
    }
}

function handleError() {
    displayErrorMessage(swNotLoadingNode());
}

function swNotLoadingNode() {
    return titleSubtitleNode(
        "Oh! Something's not right 🚧",
        "Please try refreshing the page or unregistering the service worker.",
    );
}

function titleSubtitleNode(title: string, subtitle: string) {
    let h3 = document.createElement("h3");
    h3.textContent = title;
    h3.className = "InterTightMedium";
    let p = document.createElement("p");
    p.textContent = subtitle;
    p.className = "InterTightMedium";
    p.style.color = "#696969";
    p.style.fontSize = "18px";
    let div = document.createElement("div");
    div.appendChild(h3);
    div.appendChild(p);
    return div;
}

function displayErrorMessage(messageNode: any) {
    let messageDiv = document.getElementById("loading-message");
    messageDiv.replaceChildren(messageNode);
}

main();

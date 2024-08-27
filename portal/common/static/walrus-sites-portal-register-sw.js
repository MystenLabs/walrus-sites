// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

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
                    handleError(new Error("Service Worker is installed but not redirecting"));
                }
            })
            .catch(handleError);
    } else {
        const currentUrl = new URL(window.location.href);
        console.warn("This browser does not yet support Walrus Sites 💔, redirecting to blob.store");
        window.location.href = currentUrl.toString().replace('walrus.site', 'blob.store');
    }
}

function handleError(error) {
    displayErrorMessage(swNotLoadingNode());
}

function swNotLoadingNode() {
    return titleSubtitleNode(
        "Oh! Something's not right 🚧",
        "Please try refreshing the page or unregistering the service worker."
    );
}

function titleSubtitleNode(title, subtitle) {
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

function displayErrorMessage(messageNode) {
    let messageDiv = document.getElementById("loading-message");
    messageDiv.replaceChildren(messageNode);
}

main();

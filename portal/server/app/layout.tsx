// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Analytics } from "@vercel/analytics/next";

export default function RootLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return (
        <html lang="en">
            <head>
                <title>Next.js</title>
            </head>
            <body>
                {children}
                <Analytics />
            </body>
        </html>
    );
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

'use client';
import {useEffect} from 'react'
import { registerServiceWorker } from '../src/walrus-sites-portal-register-sw';

export default function Page() {
    useEffect(() => {
        if ('serviceWorker' in navigator && 'PushManager' in window) {
            registerServiceWorker()
        }
    }, [])
}

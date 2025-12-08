// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// TODO: here should be included the logic of reading files/resources.
// TODO: files should be also be possible to be loaded from the browser.
// TODO: configuration file loading logic (e.g. ws-resources) should be included.
import { parseSitesConfig, type SitesConfig } from '../utils/sites_config_parser'
import { readFileSync } from 'fs'
import { parse as parseYaml } from 'yaml'

export function loadSitesConfig(path: string): SitesConfig {
    const fileContent = readFileSync(path, 'utf-8')
    const yamlData = parseYaml(fileContent)
    return parseSitesConfig(yamlData)
}

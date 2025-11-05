// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**************************************************************
 * THIS FILE IS GENERATED AND SHOULD NOT BE MANUALLY MODIFIED *
 **************************************************************/


/** The module exposes the functionality to create and update Walrus sites. */

import { MoveStruct, normalizeMoveArguments, type RawTransactionArgument } from '../utils/index.js';
import { bcs } from '@mysten/sui/bcs';
import { type Transaction } from '@mysten/sui/transactions';
import * as object from './deps/sui/object.js';
import * as vec_map from './deps/sui/vec_map.js';
const $moduleName = '@walrus/sites::site';
export const Site = new MoveStruct({ name: `${$moduleName}::Site`, fields: {
        id: object.UID,
        name: bcs.string(),
        link: bcs.option(bcs.string()),
        image_url: bcs.option(bcs.string()),
        description: bcs.option(bcs.string()),
        project_url: bcs.option(bcs.string()),
        creator: bcs.option(bcs.string())
    } });
export const Range = new MoveStruct({ name: `${$moduleName}::Range`, fields: {
        start: bcs.option(bcs.u64()),
        end: bcs.option(bcs.u64())
    } });
export const Resource = new MoveStruct({ name: `${$moduleName}::Resource`, fields: {
        path: bcs.string(),
        headers: vec_map.VecMap(bcs.string(), bcs.string()),
        blob_id: bcs.u256(),
        blob_hash: bcs.u256(),
        range: bcs.option(Range)
    } });
export const ResourcePath = new MoveStruct({ name: `${$moduleName}::ResourcePath`, fields: {
        path: bcs.string()
    } });
export const Routes = new MoveStruct({ name: `${$moduleName}::Routes`, fields: {
        route_list: vec_map.VecMap(bcs.string(), bcs.string())
    } });
export const SITE = new MoveStruct({ name: `${$moduleName}::SITE`, fields: {
        dummy_field: bcs.bool()
    } });
export interface NewSiteArguments {
    name: RawTransactionArgument<string>;
    metadata: RawTransactionArgument<string>;
}
export interface NewSiteOptions {
    package?: string;
    arguments: NewSiteArguments | [
        name: RawTransactionArgument<string>,
        metadata: RawTransactionArgument<string>
    ];
}
/** Creates a new site. */
export function newSite(options: NewSiteOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String',
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["name", "metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'new_site',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface NewRangeOptionArguments {
    rangeStart: RawTransactionArgument<number | bigint | null>;
    rangeEnd: RawTransactionArgument<number | bigint | null>;
}
export interface NewRangeOptionOptions {
    package?: string;
    arguments: NewRangeOptionArguments | [
        rangeStart: RawTransactionArgument<number | bigint | null>,
        rangeEnd: RawTransactionArgument<number | bigint | null>
    ];
}
/** Optionally creates a new Range object. */
export function newRangeOption(options: NewRangeOptionOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<u64>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<u64>'
    ] satisfies string[];
    const parameterNames = ["rangeStart", "rangeEnd"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'new_range_option',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface NewRangeArguments {
    rangeStart: RawTransactionArgument<number | bigint | null>;
    rangeEnd: RawTransactionArgument<number | bigint | null>;
}
export interface NewRangeOptions {
    package?: string;
    arguments: NewRangeArguments | [
        rangeStart: RawTransactionArgument<number | bigint | null>,
        rangeEnd: RawTransactionArgument<number | bigint | null>
    ];
}
/**
 * Creates a new Range object.
 *
 * aborts if both range_start and range_end are none.
 */
export function newRange(options: NewRangeOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<u64>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<u64>'
    ] satisfies string[];
    const parameterNames = ["rangeStart", "rangeEnd"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'new_range',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface NewResourceArguments {
    path: RawTransactionArgument<string>;
    blobId: RawTransactionArgument<number | bigint>;
    blobHash: RawTransactionArgument<number | bigint>;
    range: RawTransactionArgument<string | null>;
}
export interface NewResourceOptions {
    package?: string;
    arguments: NewResourceArguments | [
        path: RawTransactionArgument<string>,
        blobId: RawTransactionArgument<number | bigint>,
        blobHash: RawTransactionArgument<number | bigint>,
        range: RawTransactionArgument<string | null>
    ];
}
/** Creates a new resource. */
export function newResource(options: NewResourceOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String',
        'u256',
        'u256',
        `0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<${packageAddress}::site::Range>`
    ] satisfies string[];
    const parameterNames = ["path", "blobId", "blobHash", "range"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'new_resource',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface AddHeaderArguments {
    resource: RawTransactionArgument<string>;
    name: RawTransactionArgument<string>;
    value: RawTransactionArgument<string>;
}
export interface AddHeaderOptions {
    package?: string;
    arguments: AddHeaderArguments | [
        resource: RawTransactionArgument<string>,
        name: RawTransactionArgument<string>,
        value: RawTransactionArgument<string>
    ];
}
/** Adds a header to the Resource's headers vector. */
export function addHeader(options: AddHeaderOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Resource`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String',
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["resource", "name", "value"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'add_header',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface UpdateNameArguments {
    site: RawTransactionArgument<string>;
    newName: RawTransactionArgument<string>;
}
export interface UpdateNameOptions {
    package?: string;
    arguments: UpdateNameArguments | [
        site: RawTransactionArgument<string>,
        newName: RawTransactionArgument<string>
    ];
}
/** Updates the name of a site. */
export function updateName(options: UpdateNameOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "newName"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'update_name',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface UpdateMetadataArguments {
    site: RawTransactionArgument<string>;
    metadata: RawTransactionArgument<string>;
}
export interface UpdateMetadataOptions {
    package?: string;
    arguments: UpdateMetadataArguments | [
        site: RawTransactionArgument<string>,
        metadata: RawTransactionArgument<string>
    ];
}
/** Update the site metadata. */
export function updateMetadata(options: UpdateMetadataOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["site", "metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'update_metadata',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface AddResourceArguments {
    site: RawTransactionArgument<string>;
    resource: RawTransactionArgument<string>;
}
export interface AddResourceOptions {
    package?: string;
    arguments: AddResourceArguments | [
        site: RawTransactionArgument<string>,
        resource: RawTransactionArgument<string>
    ];
}
/** Adds a resource to an existing site. */
export function addResource(options: AddResourceOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        `${packageAddress}::site::Resource`
    ] satisfies string[];
    const parameterNames = ["site", "resource"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'add_resource',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface RemoveResourceArguments {
    site: RawTransactionArgument<string>;
    path: RawTransactionArgument<string>;
}
export interface RemoveResourceOptions {
    package?: string;
    arguments: RemoveResourceArguments | [
        site: RawTransactionArgument<string>,
        path: RawTransactionArgument<string>
    ];
}
/**
 * Removes a resource from a site.
 *
 * Aborts if the resource does not exist.
 */
export function removeResource(options: RemoveResourceOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "path"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'remove_resource',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface RemoveResourceIfExistsArguments {
    site: RawTransactionArgument<string>;
    path: RawTransactionArgument<string>;
}
export interface RemoveResourceIfExistsOptions {
    package?: string;
    arguments: RemoveResourceIfExistsArguments | [
        site: RawTransactionArgument<string>,
        path: RawTransactionArgument<string>
    ];
}
/** Removes a resource from a site if it exists. */
export function removeResourceIfExists(options: RemoveResourceIfExistsOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "path"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'remove_resource_if_exists',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface MoveResourceArguments {
    site: RawTransactionArgument<string>;
    oldPath: RawTransactionArgument<string>;
    newPath: RawTransactionArgument<string>;
}
export interface MoveResourceOptions {
    package?: string;
    arguments: MoveResourceArguments | [
        site: RawTransactionArgument<string>,
        oldPath: RawTransactionArgument<string>,
        newPath: RawTransactionArgument<string>
    ];
}
/** Changes the path of a resource on a site. */
export function moveResource(options: MoveResourceOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String',
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "oldPath", "newPath"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'move_resource',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface CreateRoutesArguments {
    site: RawTransactionArgument<string>;
}
export interface CreateRoutesOptions {
    package?: string;
    arguments: CreateRoutesArguments | [
        site: RawTransactionArgument<string>
    ];
}
/** Add the routes dynamic field to the site. */
export function createRoutes(options: CreateRoutesOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'create_routes',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface RemoveAllRoutesIfExistArguments {
    site: RawTransactionArgument<string>;
}
export interface RemoveAllRoutesIfExistOptions {
    package?: string;
    arguments: RemoveAllRoutesIfExistArguments | [
        site: RawTransactionArgument<string>
    ];
}
/** Remove all routes from the site. */
export function removeAllRoutesIfExist(options: RemoveAllRoutesIfExistOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'remove_all_routes_if_exist',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface InsertRouteArguments {
    site: RawTransactionArgument<string>;
    route: RawTransactionArgument<string>;
    resourcePath: RawTransactionArgument<string>;
}
export interface InsertRouteOptions {
    package?: string;
    arguments: InsertRouteArguments | [
        site: RawTransactionArgument<string>,
        route: RawTransactionArgument<string>,
        resourcePath: RawTransactionArgument<string>
    ];
}
/**
 * Add a route to the site.
 *
 * The insertion operation fails:
 *
 * - if the route already exists; or
 * - if the related resource path does not already exist as a dynamic field on the
 *   site.
 */
export function insertRoute(options: InsertRouteOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String',
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "route", "resourcePath"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'insert_route',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface RemoveRouteArguments {
    site: RawTransactionArgument<string>;
    route: RawTransactionArgument<string>;
}
export interface RemoveRouteOptions {
    package?: string;
    arguments: RemoveRouteArguments | [
        site: RawTransactionArgument<string>,
        route: RawTransactionArgument<string>
    ];
}
/** Remove a route from the site. */
export function removeRoute(options: RemoveRouteOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::string::String'
    ] satisfies string[];
    const parameterNames = ["site", "route"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'remove_route',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface BurnArguments {
    site: RawTransactionArgument<string>;
}
export interface BurnOptions {
    package?: string;
    arguments: BurnArguments | [
        site: RawTransactionArgument<string>
    ];
}
/**
 * Deletes a site object.
 *
 * NB: This function does **NOT** delete the dynamic fields! Make sure to call this
 * function after deleting manually all the dynamic fields attached to the sites
 * object. If you don't delete the dynamic fields, they will become unaccessible
 * and you will not be able to delete them in the future.
 */
export function burn(options: BurnOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'burn',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteNameArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteNameOptions {
    package?: string;
    arguments: GetSiteNameArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteName(options: GetSiteNameOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_name',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteLinkArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteLinkOptions {
    package?: string;
    arguments: GetSiteLinkArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteLink(options: GetSiteLinkOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_link',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteImageUrlArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteImageUrlOptions {
    package?: string;
    arguments: GetSiteImageUrlArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteImageUrl(options: GetSiteImageUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_image_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteDescriptionArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteDescriptionOptions {
    package?: string;
    arguments: GetSiteDescriptionArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteDescription(options: GetSiteDescriptionOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_description',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteProjectUrlArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteProjectUrlOptions {
    package?: string;
    arguments: GetSiteProjectUrlArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteProjectUrl(options: GetSiteProjectUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_project_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface GetSiteCreatorArguments {
    site: RawTransactionArgument<string>;
}
export interface GetSiteCreatorOptions {
    package?: string;
    arguments: GetSiteCreatorArguments | [
        site: RawTransactionArgument<string>
    ];
}
export function getSiteCreator(options: GetSiteCreatorOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::site::Site`
    ] satisfies string[];
    const parameterNames = ["site"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'site',
        function: 'get_site_creator',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}

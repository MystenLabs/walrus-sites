// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**************************************************************
 * THIS FILE IS GENERATED AND SHOULD NOT BE MANUALLY MODIFIED *
 **************************************************************/
import { MoveStruct, normalizeMoveArguments, type RawTransactionArgument } from '../utils/index.js';
import { bcs } from '@mysten/sui/bcs';
import { type Transaction } from '@mysten/sui/transactions';
const $moduleName = '@walrus/sites::metadata';
export const Metadata = new MoveStruct({ name: `${$moduleName}::Metadata`, fields: {
        link: bcs.option(bcs.string()),
        image_url: bcs.option(bcs.string()),
        description: bcs.option(bcs.string()),
        project_url: bcs.option(bcs.string()),
        creator: bcs.option(bcs.string())
    } });
export interface NewMetadataArguments {
    link: RawTransactionArgument<string | null>;
    imageUrl: RawTransactionArgument<string | null>;
    description: RawTransactionArgument<string | null>;
    projectUrl: RawTransactionArgument<string | null>;
    creator: RawTransactionArgument<string | null>;
}
export interface NewMetadataOptions {
    package?: string;
    arguments: NewMetadataArguments | [
        link: RawTransactionArgument<string | null>,
        imageUrl: RawTransactionArgument<string | null>,
        description: RawTransactionArgument<string | null>,
        projectUrl: RawTransactionArgument<string | null>,
        creator: RawTransactionArgument<string | null>
    ];
}
/** Creates a new Metadata object. */
export function newMetadata(options: NewMetadataOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>',
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["link", "imageUrl", "description", "projectUrl", "creator"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'new_metadata',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface LinkArguments {
    metadata: RawTransactionArgument<string>;
}
export interface LinkOptions {
    package?: string;
    arguments: LinkArguments | [
        metadata: RawTransactionArgument<string>
    ];
}
export function link(options: LinkOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'link',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface ImageUrlArguments {
    metadata: RawTransactionArgument<string>;
}
export interface ImageUrlOptions {
    package?: string;
    arguments: ImageUrlArguments | [
        metadata: RawTransactionArgument<string>
    ];
}
export function imageUrl(options: ImageUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'image_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface DescriptionArguments {
    metadata: RawTransactionArgument<string>;
}
export interface DescriptionOptions {
    package?: string;
    arguments: DescriptionArguments | [
        metadata: RawTransactionArgument<string>
    ];
}
export function description(options: DescriptionOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'description',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface ProjectUrlArguments {
    metadata: RawTransactionArgument<string>;
}
export interface ProjectUrlOptions {
    package?: string;
    arguments: ProjectUrlArguments | [
        metadata: RawTransactionArgument<string>
    ];
}
export function projectUrl(options: ProjectUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'project_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface CreatorArguments {
    metadata: RawTransactionArgument<string>;
}
export interface CreatorOptions {
    package?: string;
    arguments: CreatorArguments | [
        metadata: RawTransactionArgument<string>
    ];
}
export function creator(options: CreatorOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`
    ] satisfies string[];
    const parameterNames = ["metadata"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'creator',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface SetLinkArguments {
    metadata: RawTransactionArgument<string>;
    link: RawTransactionArgument<string | null>;
}
export interface SetLinkOptions {
    package?: string;
    arguments: SetLinkArguments | [
        metadata: RawTransactionArgument<string>,
        link: RawTransactionArgument<string | null>
    ];
}
export function setLink(options: SetLinkOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["metadata", "link"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'set_link',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface SetImageUrlArguments {
    metadata: RawTransactionArgument<string>;
    imageUrl: RawTransactionArgument<string | null>;
}
export interface SetImageUrlOptions {
    package?: string;
    arguments: SetImageUrlArguments | [
        metadata: RawTransactionArgument<string>,
        imageUrl: RawTransactionArgument<string | null>
    ];
}
export function setImageUrl(options: SetImageUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["metadata", "imageUrl"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'set_image_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface SetDescriptionArguments {
    metadata: RawTransactionArgument<string>;
    description: RawTransactionArgument<string | null>;
}
export interface SetDescriptionOptions {
    package?: string;
    arguments: SetDescriptionArguments | [
        metadata: RawTransactionArgument<string>,
        description: RawTransactionArgument<string | null>
    ];
}
export function setDescription(options: SetDescriptionOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["metadata", "description"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'set_description',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface SetProjectUrlArguments {
    metadata: RawTransactionArgument<string>;
    projectUrl: RawTransactionArgument<string | null>;
}
export interface SetProjectUrlOptions {
    package?: string;
    arguments: SetProjectUrlArguments | [
        metadata: RawTransactionArgument<string>,
        projectUrl: RawTransactionArgument<string | null>
    ];
}
export function setProjectUrl(options: SetProjectUrlOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["metadata", "projectUrl"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'set_project_url',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}
export interface SetCreatorArguments {
    metadata: RawTransactionArgument<string>;
    creator: RawTransactionArgument<string | null>;
}
export interface SetCreatorOptions {
    package?: string;
    arguments: SetCreatorArguments | [
        metadata: RawTransactionArgument<string>,
        creator: RawTransactionArgument<string | null>
    ];
}
export function setCreator(options: SetCreatorOptions) {
    const packageAddress = options.package ?? '@walrus/sites';
    const argumentsTypes = [
        `${packageAddress}::metadata::Metadata`,
        '0x0000000000000000000000000000000000000000000000000000000000000001::option::Option<0x0000000000000000000000000000000000000000000000000000000000000001::string::String>'
    ] satisfies string[];
    const parameterNames = ["metadata", "creator"];
    return (tx: Transaction) => tx.moveCall({
        package: packageAddress,
        module: 'metadata',
        function: 'set_creator',
        arguments: normalizeMoveArguments(options.arguments, argumentsTypes, parameterNames),
    });
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Purpose: Contains the enumeration of HTTP status codes.
export enum HttpStatusCodes {
    OK = 200,
    CREATED = 201,
    NO_CONTENT = 204,
    MOVED_PERMANENTLY = 301,
    FOUND = 302,
    TOO_MANY_REDIRECTS = 310,
    BAD_REQUEST = 400,
    UNAUTHORIZED = 401,
    FORBIDDEN = 403,
    NOT_FOUND = 404,
    INTERNAL_SERVER_ERROR = 500,
    LOOP_DETECTED = 508
}

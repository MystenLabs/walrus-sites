// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Purpose: Contains the enumeration of HTTP status codes.
export enum HttpStatusCodes {
    TOO_MANY_REDIRECTS = 310,
    NOT_FOUND = 404,
    UNPROCESSABLE_CONTENT = 422,
    INTERNAL_SERVER_ERROR = 500,
    SERVICE_UNAVAILABLE = 503,
    LOOP_DETECTED = 508,
}

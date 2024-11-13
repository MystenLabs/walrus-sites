// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Provides a simple logger interface function for
/// logging messages on different runtimes.
type LoggingPredicate = (...args: any) => void;

/**
 * Logger used as an abstraction for logging messages on different runtimes.
 */
 class Logger {
    private infoPredicate: LoggingPredicate;
    private warnPredicate: LoggingPredicate;
    private errorPredicate: LoggingPredicate;

    /// Constructor. Initializes the predicates for logging.
    /// If no predicates are provided, the console object is used.
    /// @param infoPredicate Function for info messages.
    /// @param warnPredicate Function for warning messages.
    /// @param errorPredicate Function for error messages.
    constructor(
        logPredicate: LoggingPredicate = console.log,
        warnPredicate: LoggingPredicate = console.warn,
        errorPredicate: LoggingPredicate = console.error
    ) {
        this.infoPredicate = logPredicate;
        this.warnPredicate = warnPredicate;
        this.errorPredicate = errorPredicate;
    }

    info(...args: any): void {
        this.infoPredicate(...args);
    }

    warn(...args: any): void {
        this.warnPredicate(...args);
    }

    error(...args: any): void {
        this.errorPredicate(...args);
    }

    setInfoPredicate(predicate: LoggingPredicate): void {
        this.infoPredicate = predicate;
    }

    setWarnPredicate(predicate: LoggingPredicate): void {
        this.warnPredicate = predicate;
    }

    setErrorPredicate(predicate: LoggingPredicate): void {
        this.errorPredicate = predicate;
    }
}

const logger = new Logger();
export default logger;

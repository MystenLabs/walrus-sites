// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Provides a simple logger interface function for
/// logging messages on different runtimes.
type LoggingPredicate = (...args: any[]) => void;

/**
 * Logger used as an abstraction for logging messages on different runtimes.
 * The logger prints messages based on the predicate provided for each level.
 * This allows for easy switching between different logging mechanisms,
 * integrating it with a log exporter, or disabling logging altogether.
 */
 class Logger {
    private debugPredicate: LoggingPredicate;
    private infoPredicate: LoggingPredicate;
    private warnPredicate: LoggingPredicate;
    private errorPredicate: LoggingPredicate;

    /// Constructor. Initializes the predicates for logging.
    /// If no predicates are provided, the console object is used.
    /// @param debugPredicate Function for debug messages.
    /// @param infoPredicate Function for info messages.
    /// @param warnPredicate Function for warning messages.
    /// @param errorPredicate Function for error messages.
    constructor(
		debugPredicate: LoggingPredicate = (...args) => console.debug(...args),
        logPredicate: LoggingPredicate = (...args) => console.log(...args),
        warnPredicate: LoggingPredicate = (...args) => console.warn(...args),
        errorPredicate: LoggingPredicate = (...args) => console.error(...args)
    ) {
        this.debugPredicate = debugPredicate;
        this.infoPredicate = logPredicate;
        this.warnPredicate = warnPredicate;
        this.errorPredicate = errorPredicate;
    }

    /// The highest level of logging, used for debugging purposes.
    debug(...args: any[]): void {
        this.debugPredicate(...args);
    }

    /// General logging level, used for informational messages.
    info(...args: any[]): void {
        this.infoPredicate(...args);
    }

    /// Logging level for warnings, used for non-critical issues, or states
    /// that need to be considered.
    warn(...args: any[]): void {
        this.warnPredicate(...args);
    }

    /// Logging level for errors, used for critical issues.
    error(...args: any[]): void {
        this.errorPredicate(...args);
    }

    setDebugPredicate(predicate: LoggingPredicate): void {
        this.debugPredicate = predicate;
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

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Provides a simple logger interface function for
/// logging messages on different runtimes.
type LoggingPredicate  = (message: string, ...args: any[]) => void

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
    // Context is useful for structured logging. Use it to group logs by a specific property
    // e.g. user ID or request ID.
    context?: String;

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
    debug(message: string, ...args: any[]): void {
        this.debugPredicate(message, ...args);
    }

    /// General logging level, used for informational messages.
    info(message: string, ...args: any[]): void {
        this.infoPredicate(message, ...args);
    }

    /// Logging level for warnings, used for non-critical issues, or states
    /// that need to be considered.
    warn(message: string, ...args: any[]): void {
        this.warnPredicate(message, ...args);
    }

    /// Logging level for errors, used for critical issues.
    error(message: string, ...args: any[]): void {
        this.errorPredicate(message, ...args);
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

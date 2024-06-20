// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Display,
    io::{stderr, stdout},
};

use crossterm::{
    cursor::{RestorePosition, SavePosition},
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};

pub fn header<S: Display>(message: S) {
    if cfg!(not(test)) {
        crossterm::execute!(
            stdout(),
            PrintStyledContent(format!("\n{message}\n").green().bold()),
        )
        .unwrap();
    }
}

pub fn error<S: Display>(message: S) {
    if cfg!(not(test)) {
        crossterm::execute!(
            stderr(),
            PrintStyledContent(format!("\n{message}\n").red().bold()),
        )
        .unwrap();
    }
}

pub fn action<S: Display>(message: S) {
    if cfg!(not(test)) {
        crossterm::execute!(stdout(), Print(format!("{message} ... ")), SavePosition).unwrap();
    }
}

pub fn done() {
    if cfg!(not(test)) {
        crossterm::execute!(
            stdout(),
            RestorePosition,
            Clear(ClearType::UntilNewLine),
            Print(format!("[{}]\n", "Ok".green()))
        )
        .unwrap();
    }
}

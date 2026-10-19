// Copyright (c) 2026 Joseph Hale
// SPDX-License-Identifier: MPL-2.0

use std::io::Write;

pub fn say(line: &str) {
	match writeln!(std::io::stdout(), "{line}") {
		Ok(()) => (),
		Err(_) => std::process::exit(0),
	}
}

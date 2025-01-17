// lib.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

#![recursion_limit = "1024"]
#![cfg_attr(
    all(test, feature = "clippy"),
    allow(option_unwrap_used, result_unwrap_used)
)]
#![cfg_attr(
    feature = "clippy",
    warn(
        option_unwrap_used,
        result_unwrap_used,
        print_stdout,
        wrong_pub_self_convention,
        mut_mut,
        non_ascii_literal,
        similar_names,
        unicode_not_nfc,
        enum_glob_use,
        if_not_else,
        items_after_statements,
        used_underscore_binding
    )
)]
// Enable lint group collections
#![warn(nonstandard_style, bad_style, unused)]
#![warn(rust_2018_idioms)]
// standalone lints
#![warn(
    const_err,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    unconditional_recursion,
    while_true,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    missing_copy_implementations
)]

//! FIXME: Docs

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
#[macro_use]
extern crate maplit;

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate log;

pub mod database;
#[allow(missing_docs)]
pub mod dbqueries;
#[allow(missing_docs)]
pub mod downloader;
#[allow(missing_docs)]
pub mod errors;
mod feed;
pub(crate) mod models;
pub mod opml;
mod parser;
pub mod pipeline;
mod schema;
pub mod utils;

pub use crate::feed::{Feed, FeedBuilder};
pub use crate::models::Save;
pub use crate::models::{Episode, EpisodeWidgetModel, Show, ShowCoverModel, Source};

// Set the user agent, See #53 for more
// Keep this in sync with Tor-browser releases
/// The user-agent to be used for all the requests.
/// It originates from the Tor-browser UA.
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; rv:78.0) Gecko/20100101 Firefox/78.0";

/// [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) Paths.
#[allow(missing_debug_implementations)]
pub mod xdg_dirs {
    use once_cell::sync::Lazy;
    use std::path::PathBuf;

    pub(crate) static PODCASTS_XDG: Lazy<xdg::BaseDirectories> =
        Lazy::new(|| xdg::BaseDirectories::with_prefix("gnome-podcasts").unwrap());

    /// XDG_DATA Directory `Pathbuf`.
    pub static PODCASTS_DATA: Lazy<PathBuf> = Lazy::new(|| {
        PODCASTS_XDG
            .create_data_directory(PODCASTS_XDG.get_data_home())
            .unwrap()
    });

    /// XDG_CONFIG Directory `Pathbuf`.
    pub static PODCASTS_CONFIG: Lazy<PathBuf> = Lazy::new(|| {
        PODCASTS_XDG
            .create_config_directory(PODCASTS_XDG.get_config_home())
            .unwrap()
    });

    /// XDG_CACHE Directory `Pathbuf`.
    pub static PODCASTS_CACHE: Lazy<PathBuf> = Lazy::new(|| {
        PODCASTS_XDG
            .create_cache_directory(PODCASTS_XDG.get_cache_home())
            .unwrap()
    });

    /// GNOME Podcasts Download Directory `PathBuf`.
    pub static DL_DIR: Lazy<PathBuf> =
        Lazy::new(|| PODCASTS_XDG.create_data_directory("Downloads").unwrap());
}

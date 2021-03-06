// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use {CrateLint, PathResult};

use std::collections::BTreeSet;

use syntax::ast::Ident;
use syntax::symbol::{keywords, Symbol};
use syntax_pos::Span;

use resolve_imports::ImportResolver;

impl<'a, 'b:'a, 'c: 'b> ImportResolver<'a, 'b, 'c> {
    /// Add suggestions for a path that cannot be resolved.
    pub(crate) fn make_path_suggestion(
        &mut self,
        span: Span,
        path: Vec<Ident>
    ) -> Option<Vec<Ident>> {
        debug!("make_path_suggestion: span={:?} path={:?}", span, path);
        // If we don't have a path to suggest changes to, then return.
        if path.is_empty() {
            return None;
        }

        // Check whether a ident is a path segment that is not root.
        let is_special = |ident: Ident| ident.is_path_segment_keyword() &&
                                        ident.name != keywords::CrateRoot.name();

        match (path.get(0), path.get(1)) {
            // Make suggestions that require at least two non-special path segments.
            (Some(fst), Some(snd)) if !is_special(*fst) && !is_special(*snd) => {
                debug!("make_path_suggestion: fst={:?} snd={:?}", fst, snd);

                self.make_missing_self_suggestion(span, path.clone())
                    .or_else(|| self.make_missing_crate_suggestion(span, path.clone()))
                    .or_else(|| self.make_missing_super_suggestion(span, path.clone()))
                    .or_else(|| self.make_external_crate_suggestion(span, path.clone()))
            },
            _ => None,
        }
    }

    /// Suggest a missing `self::` if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foo::Bar;
    ///    |     ^^^ Did you mean `self::foo`?
    /// ```
    fn make_missing_self_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Ident>
    ) -> Option<Vec<Ident>> {
        // Replace first ident with `self` and check if that is valid.
        path[0].name = keywords::SelfValue.name();
        let result = self.resolve_path(None, &path, None, false, span, CrateLint::No);
        debug!("make_missing_self_suggestion: path={:?} result={:?}", path, result);
        if let PathResult::Module(..) = result {
            Some(path)
        } else {
            None
        }
    }

    /// Suggest a missing `crate::` if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foo::Bar;
    ///    |     ^^^ Did you mean `crate::foo`?
    /// ```
    fn make_missing_crate_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Ident>
    ) -> Option<Vec<Ident>> {
        // Replace first ident with `crate` and check if that is valid.
        path[0].name = keywords::Crate.name();
        let result = self.resolve_path(None, &path, None, false, span, CrateLint::No);
        debug!("make_missing_crate_suggestion:  path={:?} result={:?}", path, result);
        if let PathResult::Module(..) = result {
            Some(path)
        } else {
            None
        }
    }

    /// Suggest a missing `super::` if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foo::Bar;
    ///    |     ^^^ Did you mean `super::foo`?
    /// ```
    fn make_missing_super_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Ident>
    ) -> Option<Vec<Ident>> {
        // Replace first ident with `crate` and check if that is valid.
        path[0].name = keywords::Super.name();
        let result = self.resolve_path(None, &path, None, false, span, CrateLint::No);
        debug!("make_missing_super_suggestion:  path={:?} result={:?}", path, result);
        if let PathResult::Module(..) = result {
            Some(path)
        } else {
            None
        }
    }

    /// Suggest a missing external crate name if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foobar::Baz;
    ///    |     ^^^^^^ Did you mean `baz::foobar`?
    /// ```
    ///
    /// Used when importing a submodule of an external crate but missing that crate's
    /// name as the first part of path.
    fn make_external_crate_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Ident>
    ) -> Option<Vec<Ident>> {
        // Need to clone else we can't call `resolve_path` without a borrow error. We also store
        // into a `BTreeMap` so we can get consistent ordering (and therefore the same diagnostic)
        // each time.
        let external_crate_names: BTreeSet<Symbol> = self.resolver.session.extern_prelude
            .clone().drain().collect();

        // Insert a new path segment that we can replace.
        let new_path_segment = path[0].clone();
        path.insert(1, new_path_segment);

        // Iterate in reverse so that we start with crates at the end of the alphabet. This means
        // that we'll always get `std` before `core`.
        for name in external_crate_names.iter().rev() {
            let ident = Ident::with_empty_ctxt(*name);
            // Calling `maybe_process_path_extern` ensures that we're only running `resolve_path`
            // on a crate name that won't ICE.
            if let Some(_) = self.crate_loader.maybe_process_path_extern(*name, ident.span) {
                // Replace the first after root (a placeholder we inserted) with a crate name
                // and check if that is valid.
                path[1].name = *name;
                let result = self.resolve_path(None, &path, None, false, span, CrateLint::No);
                debug!("make_external_crate_suggestion: name={:?} path={:?} result={:?}",
                       name, path, result);
                if let PathResult::Module(..) = result {
                    return Some(path)
                }
            }
        }

        // Remove our placeholder segment.
        path.remove(1);
        None
    }
}
